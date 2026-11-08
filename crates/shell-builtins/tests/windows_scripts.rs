//! Windows command-script launch tests.
//!
//! These cover the launch path that produces `os error 193` when it is wrong:
//! `opencode`, `omnirush` and every other npm/pnpm-installed tool are `.cmd`
//! shims, not native images, and an extensionless POSIX `sh` shim with the same
//! name sits next to them. Resolution must pick the `.cmd` shim and run it
//! through `cmd.exe`, never hand the `sh` shim to `CreateProcess`.
//!
//! Everything here uses isolated temp directories. No test depends on a
//! particular program being installed beyond the Windows command interpreter.

#![cfg(windows)]

use shell_builtins::{Dispatcher, ExternalCommand, ProcessError, ProcessOptions, StdioMode};
use shell_runtime::Session;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

/// Set on the shim's child processes so the copied test binary prints argv.
const SCRIPT_HELPER_ENV: &str = "KERNEXT_SCRIPT_HELPER";
/// Separates the harness arguments from the arguments under test.
const SCRIPT_HELPER_MARKER: &str = "KERNEXT_SCRIPT_ARGS";

/// Native helper that prints each argument on its own line.
///
/// Scripts forward `%*` to this program, which is how an npm shim works: the
/// original quoted text is re-inserted, so this measures what the launcher
/// actually handed to `cmd.exe` rather than what the script re-parsed.
#[test]
fn windows_script_helper() {
    if env::var(SCRIPT_HELPER_ENV).is_err() {
        return;
    }

    for (index, argument) in env::args()
        .skip_while(|argument| argument != SCRIPT_HELPER_MARKER)
        .skip(1)
        .enumerate()
    {
        println!("arg{index}={argument}");
    }
}

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn new(label: &str) -> Self {
        let unique = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "kernext-windows-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp directory should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    /// Writes a command script with CRLF line endings so `cmd.exe` parses it.
    fn write_script(&self, name: &str, body: &str) -> PathBuf {
        let path = self.path().join(name);
        let mut contents = String::from("@echo off\r\n");
        contents.push_str(&body.replace('\n', "\r\n"));
        contents.push_str("\r\n");
        fs::write(&path, contents).expect("script should be written");
        path
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn session(directory: &Path) -> Session {
    let mut session = Session::new(Default::default()).expect("session should start");
    session.set_working_directory(directory.to_path_buf());
    session
}

/// Dispatcher that resolves bare names only inside the test directory and
/// captures output, so tests never touch the host `PATH`.
fn dispatcher(directory: &Path) -> Dispatcher {
    Dispatcher::with_options(
        ProcessOptions::new()
            .with_stdio(StdioMode::Capture)
            .with_env("PATH", directory.as_os_str()),
    )
}

/// Installs the copied test binary plus a shim that forwards `%*` to it.
fn install_argument_shim(directory: &TempDirectory) {
    fs::copy(
        env::current_exe().expect("test executable should be known"),
        directory.path().join("argv.exe"),
    )
    .expect("helper executable should be copied");
    directory.write_script(
        "shim.cmd",
        &format!(
            "\"%~dp0argv.exe\" --exact windows_script_helper --nocapture {SCRIPT_HELPER_MARKER} %*\n"
        ),
    );
}

fn argument_shim_dispatcher(directory: &Path) -> Dispatcher {
    Dispatcher::with_options(
        ProcessOptions::new()
            .with_stdio(StdioMode::Capture)
            .with_env("PATH", directory.as_os_str())
            .with_env(SCRIPT_HELPER_ENV, "argv"),
    )
}

fn run(directory: &Path, input: &str) -> shell_builtins::CommandOutcome {
    let mut dispatcher = dispatcher(directory);
    let mut session = session(directory);
    dispatcher.execute(&mut session, input)
}

fn text(outcome: &shell_builtins::CommandOutcome) -> String {
    outcome.output().unwrap_or_default().replace("\r\n", "\n")
}

#[test]
fn cmd_script_runs_through_the_command_interpreter() {
    let directory = TempDirectory::new("cmd");
    directory.write_script("hello.cmd", "echo hello from cmd\n");

    let outcome = run(directory.path(), "hello");

    assert_eq!(text(&outcome), "hello from cmd\n", "{outcome:?}");
    assert!(outcome.success(), "{outcome:?}");
    assert_eq!(outcome.exit_code(), Some(0));
}

#[test]
fn bat_script_runs_through_the_command_interpreter() {
    let directory = TempDirectory::new("bat");
    directory.write_script("legacy.bat", "echo bat-ok\n");

    let outcome = run(directory.path(), "legacy");

    assert_eq!(text(&outcome), "bat-ok\n", "{outcome:?}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn npm_style_shim_forwards_every_argument() {
    let directory = TempDirectory::new("shim");
    directory.write_script(
        "shim.cmd",
        "echo one=[%~1]\necho two=[%~2]\necho three=[%~3]\n",
    );

    let outcome = run(directory.path(), "shim \"hello world\" second third");

    assert_eq!(
        text(&outcome),
        "one=[hello world]\ntwo=[second]\nthree=[third]\n",
        "{outcome:?}"
    );
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn metacharacters_are_passed_as_literal_argument_text() {
    let directory = TempDirectory::new("metacharacters");
    install_argument_shim(&directory);

    let mut dispatcher = argument_shim_dispatcher(directory.path());
    let mut session = session(directory.path());
    let outcome = dispatcher.execute(
        &mut session,
        "shim \"c&d\" \"(x)\" \"a^b\" \"hi!there\" \"pipe|text\"",
    );

    let output = text(&outcome);
    for expected in [
        "arg0=c&d",
        "arg1=(x)",
        "arg2=a^b",
        "arg3=hi!there",
        "arg4=pipe|text",
    ] {
        assert!(
            output.contains(expected),
            "missing {expected} in {outcome:?}"
        );
    }
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn arguments_cannot_inject_extra_commands() {
    let directory = TempDirectory::new("injection");
    install_argument_shim(&directory);

    // The whole string is quoted shell input, so it is a single argument. If
    // escaping were wrong, cmd.exe would run `echo INJECTED` separately.
    let mut dispatcher = argument_shim_dispatcher(directory.path());
    let mut session = session(directory.path());
    let outcome = dispatcher.execute(&mut session, "shim \"x & echo INJECTED\"");

    let output = text(&outcome);
    assert!(
        output.contains("arg0=x & echo INJECTED"),
        "argument was altered: {outcome:?}"
    );
    assert!(
        !output.lines().any(|line| line.trim() == "INJECTED"),
        "injected command ran: {outcome:?}"
    );
    assert!(
        !directory.path().join("injected.txt").exists(),
        "injected command created a file"
    );
}

#[test]
fn percent_arguments_are_rejected_instead_of_expanded() {
    let directory = TempDirectory::new("percent");
    directory.write_script("percent.cmd", "echo one=[%~1]\n");

    let outcome = run(directory.path(), "percent \"x%PATH%y\"");

    let output = text(&outcome);
    assert!(!outcome.success(), "{outcome:?}");
    assert!(output.contains("cmd.exe"), "{output}");
    assert!(!output.contains("one=["), "{output}");
    assert!(!output.contains(';'), "expanded PATH leaked: {output}");
}

#[test]
fn double_quotes_inside_an_argument_are_rejected() {
    let directory = TempDirectory::new("quotes");
    directory.write_script("quotes.cmd", "echo one=[%~1]\n");

    let outcome = run(directory.path(), "quotes \"say \\\"hi\\\"\"");

    let output = text(&outcome);
    assert!(!outcome.success(), "{outcome:?}");
    assert!(output.contains("double quotes"), "{output}");
}

#[test]
fn exit_codes_are_preserved() {
    let directory = TempDirectory::new("exit-code");
    directory.write_script("fail.cmd", "exit /b 42\n");

    let outcome = run(directory.path(), "fail");

    assert!(!outcome.success(), "{outcome:?}");
    assert_eq!(outcome.exit_code(), Some(42), "{outcome:?}");
}

#[test]
fn stderr_redirection_captures_script_errors() {
    let directory = TempDirectory::new("stderr");
    directory.write_script("noisy.cmd", "echo oops 1>&2\n");

    let log = directory.path().join("errors.txt");
    let outcome = run(directory.path(), "noisy 2> errors.txt");

    assert!(outcome.success(), "{outcome:?}");
    let log = fs::read_to_string(&log).expect("stderr log should exist");
    assert!(log.contains("oops"), "{log}");
}

#[test]
fn script_paths_containing_spaces_resolve_and_run() {
    let directory = TempDirectory::new("spaces");
    let nested = directory.path().join("scripts with spaces");
    fs::create_dir_all(&nested).expect("nested directory should be created");

    let script = nested.join("space tool.cmd");
    fs::write(&script, "@echo off\r\necho spaced\r\n").expect("script should be written");

    let mut dispatcher = Dispatcher::with_options(
        ProcessOptions::new()
            .with_stdio(StdioMode::Capture)
            .with_env("PATH", directory.path().as_os_str()),
    );
    let mut session = session(directory.path());
    let command = format!("\"{}\"", script.display());
    let outcome = dispatcher.execute(&mut session, &command);

    assert_eq!(text(&outcome), "spaced\n", "{outcome:?}");
}

#[test]
fn unicode_arguments_survive_the_script_launcher() {
    let directory = TempDirectory::new("unicode");
    directory.write_script("unicode.cmd", "echo one=[%~1]\necho two=[%~2]\n");

    let outcome = run(directory.path(), "unicode \"héllo wörld\" \"日本語\"");

    assert_eq!(
        text(&outcome),
        "one=[héllo wörld]\ntwo=[日本語]\n",
        "{outcome:?}"
    );
}

#[test]
fn scripts_read_stdin_when_input_is_redirected() {
    let directory = TempDirectory::new("stdin");
    directory.write_script("readline.cmd", "set /p line=\necho got=%line%\n");
    fs::write(directory.path().join("input.txt"), "piped\r\n").expect("input should be written");

    let outcome = run(directory.path(), "readline < input.txt");

    assert_eq!(text(&outcome), "got=piped\n", "{outcome:?}");
}

#[test]
fn scripts_work_as_pipeline_producers() {
    let directory = TempDirectory::new("pipeline");
    directory.write_script("producer.cmd", "echo zebra\necho apple\n");

    let outcome = run(directory.path(), "producer | sort");

    assert_eq!(text(&outcome), "apple\nzebra\n", "{outcome:?}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn output_redirection_writes_script_output() {
    let directory = TempDirectory::new("redirect");
    directory.write_script("writer.cmd", "echo redirected\n");

    let outcome = run(directory.path(), "writer > out.txt");

    assert!(outcome.success(), "{outcome:?}");
    let written = fs::read_to_string(directory.path().join("out.txt")).expect("output file exists");
    assert_eq!(written.replace("\r\n", "\n"), "redirected\n");
}

#[test]
fn missing_commands_report_not_found() {
    let directory = TempDirectory::new("missing");

    let outcome = run(directory.path(), "definitely-not-a-real-command");

    let output = text(&outcome);
    assert!(!outcome.success(), "{outcome:?}");
    assert!(
        output.to_lowercase().contains("command not found"),
        "{output}"
    );
}

#[test]
fn extensionless_shims_are_not_treated_as_executables() {
    // This is the shape npm/pnpm install: a POSIX `sh` script with no extension.
    // Handing it to CreateProcess is what produced `os error 193`.
    let directory = TempDirectory::new("extensionless");
    let shim = directory.path().join("sh-only");
    fs::write(&shim, "#!/bin/sh\necho should-not-run\n").expect("shim should be written");

    let error = ExternalCommand::new("sh-only")
        .env("PATH", directory.path().as_os_str())
        .resolve()
        .expect_err("extensionless files are not Windows executables");

    assert!(matches!(error, ProcessError::NotFound { .. }), "{error:?}");
}

#[test]
fn pathext_order_decides_between_native_and_script_candidates() {
    let directory = TempDirectory::new("pathext");
    directory.write_script("tool.cmd", "echo from-cmd\n");
    fs::copy(
        env::current_exe().expect("test executable should be known"),
        directory.path().join("tool.exe"),
    )
    .expect("helper executable should be copied");

    let native_first = ExternalCommand::new("tool")
        .env("PATH", directory.path().as_os_str())
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .resolve()
        .expect("tool should resolve");
    let script_first = ExternalCommand::new("tool")
        .env("PATH", directory.path().as_os_str())
        .env("PATHEXT", ".CMD;.EXE")
        .resolve()
        .expect("tool should resolve");

    assert_eq!(
        native_first.file_name().and_then(|name| name.to_str()),
        Some("tool.exe")
    );
    assert_eq!(
        script_first.file_name().and_then(|name| name.to_str()),
        Some("tool.cmd")
    );
}

#[test]
fn explicit_script_paths_are_launched_as_scripts() {
    let directory = TempDirectory::new("explicit");
    let script = directory.write_script("direct.cmd", "echo direct\n");

    let mut dispatcher = dispatcher(directory.path());
    let mut session = session(directory.path());
    let outcome = dispatcher.execute(&mut session, &format!("\"{}\"", script.display()));

    assert_eq!(text(&outcome), "direct\n", "{outcome:?}");
}

#[test]
fn native_executables_still_launch_without_an_interpreter() {
    let directory = TempDirectory::new("native");
    let interpreter = env::var("COMSPEC").expect("COMSPEC should be set on Windows");
    let outcome = run(
        directory.path(),
        &format!("\"{interpreter}\" /d /c echo direct-native"),
    );

    assert_eq!(text(&outcome), "direct-native\n", "{outcome:?}");
    assert!(outcome.success(), "{outcome:?}");
}
