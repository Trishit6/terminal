//! Integration tests for external command execution.
//!
//! The test binary doubles as a portable helper program. When the shell spawns
//! it with `CUSTOM_SHELL_TEST_HELPER` set, the helper test prints its arguments,
//! working directory, and environment, or exits with a non-zero status. When the
//! suite runs normally the variable is absent and the helper is a no-op.

use shell_builtins::{Dispatcher, ExternalCommand, ProcessError, ProcessOptions, StdioMode};
use shell_runtime::Session;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const HELPER_ENV: &str = "CUSTOM_SHELL_TEST_HELPER";
const ENVIRONMENT_MARKER: &str = "CUSTOM_SHELL_TEST_VALUE";
const ARGUMENT_MARKER: &str = "__custom_shell_helper_args__";
const HELPER_TEST: &str = "external_command_helper";

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

/// Portable helper process used by the execution tests.
#[test]
fn external_command_helper() {
    let Ok(mode) = env::var(HELPER_ENV) else {
        return;
    };

    let arguments: Vec<String> = env::args()
        .skip_while(|argument| argument != ARGUMENT_MARKER)
        .skip(1)
        .collect();

    match mode.as_str() {
        "echo" => {
            let directory = env::current_dir().expect("helper should know its directory");
            println!("stdout-marker");
            println!("args={}", arguments.join("|"));
            println!("cwd={}", directory.display());

            if let Ok(value) = env::var(ENVIRONMENT_MARKER) {
                println!("env={value}");
            }

            eprintln!("stderr-marker");
        }
        "fail" => {
            eprintln!("failing-helper");
            std::process::exit(7);
        }
        _ => {}
    }
}

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn new(label: &str) -> Self {
        let unique = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "kernext-external-{label}-{}-{unique}",
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

fn executable_text() -> String {
    current_executable().to_string_lossy().into_owned()
}

fn helper_command() -> ExternalCommand {
    ExternalCommand::new(executable_text())
        .args(["--exact", HELPER_TEST, "--nocapture", ARGUMENT_MARKER])
        .env(HELPER_ENV, "echo")
        .stdio(StdioMode::Capture)
}

fn session() -> Session {
    Session::new(Default::default()).expect("session should start")
}

fn executable_suffix() -> &'static str {
    if cfg!(windows) { ".exe" } else { "" }
}

fn relative_program_name(path: &Path) -> String {
    let name = path.file_name().expect("executable has a name");
    format!(".{}{}", std::path::MAIN_SEPARATOR, name.to_string_lossy())
}

fn install_helper(directory: &TempDirectory, name: &str) -> String {
    let name = format!("{name}{}", executable_suffix());
    let target = directory.path().join(&name);

    // On Unix, symlink to the already-running test binary. Copying it can race
    // with a concurrent fork and produce ETXTBSY when the copy is still open
    // for writing. Windows keeps a real copy because symlinks need privileges.
    #[cfg(unix)]
    std::os::unix::fs::symlink(current_executable(), &target)
        .expect("helper symlink should be created");
    #[cfg(not(unix))]
    fs::copy(current_executable(), &target).expect("helper should be copied");

    name
}

#[test]
fn absolute_paths_execute_and_capture_stdout_and_stderr() {
    let report = helper_command()
        .arg("one")
        .run()
        .expect("helper should run");

    assert!(report.success());
    assert_eq!(report.exit_code(), Some(0));
    assert!(report.stdout().contains("stdout-marker"), "{report:?}");
    assert!(report.stdout().contains("args=one"), "{report:?}");
    assert!(report.stderr().contains("stderr-marker"), "{report:?}");
}

#[test]
fn arguments_with_spaces_and_quotes_stay_separate() {
    let report = helper_command()
        .arg("hello world")
        .arg("it's quoted")
        .arg("plain")
        .run()
        .expect("helper should run");

    assert!(
        report
            .stdout()
            .contains("args=hello world|it's quoted|plain"),
        "{report:?}"
    );
}

#[test]
fn relative_paths_resolve_against_the_working_directory() {
    let executable = current_executable();
    let directory = executable.parent().expect("executable has a parent");
    let program = relative_program_name(&executable);

    let report = ExternalCommand::new(program)
        .args(["--exact", HELPER_TEST, "--nocapture", ARGUMENT_MARKER])
        .env(HELPER_ENV, "echo")
        .current_dir(directory)
        .stdio(StdioMode::Capture)
        .run()
        .expect("relative helper should run");

    assert!(report.success(), "{report:?}");
}

#[test]
fn bare_names_resolve_through_path() {
    let directory = TempDirectory::new("path");
    let name = install_helper(&directory, "path-helper");

    let report = ExternalCommand::new(&name)
        .args(["--exact", HELPER_TEST, "--nocapture", ARGUMENT_MARKER])
        .env(HELPER_ENV, "echo")
        .env("PATH", directory.path())
        .stdio(StdioMode::Capture)
        .run()
        .expect("PATH lookup should find the helper");

    assert!(report.success(), "{report:?}");
}

#[test]
fn working_directory_is_propagated_to_the_child() {
    let directory = TempDirectory::new("working-directory");
    let marker = directory
        .path()
        .file_name()
        .expect("temp directory has a name")
        .to_string_lossy()
        .into_owned();

    let report = helper_command()
        .current_dir(directory.path())
        .run()
        .expect("helper should run");

    assert!(
        report.stdout().contains("cwd=") && report.stdout().contains(&marker),
        "{report:?}"
    );
}

#[test]
fn environment_overrides_reach_the_child() {
    let report = helper_command()
        .env(ENVIRONMENT_MARKER, "propagated")
        .run()
        .expect("helper should run");

    assert!(report.stdout().contains("env=propagated"), "{report:?}");
}

#[test]
fn non_zero_exit_status_is_reported() {
    let report = ExternalCommand::new(executable_text())
        .args(["--exact", HELPER_TEST, "--nocapture", ARGUMENT_MARKER])
        .env(HELPER_ENV, "fail")
        .stdio(StdioMode::Capture)
        .run()
        .expect("helper should start even when it fails");

    assert!(!report.success());
    assert_eq!(report.exit_code(), Some(7));
    assert!(report.stderr().contains("failing-helper"), "{report:?}");
}

#[test]
fn unknown_executables_report_not_found() {
    let directory = TempDirectory::new("not-found");

    let error = ExternalCommand::new("definitely-not-a-real-command-xyz")
        .current_dir(directory.path())
        .env("PATH", directory.path())
        .stdio(StdioMode::Capture)
        .run()
        .expect_err("missing command should fail");

    assert!(matches!(error, ProcessError::NotFound { .. }));
    assert_eq!(
        error.to_string(),
        "command not found: definitely-not-a-real-command-xyz"
    );
}

#[cfg(unix)]
#[test]
fn files_without_execute_permission_report_permission_denied() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDirectory::new("permission");
    let target = directory.path().join("not-executable");
    fs::write(&target, b"#!/bin/sh\n").expect("file should be written");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o644))
        .expect("permissions should be set");

    let error = ExternalCommand::new(target.to_string_lossy().into_owned())
        .stdio(StdioMode::Capture)
        .run()
        .expect_err("non-executable file should fail");

    assert!(matches!(error, ProcessError::PermissionDenied { .. }));
}

#[cfg(unix)]
#[test]
fn invalid_executables_report_failures() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDirectory::new("spawn-error");
    let target = directory.path().join("invalid-format");
    fs::write(&target, b"this is not an executable format\n").expect("file should be written");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755))
        .expect("permissions should be set");

    // Executing a text file that is not a valid binary fails everywhere, but
    // how it fails depends on the libc: glibc and Darwin's execvp retry a
    // non-ELF file through /bin/sh exactly like a POSIX shell would, so the
    // child runs, prints "this: not found", and exits 127; musl reports
    // ENOEXEC from the spawn instead. Either way the user sees a real error.
    match ExternalCommand::new(target.to_string_lossy().into_owned())
        .stdio(StdioMode::Capture)
        .run()
    {
        Ok(report) => {
            assert!(
                !report.success(),
                "shell fallback should report failure: {report:?}"
            );
            assert_ne!(
                report.exit_code(),
                Some(0),
                "shell fallback should not exit zero: {report:?}"
            );
            assert!(
                !report.stderr().trim().is_empty(),
                "shell fallback should explain the failure: {report:?}"
            );
        }
        Err(ProcessError::Spawn { .. }) => {}
        Err(other) => panic!("unexpected error for invalid executable: {other:?}"),
    }
}

#[test]
fn built_ins_take_precedence_over_programs_on_path() {
    let directory = TempDirectory::new("precedence");
    install_helper(&directory, "echo");

    let options = ProcessOptions::new()
        .with_stdio(StdioMode::Capture)
        .with_env("PATH", directory.path());
    let mut dispatcher = Dispatcher::with_options(options);
    let mut session = session();

    let outcome = dispatcher.execute(&mut session, "echo hello world");

    assert_eq!(outcome.output(), Some("hello world\n"), "{outcome:?}");
    assert!(outcome.success());
    assert_eq!(outcome.exit_code(), None);
}

#[test]
fn dispatcher_runs_externals_and_preserves_quoted_arguments() {
    let directory = TempDirectory::new("quoted-arguments");
    let helper = install_helper(&directory, "quote-helper");

    let options = ProcessOptions::new()
        .with_stdio(StdioMode::Capture)
        .with_env("PATH", directory.path())
        .with_env(HELPER_ENV, "echo");
    let mut dispatcher = Dispatcher::with_options(options);
    let mut session = session();

    let command = format!(
        "{helper} --exact {HELPER_TEST} --nocapture {ARGUMENT_MARKER} \"hello world\" 'single quoted'"
    );
    let outcome = dispatcher.execute(&mut session, &command);

    let output = outcome
        .output()
        .expect("external command should produce output");
    assert!(
        output.contains("args=hello world|single quoted"),
        "{output}"
    );
    assert!(outcome.success(), "{outcome:?}");
    assert_eq!(outcome.exit_code(), Some(0));
}

#[test]
fn dispatcher_reports_non_zero_external_status() {
    let directory = TempDirectory::new("non-zero");
    let helper = install_helper(&directory, "fail-helper");

    let options = ProcessOptions::new()
        .with_stdio(StdioMode::Capture)
        .with_env("PATH", directory.path())
        .with_env(HELPER_ENV, "fail");
    let mut dispatcher = Dispatcher::with_options(options);
    let mut session = session();

    let command = format!("{helper} --exact {HELPER_TEST} --nocapture {ARGUMENT_MARKER}");
    let outcome = dispatcher.execute(&mut session, &command);

    assert!(!outcome.success(), "{outcome:?}");
    assert_eq!(outcome.exit_code(), Some(7));
    assert!(
        outcome
            .output()
            .is_some_and(|output| output.contains("failing-helper")),
        "{outcome:?}"
    );
}

#[test]
fn external_exit_status_drives_logical_chaining() {
    let directory = TempDirectory::new("logical-chain");
    let helper = install_helper(&directory, "chain-helper");

    let options = ProcessOptions::new()
        .with_stdio(StdioMode::Capture)
        .with_env("PATH", directory.path())
        .with_env(HELPER_ENV, "fail");
    let mut dispatcher = Dispatcher::with_options(options);
    let mut session = session();

    let command =
        format!("{helper} --exact {HELPER_TEST} --nocapture {ARGUMENT_MARKER} || echo recovered");
    let outcome = dispatcher.execute(&mut session, &command);

    let output = outcome.output().expect("chain should produce output");
    assert!(output.contains("failing-helper"), "{output}");
    assert!(output.contains("recovered"), "{output}");
    assert!(outcome.success(), "{outcome:?}");
    assert_eq!(outcome.exit_code(), None);
}

#[test]
fn dispatcher_reports_unknown_externals_without_stopping() {
    let directory = TempDirectory::new("dispatcher-missing");
    let options = ProcessOptions::new()
        .with_stdio(StdioMode::Capture)
        .with_env("PATH", directory.path());
    let mut dispatcher = Dispatcher::with_options(options);
    let mut session = session();

    let outcome = dispatcher.execute(&mut session, "definitely-not-a-real-command-xyz");

    assert_eq!(
        outcome.output(),
        Some("Command not found: definitely-not-a-real-command-xyz")
    );
    assert!(!outcome.success());
    assert_eq!(outcome.exit_code(), None);

    let follow_up = dispatcher.execute(&mut session, "version");
    assert!(follow_up.success());
    assert!(
        follow_up
            .output()
            .is_some_and(|output| output.starts_with("KERNEXT v"))
    );
}

#[test]
fn session_exports_reach_external_children() {
    let options = ProcessOptions::new()
        .with_stdio(StdioMode::Capture)
        .with_env(HELPER_ENV, "echo");
    let mut dispatcher = Dispatcher::with_options(options);
    let mut session = session();

    let exported = dispatcher.execute(&mut session, "export CUSTOM_SHELL_TEST_VALUE=from-shell");
    assert!(exported.success(), "{exported:?}");

    let command = format!(
        "{} --exact {HELPER_TEST} --nocapture {ARGUMENT_MARKER}",
        executable_text()
    );
    let outcome = dispatcher.execute(&mut session, &command);

    let output = outcome
        .output()
        .expect("external command should produce output");
    assert!(output.contains("env=from-shell"), "{output}");
    assert!(outcome.success(), "{outcome:?}");
}
