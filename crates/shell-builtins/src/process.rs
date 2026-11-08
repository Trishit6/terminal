//! External command resolution and execution.
//!
//! Programs are launched directly with [`std::process::Command`]. The shell never
//! hands a command line to `sh -c` or another shell for parsing: the lexer and
//! parser decide where each argument starts and ends, and this module passes
//! those arguments to the operating system as separate values.
//!
//! Command names containing a path separator (or an absolute path) are treated
//! as paths and resolved against the session working directory. Bare names are
//! looked up in the `PATH` supplied by the process options, falling back to the
//! shell's own environment.
//!
//! # Windows launchers
//!
//! Windows can only start native images directly, so resolution honours
//! `PATHEXT` and classifies the result:
//!
//! - `.exe` and `.com` files are native images and launch directly.
//! - `.cmd` and `.bat` files are command scripts. They are run through the
//!   system command interpreter (`%COMSPEC%`, then `%SystemRoot%\System32\cmd.exe`)
//!   with `/d /v:off /s /c`, and every argument is escaped before it is placed
//!   on the interpreter's command line.
//!
//! Extensionless files are never treated as executables on Windows. npm and pnpm
//! install a POSIX `sh` shim (no extension) next to a `.cmd` shim; handing the
//! extensionless shim to `CreateProcess` is exactly what produced
//! `os error 193` ("not a valid Win32 application"). The `.cmd` shim is the
//! correct Windows entry point and is now what resolution selects.
//!
//! # Interrupts
//!
//! While a foreground child owns the terminal, [`InterruptGuard`] suppresses the
//! shell's own Ctrl+C handling so the signal reaches the child instead of
//! terminating KERNEXT. Children are spawned with the default Ctrl+C disposition
//! restored, even when the guard is active.
//!

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
    /// Run the child on a pseudoterminal (ConPTY on Windows, `openpty` on
    /// Unix) and relay it to the shell's own streams.
    ///
    /// This is a genuine interactive terminal: the child sees a TTY, receives
    /// keystrokes as they are typed, and is told the terminal size. It is used
    /// only for a single external command with no pipes or redirections;
    /// pipeline stages always inherit or pipe their streams.
    Pty,
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

/// How a resolved program must be started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutableKind {
    /// A native executable image launched directly.
    Native,
    /// A Windows command script run through the system command interpreter.
    #[cfg(windows)]
    BatchScript,
}

/// A resolved program: the file to start and how to start it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedProgram {
    path: PathBuf,
    kind: ExecutableKind,
}

/// How the operating system must be asked to start a resolved program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchSpec {
    /// Executable image the operating system starts.
    pub executable: PathBuf,
    /// Arguments passed before the raw argument text, if any.
    pub arguments: Vec<String>,
    /// Pre-escaped command line appended verbatim (Windows script launches).
    pub raw_arguments: Option<String>,
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

    /// Creates a status from a raw platform exit code.
    ///
    /// Used by the pseudoterminal engine, which receives a `DWORD` from
    /// `GetExitCodeProcess` rather than an [`ExitStatus`].
    #[cfg(windows)]
    pub(crate) fn from_code(code: i32) -> Self {
        Self {
            success: code == 0,
            code: normalize_exit_code(Some(code)),
            signal: None,
        }
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
            code: normalize_exit_code(status.code()),
            signal,
        }
    }
}

/// Maps platform-specific termination codes onto shell exit codes.
///
/// Windows reports a Ctrl+C termination as the NTSTATUS `0xC000013A` (which
/// [`ExitStatus::code`] surfaces as a negative `i32`). Shells use `130` for an
/// interrupted foreground command, so KERNEXT reports that instead.
fn normalize_exit_code(code: Option<i32>) -> Option<i32> {
    #[cfg(windows)]
    const STATUS_CONTROL_C_EXIT: u32 = 0xC000_013A;

    #[cfg(windows)]
    if code.is_some_and(|code| code as u32 == STATUS_CONTROL_C_EXIT) {
        return Some(130);
    }

    code
}

/// Suppresses the shell's own interrupt handling while a foreground child runs.
///
/// Without this, Ctrl+C terminates the shell as well as the child, because both
/// are attached to the same terminal. The guard only affects the shell process:
/// the child keeps its own handler (Unix children get `SIGINT` reset to the
/// default disposition at spawn time).
///
/// Dropping the guard restores the previous behaviour. Creating two guards at
/// once is not supported, which matches the call sites: exactly one foreground
/// command (or pipeline) is waited for at a time.
pub(crate) struct InterruptGuard {
    #[cfg(windows)]
    installed: bool,
    #[cfg(unix)]
    previous: Option<libc::sighandler_t>,
}

impl InterruptGuard {
    /// Stops Ctrl+C from terminating the shell.
    #[must_use]
    pub(crate) fn acquire() -> Self {
        #[cfg(windows)]
        {
            // A null handler with `add = TRUE` makes this process ignore
            // Ctrl+C. It does not affect other console processes.
            let installed =
                unsafe { windows_sys::Win32::System::Console::SetConsoleCtrlHandler(None, 1) != 0 };

            Self { installed }
        }

        #[cfg(unix)]
        {
            // `signal` is portable across the Unix targets KERNEXT supports and
            // keeps the guard free of platform-specific `sigaction` layouts.
            // SAFETY: `SIG_IGN` is a valid disposition for `SIGINT`.
            let previous = unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };

            // Failure only means the shell keeps its previous disposition,
            // which is a degraded but safe outcome.
            Self {
                previous: (previous != libc::SIG_ERR).then_some(previous),
            }
        }
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        if self.installed {
            unsafe {
                windows_sys::Win32::System::Console::SetConsoleCtrlHandler(None, 0);
            }
        }

        #[cfg(unix)]
        if let Some(previous) = self.previous {
            // SAFETY: `previous` came from a successful `signal` call, so it is
            // a valid disposition to restore.
            unsafe {
                libc::signal(libc::SIGINT, previous);
            }
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
    /// Builds a report for output that was streamed instead of captured.
    pub(crate) fn from_status(status: ProcessStatus) -> Self {
        Self {
            status,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

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
        /// Operating-system detail, when one is available.
        detail: Option<String>,
    },
    /// The file exists and is executable in principle, but this platform cannot
    /// load it (for example a Mach-O binary on Windows, or a text file with a
    /// missing shebang).
    InvalidFormat {
        /// Program name as typed by the user.
        program: String,
        /// File that could not be executed.
        path: PathBuf,
        /// Operating-system detail.
        detail: String,
    },
    /// The executable exists but the interpreter or library it needs does not.
    ///
    /// A script whose shebang points at a missing interpreter, or a Windows
    /// image whose dependent DLL is absent, resolves successfully and then
    /// fails at load time.
    MissingInterpreter {
        /// Program name as typed by the user.
        program: String,
        /// What is missing, in plain language.
        detail: String,
    },
    /// The program name cannot be turned into a valid operating-system path.
    InvalidPath {
        /// Program name as typed by the user.
        program: String,
        /// Why the name is invalid.
        reason: String,
    },
    /// The arguments cannot be passed to this program without altering them.
    ///
    /// Windows command scripts are interpreted by `cmd.exe`, which has no
    /// escaping for some characters. Rather than silently changing an argument
    /// (or letting it become an extra command), launching fails with this error.
    Argument {
        /// Program name as typed by the user.
        program: String,
        /// Why the arguments cannot be forwarded safely.
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
            | Self::PermissionDenied { program, .. }
            | Self::InvalidFormat { program, .. }
            | Self::MissingInterpreter { program, .. }
            | Self::InvalidPath { program, .. }
            | Self::Argument { program, .. }
            | Self::Spawn { program, .. }
            | Self::Wait { program, .. } => program,
        }
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { program } => write!(formatter, "command not found: {program}"),
            Self::PermissionDenied { program, detail } => {
                write!(formatter, "permission denied: {program}")?;

                match detail {
                    Some(detail) => write!(formatter, ": {detail}"),
                    None => Ok(()),
                }
            }
            Self::InvalidFormat {
                program,
                path,
                detail,
            } => write!(
                formatter,
                "cannot execute '{program}': {} is not a valid executable for this platform ({detail})",
                path.display()
            ),
            Self::MissingInterpreter { program, detail } => {
                write!(formatter, "cannot run '{program}': {detail}")
            }
            Self::InvalidPath { program, reason } => {
                write!(formatter, "invalid executable path '{program}': {reason}")
            }
            Self::Argument { program, reason } => {
                write!(formatter, "cannot pass arguments to '{program}': {reason}")
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
            Self::NotFound { .. }
            | Self::PermissionDenied { .. }
            | Self::InvalidFormat { .. }
            | Self::MissingInterpreter { .. }
            | Self::InvalidPath { .. }
            | Self::Argument { .. } => None,
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
        Ok(self.resolved_program()?.path)
    }

    /// Resolves the program and records how it must be launched.
    fn resolved_program(&self) -> Result<ResolvedProgram, ProcessError> {
        let working_directory = match &self.working_directory {
            Some(directory) => directory.clone(),
            None => env::current_dir().map_err(|source| ProcessError::Spawn {
                program: self.program.clone(),
                source,
            })?,
        };

        let path = self.environment_lookup("PATH");
        let pathext = self.environment_lookup("PATHEXT");

        resolve_program_with_path(
            &self.program,
            &working_directory,
            path.as_deref(),
            pathext.as_deref(),
        )
    }

    /// Looks a variable up in the command's overrides, then the process
    /// environment unless the environment was cleared for the child.
    fn environment_lookup(&self, name: &str) -> Option<OsString> {
        environment_value(&self.environment, name).or_else(|| {
            if self.clear_environment {
                None
            } else {
                env::var_os(name)
            }
        })
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
                // Ctrl+C must stop the child, not the shell.
                let guard = InterruptGuard::acquire();
                let mut child =
                    self.spawn_with(Stdio::inherit(), Stdio::inherit(), Stdio::inherit())?;
                let status = child.wait().map_err(|source| ProcessError::Wait {
                    program: self.program.clone(),
                    source,
                })?;
                drop(guard);

                Ok(ProcessReport {
                    status: ProcessStatus::from_exit_status(status),
                    stdout: String::new(),
                    stderr: String::new(),
                })
            }
            StdioMode::Pty => crate::pty::run_interactive(&self),
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
        let resolved = self.resolved_program()?;
        let mut command = self.build_command(&resolved)?;

        command.stdin(stdin).stdout(stdout).stderr(stderr);

        command
            .spawn()
            .map_err(|source| map_spawn_error(&self.program, &resolved.path, source))
    }

    /// Working directory the child will run in, if one was set explicitly.
    pub(crate) fn working_directory(&self) -> Option<&Path> {
        self.working_directory.as_deref()
    }

    /// Environment overrides applied to the child, in insertion order.
    pub(crate) fn environment_entries(&self) -> &[(OsString, OsString)] {
        &self.environment
    }

    /// Whether the child receives only the configured overrides.
    pub(crate) const fn clears_environment(&self) -> bool {
        self.clear_environment
    }

    /// Resolves how the operating system must start this program.
    ///
    /// The console engine and the pseudoterminal engine share this so a
    /// Windows command script is launched identically in both.
    pub(crate) fn launch_spec(&self) -> Result<LaunchSpec, ProcessError> {
        let resolved = self.resolved_program()?;

        Ok(match resolved.kind {
            ExecutableKind::Native => LaunchSpec {
                executable: resolved.path,
                arguments: self.arguments.clone(),
                raw_arguments: None,
            },
            #[cfg(windows)]
            ExecutableKind::BatchScript => LaunchSpec {
                executable: self.command_interpreter(),
                arguments: ["/d", "/v:off", "/s", "/c"].map(str::to_owned).to_vec(),
                raw_arguments: Some(batch_command_line(
                    &resolved.path,
                    &self.arguments,
                    &self.program,
                )?),
            },
        })
    }

    /// Builds the process command for a resolved program.
    ///
    /// Native images run directly. Windows command scripts run through the
    /// system interpreter, which receives one pre-escaped command line.
    fn build_command(&self, resolved: &ResolvedProgram) -> Result<Command, ProcessError> {
        let spec = self.launch_spec_for(resolved)?;
        let mut command = Command::new(&spec.executable);

        command.args(&spec.arguments);

        if let Some(raw) = &spec.raw_arguments {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;

                // Only Windows command scripts use a pre-escaped command line.
                command.raw_arg(raw);
            }

            #[cfg(not(windows))]
            let _ = raw;
        }

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

        reset_child_interrupt_disposition(&mut command);

        Ok(command)
    }

    /// Builds launch information for an already-resolved program.
    fn launch_spec_for(&self, resolved: &ResolvedProgram) -> Result<LaunchSpec, ProcessError> {
        Ok(match resolved.kind {
            ExecutableKind::Native => LaunchSpec {
                executable: resolved.path.clone(),
                arguments: self.arguments.clone(),
                raw_arguments: None,
            },
            #[cfg(windows)]
            ExecutableKind::BatchScript => LaunchSpec {
                executable: self.command_interpreter(),
                // `/d` skips AutoRun registry commands, `/v:off` disables
                // delayed expansion (so `!` stays literal), and `/s` gives the
                // outer quoting below a predictable, documented meaning.
                arguments: ["/d", "/v:off", "/s", "/c"].map(str::to_owned).to_vec(),
                raw_arguments: Some(batch_command_line(
                    &resolved.path,
                    &self.arguments,
                    &self.program,
                )?),
            },
        })
    }

    /// Interpreter used to run `.cmd` and `.bat` programs.
    ///
    /// `%COMSPEC%` is trusted only when it is an absolute path to an existing
    /// file, so a relative or stale value cannot redirect script execution to an
    /// unrelated binary. Otherwise the system directory is used.
    #[cfg(windows)]
    fn command_interpreter(&self) -> PathBuf {
        if let Some(comspec) = self.environment_lookup("COMSPEC") {
            let candidate = PathBuf::from(comspec);

            if candidate.is_absolute() && candidate.is_file() {
                return candidate;
            }
        }

        for root in ["SystemRoot", "windir"] {
            if let Some(root) = self.environment_lookup(root) {
                let candidate = Path::new(&root).join("System32").join("cmd.exe");

                if candidate.is_file() {
                    return candidate;
                }
            }
        }

        // Last resort: let process creation search the system directories.
        PathBuf::from("cmd.exe")
    }
}

/// Turns an operating-system spawn failure into an actionable shell error.
///
/// Resolution has already found the file, so a `NotFound` from the loader means
/// something the program depends on is missing (a script interpreter, or a DLL
/// on Windows), not that the command itself is unknown.
pub(crate) fn map_spawn_error(program: &str, path: &Path, source: io::Error) -> ProcessError {
    if source.kind() == io::ErrorKind::PermissionDenied {
        return ProcessError::PermissionDenied {
            program: program.to_owned(),
            detail: Some(source.to_string()),
        };
    }

    // `execve` reports a badly formed image as `ENOEXEC`; `CreateProcess` uses
    // `ERROR_BAD_EXE_FORMAT` (193). Rust maps both to `Uncategorized`, so the
    // raw code is the only reliable signal.
    #[cfg(unix)]
    const BAD_FORMAT: i32 = libc::ENOEXEC;
    #[cfg(windows)]
    const BAD_FORMAT: i32 = 193;

    if source.raw_os_error() == Some(BAD_FORMAT) {
        return ProcessError::InvalidFormat {
            program: program.to_owned(),
            path: path.to_path_buf(),
            detail: source.to_string(),
        };
    }

    if source.kind() == io::ErrorKind::NotFound {
        return ProcessError::MissingInterpreter {
            program: program.to_owned(),
            detail: format!(
                "'{}' exists but its interpreter or a required library was not found ({source})",
                path.display()
            ),
        };
    }

    // `CreateProcess` reports a missing dependent DLL as an NTSTATUS rather
    // than an `io::ErrorKind`.
    #[cfg(windows)]
    if source.raw_os_error().is_some_and(|code| {
        code as u32 == 0xC000_0135 // STATUS_DLL_NOT_FOUND
            || code as u32 == 0xC000_0139 // STATUS_ENTRYPOINT_NOT_FOUND
    }) {
        return ProcessError::MissingInterpreter {
            program: program.to_owned(),
            detail: format!(
                "a library required by '{}' could not be loaded ({source})",
                path.display()
            ),
        };
    }

    ProcessError::Spawn {
        program: program.to_owned(),
        source,
    }
}

/// Restores the default Ctrl+C disposition for a child on Unix.
///
/// [`InterruptGuard`] ignores `SIGINT` in the shell while a foreground child
/// runs, and signal dispositions are inherited across `exec`. Resetting it in
/// the child would otherwise make Ctrl+C do nothing. `signal` is
/// async-signal-safe, which is what `pre_exec` requires; the same is implicit on
/// Windows, where the handler is per process.
fn reset_child_interrupt_disposition(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        // SAFETY: the closure only calls an async-signal-safe function and does
        // not allocate.
        unsafe {
            command.pre_exec(|| {
                libc::signal(libc::SIGINT, libc::SIG_DFL);
                Ok(())
            });
        }
    }

    #[cfg(not(unix))]
    {
        let _ = command;
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

/// Resolves `program` to an executable path, ignoring how it must be launched.
fn resolve_with_path(
    program: &str,
    working_directory: &Path,
    path: Option<&OsStr>,
) -> Result<PathBuf, ProcessError> {
    #[cfg(windows)]
    let pathext = env::var_os("PATHEXT");
    #[cfg(not(windows))]
    let pathext: Option<OsString> = None;

    resolve_program_with_path(program, working_directory, path, pathext.as_deref())
        .map(|resolved| resolved.path)
}

fn resolve_program_with_path(
    program: &str,
    working_directory: &Path,
    path: Option<&OsStr>,
    pathext: Option<&OsStr>,
) -> Result<ResolvedProgram, ProcessError> {
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

        return resolve_candidates(append_platform_extensions(&candidate, pathext), program);
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

        let candidates = append_platform_extensions(&directory.join(program), pathext);

        match resolve_candidates(candidates, program) {
            Ok(resolved) => return Ok(resolved),
            Err(ProcessError::PermissionDenied { .. }) => found_but_not_executable = true,
            Err(_) => {}
        }
    }

    if found_but_not_executable {
        Err(ProcessError::PermissionDenied {
            program: program.to_owned(),
            detail: Some("the match is not an executable file".to_owned()),
        })
    } else {
        Err(ProcessError::NotFound {
            program: program.to_owned(),
        })
    }
}

fn resolve_candidates(
    candidates: Vec<PathBuf>,
    program: &str,
) -> Result<ResolvedProgram, ProcessError> {
    let mut found_but_not_executable = false;
    #[cfg(windows)]
    let mut unsupported_extension = None;

    for candidate in candidates {
        match executable_state(&candidate) {
            ExecutableState::Executable => {
                #[cfg(windows)]
                {
                    if let Some(kind) = windows_program_kind(&candidate) {
                        return Ok(ResolvedProgram {
                            path: candidate,
                            kind,
                        });
                    }

                    if unsupported_extension.is_none() {
                        unsupported_extension = Some(candidate);
                    }
                }

                #[cfg(not(windows))]
                {
                    return Ok(ResolvedProgram {
                        path: candidate,
                        kind: ExecutableKind::Native,
                    });
                }
            }
            ExecutableState::NotExecutable => found_but_not_executable = true,
            ExecutableState::Missing => {}
        }
    }

    #[cfg(windows)]
    if let Some(candidate) = unsupported_extension {
        return Err(ProcessError::InvalidPath {
            program: program.to_owned(),
            reason: format!(
                "Windows cannot start '{}' directly; only .exe, .com, .cmd and .bat files are executable",
                candidate.display()
            ),
        });
    }

    if found_but_not_executable {
        Err(ProcessError::PermissionDenied {
            program: program.to_owned(),
            detail: Some("the file exists but is not executable".to_owned()),
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
fn append_platform_extensions(base: &Path, pathext: Option<&OsStr>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // A name that already carries an extension is tried exactly as written and
    // then with each executable extension appended, the way `cmd.exe` searches.
    // This is what lets `python3.12` find `python3.12.exe` instead of failing
    // because the name "already has an extension".
    //
    // A name with no extension never gets an extensionless candidate: Windows
    // cannot start those, and npm/pnpm install one next to every `.cmd` shim.
    if base.extension().is_some() {
        candidates.push(base.to_path_buf());
    }

    for extension in windows_executable_extensions(pathext) {
        let mut candidate = base.as_os_str().to_os_string();
        candidate.push(&extension);
        let candidate = PathBuf::from(candidate);

        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }

    candidates
}

#[cfg(not(windows))]
fn append_platform_extensions(base: &Path, _pathext: Option<&OsStr>) -> Vec<PathBuf> {
    vec![base.to_path_buf()]
}

/// Extensions KERNEXT will launch, in the order `PATHEXT` declares.
///
/// Only native images (`.exe`, `.com`) and command scripts (`.cmd`, `.bat`) are
/// supported. Other `PATHEXT` entries (`.vbs`, `.js`, `.ps1`, `.msc`, ...) need
/// a different host program and are deliberately not launched directly.
#[cfg(windows)]
fn windows_executable_extensions(pathext: Option<&OsStr>) -> Vec<String> {
    const SUPPORTED: [&str; 4] = [".com", ".exe", ".bat", ".cmd"];
    const DEFAULT: &str = ".COM;.EXE;.BAT;.CMD";

    let raw = pathext.map_or_else(|| OsString::from(DEFAULT), OsString::from);
    let mut extensions: Vec<String> = Vec::new();

    for extension in raw.to_string_lossy().split(';') {
        let extension = extension.trim();

        if extension.is_empty() {
            continue;
        }

        let extension = if extension.starts_with('.') {
            extension.to_ascii_lowercase()
        } else {
            format!(".{}", extension.to_ascii_lowercase())
        };

        if SUPPORTED.contains(&extension.as_str()) && !extensions.contains(&extension) {
            extensions.push(extension);
        }
    }

    if extensions.is_empty() {
        return SUPPORTED.iter().map(ToString::to_string).collect();
    }

    extensions
}

/// Classifies a Windows file by the launcher it requires.
#[cfg(windows)]
fn windows_program_kind(path: &Path) -> Option<ExecutableKind> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();

    match extension.as_str() {
        "exe" | "com" => Some(ExecutableKind::Native),
        "bat" | "cmd" => Some(ExecutableKind::BatchScript),
        _ => None,
    }
}

/// Builds the complete command line handed to `cmd.exe` for a command script.
///
/// The shape is `""<script>" "<arg>"..."`: `/s` strips exactly the outer pair
/// of quotes, leaving the quoted script and arguments for normal `cmd.exe`
/// parsing. Every argument is quoted, so `&`, `|`, `<`, `>`, `(`, `)`, `^` and
/// `!` stay literal instead of becoming operators.
#[cfg(windows)]
fn batch_command_line(
    script: &Path,
    arguments: &[String],
    program: &str,
) -> Result<String, ProcessError> {
    let script = script.to_str().ok_or_else(|| ProcessError::InvalidPath {
        program: program.to_owned(),
        reason: "the script path is not valid Unicode".to_owned(),
    })?;

    if script.contains('%') {
        return Err(ProcessError::Argument {
            program: program.to_owned(),
            reason: "the script path contains '%', which cmd.exe would expand".to_owned(),
        });
    }

    let mut line = String::with_capacity(script.len() + 2);
    line.push('"');
    line.push_str(script);
    line.push('"');

    for argument in arguments {
        line.push(' ');
        push_batch_argument(argument, program, &mut line)?;
    }

    Ok(format!("\"{line}\""))
}

/// Appends one quoted argument to a `cmd.exe` command line.
///
/// `cmd.exe` has no escaping for `%` (expanded before any other parsing), and a
/// literal `"` would terminate the quoted region and let the rest of the
/// argument become commands. Both, plus line breaks and NUL bytes, are refused
/// rather than silently rewritten.
#[cfg(windows)]
fn push_batch_argument(
    argument: &str,
    program: &str,
    line: &mut String,
) -> Result<(), ProcessError> {
    if let Some(character) = argument
        .chars()
        .find(|character| matches!(character, '\0' | '\n' | '\r' | '"' | '%'))
    {
        let reason = match character {
            '\0' => "arguments containing NUL bytes cannot be passed to a command script",
            '\n' | '\r' => "arguments containing line breaks cannot be passed to a command script",
            '"' => {
                "arguments containing double quotes cannot be passed to a command script without changing their meaning"
            }
            _ => {
                "arguments containing '%' would be expanded by cmd.exe, so KERNEXT refuses to alter them"
            }
        };

        return Err(ProcessError::Argument {
            program: program.to_owned(),
            reason: reason.to_owned(),
        });
    }

    line.push('"');
    line.push_str(argument);
    line.push('"');

    Ok(())
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
