//! External command resolution and execution.
//!
//! Phase 6 launches programs directly with [`std::process::Command`]. The shell
//! never hands a command line to `sh -c`, `cmd /C`, or any other shell: the
//! lexer and parser decide where each argument starts and ends, and this module
//! passes those arguments to the operating system as separate values.
//!
//! Command names containing a path separator (or an absolute path) are treated
//! as paths and resolved against the session working directory. Bare names are
//! looked up in the `PATH` supplied by the process options, falling back to the
//! shell's own environment.

use std::env;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};

/// How a child process is connected to the shell's standard streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StdioMode {
    /// Share the shell's terminal. Output streams directly to the user, which
    /// keeps long-running programs responsive and preserves TTY behaviour.
    #[default]
    Inherit,
    /// Pipe standard output and standard error back to the shell.
    ///
    /// Intended for tests and for the pipeline engine in a later phase. The
    /// child's standard input is closed so a captured command cannot block
    /// waiting for terminal input.
    Capture,
}

/// Options applied to every external command dispatched by the shell.
///
/// Environment entries are overrides layered on top of the inherited
/// environment; they never modify the shell process's own environment.
#[derive(Debug, Clone, Default)]
pub struct ProcessOptions {
    stdio: StdioMode,
    environment: Vec<(OsString, OsString)>,
}

impl ProcessOptions {
    /// Creates options that inherit the terminal and environment.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects the standard-stream strategy.
    #[must_use]
    pub fn with_stdio(mut self, stdio: StdioMode) -> Self {
        self.stdio = stdio;
        self
    }

    /// Adds an environment override for child processes.
    ///
    /// A `PATH` override is also used to resolve bare executable names.
    #[must_use]
    pub fn with_env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.environment.push((key.into(), value.into()));
        self
    }

    /// Standard-stream strategy for child processes.
    #[must_use]
    pub const fn stdio(&self) -> StdioMode {
        self.stdio
    }

    /// Environment overrides for child processes, in insertion order.
    #[must_use]
    pub fn environment(&self) -> &[(OsString, OsString)] {
        &self.environment
    }
}

/// Why a program is not executable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutableState {
    /// The candidate is a file that the operating system can execute.
    Executable,
    /// The candidate exists but cannot be executed.
    NotExecutable,
    /// No file exists at the candidate path.
    Missing,
}

/// A completed or terminated child process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessStatus {
    success: bool,
    code: Option<i32>,
    signal: Option<i32>,
}

impl ProcessStatus {
    /// Returns `true` when the process exited with code zero.
    #[must_use]
    pub const fn success(self) -> bool {
        self.success
    }

    /// Exit code, when the process exited normally.
    ///
    /// This is `None` when the process was terminated by a signal.
    #[must_use]
    pub const fn code(self) -> Option<i32> {
        self.code
    }

    /// Signal that terminated the process, when one did.
    ///
    /// Always `None` on non-Unix platforms.
    #[must_use]
    pub const fn signal(self) -> Option<i32> {
        self.signal
    }

    pub(crate) fn from_exit_status(status: ExitStatus) -> Self {
        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;

            status.signal()
        };
        #[cfg(not(unix))]
        let signal = None;

        Self {
            success: status.success(),
            code: status.code(),
            signal,
        }
    }
}

/// Output and status produced by an external command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessReport {
    status: ProcessStatus,
    stdout: String,
    stderr: String,
}

impl ProcessReport {
    /// Termination status of the child.
    #[must_use]
    pub const fn status(&self) -> ProcessStatus {
        self.status
    }

    /// Captured standard output, or an empty string when inheriting.
    #[must_use]
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    /// Captured standard error, or an empty string when inheriting.
    #[must_use]
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    /// Exit code, when the child exited normally.
    #[must_use]
    pub const fn exit_code(&self) -> Option<i32> {
        self.status.code()
    }

    /// Returns `true` when the child exited successfully.
    #[must_use]
    pub const fn success(&self) -> bool {
        self.status.success()
    }
}

/// Failure to resolve or launch an external program.
#[derive(Debug)]
pub enum ProcessError {
    /// No executable was found for the requested name.
    NotFound {
        /// Program name as typed by the user.
        program: String,
    },
    /// A file was found but cannot be executed.
    PermissionDenied {
        /// Program name as typed by the user.
        program: String,
    },
    /// The program name cannot be turned into a valid operating-system path.
    InvalidPath {
        /// Program name as typed by the user.
        program: String,
        /// Why the name is invalid.
        reason: String,
    },
    /// The operating system refused to start the process.
    Spawn {
        /// Program name as typed by the user.
        program: String,
        /// Underlying operating-system error.
        source: io::Error,
    },
    /// The shell could not wait for the process.
    Wait {
        /// Program name as typed by the user.
        program: String,
        /// Underlying operating-system error.
        source: io::Error,
    },
}

impl ProcessError {
    /// Program name associated with this error.
    #[must_use]
    pub fn program(&self) -> &str {
        match self {
            Self::NotFound { program }
            | Self::PermissionDenied { program }
            | Self::InvalidPath { program, .. }
            | Self::Spawn { program, .. }
            | Self::Wait { program, .. } => program,
        }
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { program } => write!(formatter, "command not found: {program}"),
            Self::PermissionDenied { program } => {
                write!(formatter, "permission denied: {program}")
            }
            Self::InvalidPath { program, reason } => {
                write!(formatter, "invalid executable path '{program}': {reason}")
            }
            Self::Spawn { program, source } => {
                write!(formatter, "failed to start '{program}': {source}")
            }
            Self::Wait { program, source } => {
                write!(formatter, "failed while waiting for '{program}': {source}")
            }
        }
    }
}

impl Error for ProcessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Spawn { source, .. } | Self::Wait { source, .. } => Some(source),
            Self::NotFound { .. } | Self::PermissionDenied { .. } | Self::InvalidPath { .. } => {
                None
            }
        }
    }
}

/// A program invocation built from an already-parsed shell command.
///
/// Arguments are kept as a list of strings so the shell can preserve argument
/// boundaries without ever joining them into a command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCommand {
    program: String,
    arguments: Vec<String>,
    working_directory: Option<PathBuf>,
    environment: Vec<(OsString, OsString)>,
    clear_environment: bool,
    stdio: StdioMode,
}

impl ExternalCommand {
    /// Creates a command for `program` with no arguments.
    #[must_use]
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            arguments: Vec::new(),
            working_directory: None,
            environment: Vec::new(),
            clear_environment: false,
            stdio: StdioMode::Inherit,
        }
    }

    /// Appends one argument, preserving it as a single OS argument.
    #[must_use]
    pub fn arg(mut self, argument: impl Into<String>) -> Self {
        self.arguments.push(argument.into());
        self
    }

    /// Appends several arguments in order.
    #[must_use]
    pub fn args<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.arguments.extend(arguments.into_iter().map(Into::into));
        self
    }

    /// Sets the directory the child process runs in.
    #[must_use]
    pub fn current_dir(mut self, directory: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(directory.into());
        self
    }

    /// Adds one environment override for the child.
    #[must_use]
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.environment.push((key.into(), value.into()));
        self
    }

    /// Adds several environment overrides for the child.
    #[must_use]
    pub fn envs<I, K, V>(mut self, variables: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.environment.extend(
            variables
                .into_iter()
                .map(|(key, value)| (key.into(), value.into())),
        );
        self
    }

    /// Replaces the inherited environment with exactly the supplied variables.
    ///
    /// The shell uses this so child processes see the session's exported
    /// variables (including `unset` hiding) rather than the host environment
    /// that happened to be present when the shell started.
    #[must_use]
    pub const fn clear_env(mut self) -> Self {
        self.clear_environment = true;
        self
    }

    /// Selects the standard-stream strategy.
    #[must_use]
    pub const fn stdio(mut self, stdio: StdioMode) -> Self {
        self.stdio = stdio;
        self
    }

    /// Program name as typed by the user.
    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Arguments in order.
    #[must_use]
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    /// Resolves the program to an absolute executable path without running it.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError`] when the program cannot be found, is not
    /// executable, or cannot be turned into a path.
    pub fn resolve(&self) -> Result<PathBuf, ProcessError> {
        let working_directory = match &self.working_directory {
            Some(directory) => directory.clone(),
            None => env::current_dir().map_err(|source| ProcessError::Spawn {
                program: self.program.clone(),
                source,
            })?,
        };
        let path = environment_value(&self.environment, "PATH").or_else(|| {
            if self.clear_environment {
                None
            } else {
                env::var_os("PATH")
            }
        });

        resolve_with_path(&self.program, &working_directory, path.as_deref())
    }

    /// Resolves and runs the program to completion.
    ///
    /// In [`StdioMode::Inherit`] the child shares the shell's terminal and no
    /// output is stored in memory. In [`StdioMode::Capture`] standard output
    /// and standard error are captured as lossy UTF-8.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError`] when resolution, starting, or waiting fails.
    pub fn run(self) -> Result<ProcessReport, ProcessError> {
        match self.stdio {
            StdioMode::Inherit => {
                let mut child =
                    self.spawn_with(Stdio::inherit(), Stdio::inherit(), Stdio::inherit())?;
                let status = child.wait().map_err(|source| ProcessError::Wait {
                    program: self.program.clone(),
                    source,
                })?;

                Ok(ProcessReport {
                    status: ProcessStatus::from_exit_status(status),
                    stdout: String::new(),
                    stderr: String::new(),
                })
            }
            StdioMode::Capture => {
                let child = self.spawn_with(Stdio::null(), Stdio::piped(), Stdio::piped())?;
                let output = child
                    .wait_with_output()
                    .map_err(|source| ProcessError::Wait {
                        program: self.program.clone(),
                        source,
                    })?;

                Ok(ProcessReport {
                    status: ProcessStatus::from_exit_status(output.status),
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                })
            }
        }
    }

    /// Resolves the program and starts it with explicit standard streams.
    ///
    /// The pipeline engine uses this to connect a stage's stdout to the next
    /// stage's stdin without buffering. Callers own the returned [`Child`] and
    /// must wait for it.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError`] when resolution or spawning fails.
    pub fn spawn_with(
        &self,
        stdin: Stdio,
        stdout: Stdio,
        stderr: Stdio,
    ) -> Result<Child, ProcessError> {
        let executable = self.resolve()?;
        let mut command = self.build_command(&executable);

        command.stdin(stdin).stdout(stdout).stderr(stderr);

        command.spawn().map_err(|source| match source.kind() {
            io::ErrorKind::PermissionDenied => ProcessError::PermissionDenied {
                program: self.program.clone(),
            },
            _ => ProcessError::Spawn {
                program: self.program.clone(),
                source,
            },
        })
    }

    fn build_command(&self, executable: &Path) -> Command {
        let mut command = Command::new(executable);
        command.args(&self.arguments);

        if let Some(directory) = &self.working_directory {
            command.current_dir(directory);
        }

        if self.clear_environment {
            command.env_clear();
        }

        if !self.environment.is_empty() {
            command.envs(
                self.environment
                    .iter()
                    .map(|(key, value)| (key.as_os_str(), value.as_os_str())),
            );
        }

        command
    }
}

/// Resolves `program` using the shell process's `PATH`.
///
/// # Errors
///
/// Returns [`ProcessError`] when no executable can be found.
pub fn resolve_executable(
    program: &str,
    working_directory: &Path,
) -> Result<PathBuf, ProcessError> {
    let path = env::var_os("PATH");

    resolve_with_path(program, working_directory, path.as_deref())
}

fn resolve_with_path(
    program: &str,
    working_directory: &Path,
    path: Option<&OsStr>,
) -> Result<PathBuf, ProcessError> {
    if program.is_empty() {
        return Err(ProcessError::NotFound {
            program: program.to_owned(),
        });
    }

    if program.contains('\0') {
        return Err(ProcessError::InvalidPath {
            program: program.to_owned(),
            reason: "it contains a NUL byte".to_owned(),
        });
    }

    let program_path = Path::new(program);
    let is_explicit_path = program_path.is_absolute()
        || program.contains('/')
        || (cfg!(windows) && program.contains('\\'))
        || program_path.components().count() > 1;

    if is_explicit_path {
        let candidate = if program_path.is_absolute() {
            program_path.to_path_buf()
        } else {
            working_directory.join(program_path)
        };

        return resolve_candidates(append_platform_extensions(&candidate), program);
    }

    let Some(path) = path else {
        return Err(ProcessError::NotFound {
            program: program.to_owned(),
        });
    };

    let mut found_but_not_executable = false;

    for directory in env::split_paths(path) {
        let directory = if directory.as_os_str().is_empty() {
            working_directory.to_path_buf()
        } else if directory.is_absolute() {
            directory
        } else {
            working_directory.join(directory)
        };

        let candidates = append_platform_extensions(&directory.join(program));

        match resolve_candidates(candidates, program) {
            Ok(resolved) => return Ok(resolved),
            Err(ProcessError::PermissionDenied { .. }) => found_but_not_executable = true,
            Err(_) => {}
        }
    }

    if found_but_not_executable {
        Err(ProcessError::PermissionDenied {
            program: program.to_owned(),
        })
    } else {
        Err(ProcessError::NotFound {
            program: program.to_owned(),
        })
    }
}

fn resolve_candidates(candidates: Vec<PathBuf>, program: &str) -> Result<PathBuf, ProcessError> {
    let mut found_but_not_executable = false;

    for candidate in candidates {
        match executable_state(&candidate) {
            ExecutableState::Executable => return Ok(candidate),
            ExecutableState::NotExecutable => found_but_not_executable = true,
            ExecutableState::Missing => {}
        }
    }

    if found_but_not_executable {
        Err(ProcessError::PermissionDenied {
            program: program.to_owned(),
        })
    } else {
        Err(ProcessError::NotFound {
            program: program.to_owned(),
        })
    }
}

fn executable_state(path: &Path) -> ExecutableState {
    match fs::metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return ExecutableState::NotExecutable;
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                if metadata.permissions().mode() & 0o111 == 0 {
                    return ExecutableState::NotExecutable;
                }
            }

            ExecutableState::Executable
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => ExecutableState::Missing,
        Err(_) => ExecutableState::NotExecutable,
    }
}

fn environment_value(environment: &[(OsString, OsString)], name: &str) -> Option<OsString> {
    environment
        .iter()
        .rev()
        .find(|(key, _)| environment_key_matches(key, name))
        .map(|(_, value)| value.clone())
}

fn environment_key_matches(key: &OsStr, name: &str) -> bool {
    let key = key.to_string_lossy();

    if cfg!(windows) {
        key.eq_ignore_ascii_case(name)
    } else {
        key == name
    }
}

#[cfg(windows)]
fn append_platform_extensions(base: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![base.to_path_buf()];

    if base.extension().is_none() {
        for extension in windows_executable_extensions() {
            let mut candidate = base.as_os_str().to_os_string();
            candidate.push(&extension);
            candidates.push(PathBuf::from(candidate));
        }
    }

    candidates
}

#[cfg(not(windows))]
fn append_platform_extensions(base: &Path) -> Vec<PathBuf> {
    vec![base.to_path_buf()]
}

#[cfg(windows)]
fn windows_executable_extensions() -> Vec<String> {
    let raw = env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(".COM;.EXE"));

    raw.to_string_lossy()
        .split(';')
        .filter(|extension| !extension.is_empty())
        .map(|extension| {
            if extension.starts_with('.') {
                extension.to_ascii_lowercase()
            } else {
                format!(".{}", extension.to_ascii_lowercase())
            }
        })
        // Batch and command scripts are interpreted by another shell. Phase 6
        // launches native executables only, so they are deliberately skipped.
        .filter(|extension| extension != ".bat" && extension != ".cmd")
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TempDirectory {
        path: PathBuf,
    }

    impl TempDirectory {
        fn new(label: &str) -> Self {
            let unique = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "kernext-process-{label}-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("temp directory should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn current_executable() -> PathBuf {
        env::current_exe().expect("test executable should be known")
    }

    #[test]
    fn absolute_executable_paths_resolve_directly() {
        let directory = TempDirectory::new("absolute");
        let executable = current_executable();

        let resolved = resolve_with_path(
            executable.to_str().expect("path should be UTF-8"),
            directory.path(),
            None,
        )
        .expect("absolute path should resolve");

        assert_eq!(resolved, executable);
    }

    #[test]
    fn relative_executable_paths_resolve_against_the_working_directory() {
        let executable = current_executable();
        let directory = executable.parent().expect("executable has a parent");
        let name = executable.file_name().expect("executable has a name");
        let relative = format!(".{}{}", std::path::MAIN_SEPARATOR, name.to_string_lossy());

        let resolved =
            resolve_with_path(&relative, directory, None).expect("relative path should resolve");

        assert_eq!(resolved, executable);
    }

    #[test]
    fn bare_names_resolve_through_the_supplied_path() {
        let directory = TempDirectory::new("path");
        let executable = current_executable();
        let name = format!("resolved{}", executable_suffix());
        let target = directory.path().join(&name);
        fs::copy(&executable, &target).expect("helper should be copied");

        let path = OsString::from(directory.path());
        let resolved = resolve_with_path(&name, directory.path(), Some(&path))
            .expect("name should resolve through PATH");

        assert_eq!(resolved, target);
    }

    #[test]
    fn missing_bare_names_report_not_found() {
        let directory = TempDirectory::new("missing");
        let path = OsString::from(directory.path());

        let error = resolve_with_path(
            "definitely-not-a-real-command",
            directory.path(),
            Some(&path),
        )
        .expect_err("missing command should fail");

        assert!(matches!(error, ProcessError::NotFound { .. }));
    }

    #[test]
    fn empty_names_report_not_found() {
        let directory = TempDirectory::new("empty");

        let error =
            resolve_with_path("", directory.path(), None).expect_err("empty command should fail");

        assert!(matches!(error, ProcessError::NotFound { .. }));
        assert_eq!(error.program(), "");
    }

    #[test]
    fn nul_bytes_are_rejected_as_invalid_paths() {
        let directory = TempDirectory::new("nul");

        let error = resolve_with_path("bad\0name", directory.path(), None)
            .expect_err("NUL should be rejected");

        assert!(matches!(error, ProcessError::InvalidPath { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn files_without_execute_permission_report_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TempDirectory::new("permissions");
        let target = directory.path().join("not-executable");
        fs::write(&target, b"#!/bin/sh\n").expect("file should be written");
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644))
            .expect("permissions should be set");

        let error = resolve_with_path(
            target.to_str().expect("path should be UTF-8"),
            directory.path(),
            None,
        )
        .expect_err("non-executable file should fail");

        assert!(matches!(error, ProcessError::PermissionDenied { .. }));
    }

    #[test]
    fn arguments_are_kept_as_separate_values() {
        let command = ExternalCommand::new("program")
            .arg("one")
            .arg("two words")
            .args(["three", "'quoted'"]);

        assert_eq!(
            command.arguments(),
            ["one", "two words", "three", "'quoted'"]
        );
    }

    #[test]
    fn errors_expose_the_program_name() {
        let error = ProcessError::NotFound {
            program: "missing".to_owned(),
        };

        assert_eq!(error.program(), "missing");
        assert_eq!(error.to_string(), "command not found: missing");
    }

    #[cfg(windows)]
    fn executable_suffix() -> &'static str {
        ".exe"
    }

    #[cfg(not(windows))]
    fn executable_suffix() -> &'static str {
        ""
    }
}
