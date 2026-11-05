//! Integration tests for pipelines, redirections, and logical chaining.
//!
//! The test binary doubles as a portable pipeline utility. When spawned with
//! `CUSTOM_SHELL_PIPELINE_ARGS` the helper test reads standard input and emits
//! marker lines, so the suite never depends on `sort`, `grep`, `findstr`, or
//! any other external program being installed.

use shell_builtins::{Dispatcher, ProcessOptions, StdioMode};
use shell_runtime::Session;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const ARGUMENT_MARKER: &str = "__custom_shell_pipeline_args__";
const HELPER_TEST: &str = "pipeline_helper";

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

/// Portable helper process used by the pipeline tests.
#[test]
fn pipeline_helper() {
    let arguments: Vec<String> = env::args()
        .skip_while(|argument| argument != ARGUMENT_MARKER)
        .skip(1)
        .collect();

    let Some(mode) = arguments.first() else {
        return;
    };

    match mode.as_str() {
        "emit" => {
            emit_line("PIPELINE:alpha");
            emit_line("PIPELINE:beta");
        }
        "upper" => {
            for line in stdin_lines() {
                if let Some(rest) = line.strip_prefix("PIPELINE:") {
                    emit_line(&format!("PIPELINE:{}", rest.to_uppercase()));
                }
            }
        }
        "count" => {
            let count = stdin_lines()
                .iter()
                .filter(|line| line.starts_with("PIPELINE:"))
                .count();
            emit_line(&format!("PIPELINE:count={count}"));
        }
        "fail" => {
            let _ = writeln!(std::io::stderr(), "PIPELINE:failure");
            std::process::exit(9);
        }
        _ => {}
    }
}

/// Writes a line to stdout without panicking when a downstream stage closes.
fn emit_line(line: &str) {
    let _ = writeln!(std::io::stdout(), "{line}");
}

fn stdin_lines() -> Vec<String> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("helper stdin should be readable");
    input.lines().map(str::to_owned).collect()
}

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn new(label: &str) -> Self {
        let unique = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "kernext-pipeline-{label}-{}-{unique}",
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
        // Restore writability so cleanup works after permission tests.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o700));
        }

        let _ = fs::remove_dir_all(&self.path);
    }
}

fn current_executable() -> PathBuf {
    env::current_exe().expect("test executable should be known")
}

fn executable_suffix() -> &'static str {
    if cfg!(windows) { ".exe" } else { "" }
}

fn install_helper(directory: &TempDirectory, name: &str) -> String {
    let name = format!("{name}{}", executable_suffix());
    let target = directory.path().join(&name);

    // Unix uses a symlink to avoid ETXTBSY races with a concurrent fork.
    // Windows keeps a real copy because symlinks need privileges.
    #[cfg(unix)]
    std::os::unix::fs::symlink(current_executable(), &target)
        .expect("helper symlink should be created");
    #[cfg(not(unix))]
    fs::copy(current_executable(), &target).expect("helper should be copied");

    name
}

fn helper_invocation(name: &str, mode: &str) -> String {
    format!("{name} --exact {HELPER_TEST} --nocapture {ARGUMENT_MARKER} {mode}")
}

fn dispatcher(directory: &TempDirectory) -> Dispatcher {
    Dispatcher::with_options(
        ProcessOptions::new()
            .with_stdio(StdioMode::Capture)
            .with_env("PATH", directory.path()),
    )
}

fn session(directory: &TempDirectory) -> Session {
    let mut session = Session::new(Default::default()).expect("session should start");
    session.set_working_directory(directory.path().to_path_buf());
    session
}

fn read(directory: &TempDirectory, name: &str) -> String {
    fs::read_to_string(directory.path().join(name)).expect("output file should be readable")
}

#[test]
fn external_pipeline_connects_stdout_to_stdin() {
    let directory = TempDirectory::new("external");
    let emit = install_helper(&directory, "emit-helper");
    let upper = install_helper(&directory, "upper-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!(
        "{} | {}",
        helper_invocation(&emit, "emit"),
        helper_invocation(&upper, "upper")
    );
    let outcome = dispatcher.execute(&mut session, &command);
    let output = outcome.output().expect("pipeline should produce output");

    assert!(output.contains("PIPELINE:ALPHA"), "{output}");
    assert!(output.contains("PIPELINE:BETA"), "{output}");
    assert!(outcome.success(), "{outcome:?}");
    assert_eq!(outcome.exit_code(), Some(0));
}

#[test]
fn multiple_external_pipeline_stages_are_connected() {
    let directory = TempDirectory::new("multiple");
    let emit = install_helper(&directory, "emit-helper");
    let upper = install_helper(&directory, "upper-helper");
    let count = install_helper(&directory, "count-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!(
        "{} | {} | {}",
        helper_invocation(&emit, "emit"),
        helper_invocation(&upper, "upper"),
        helper_invocation(&count, "count")
    );
    let outcome = dispatcher.execute(&mut session, &command);
    let output = outcome.output().expect("pipeline should produce output");

    assert!(output.contains("PIPELINE:count=2"), "{output}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn built_in_producer_feeds_an_external_stage() {
    let directory = TempDirectory::new("builtin-producer");
    let upper = install_helper(&directory, "upper-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!(
        "echo PIPELINE:hello | {}",
        helper_invocation(&upper, "upper")
    );
    let outcome = dispatcher.execute(&mut session, &command);
    let output = outcome.output().expect("pipeline should produce output");

    assert!(output.contains("PIPELINE:HELLO"), "{output}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn external_producer_feeds_a_built_in_consumer() {
    let directory = TempDirectory::new("builtin-consumer");
    let emit = install_helper(&directory, "emit-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!("{} | cat", helper_invocation(&emit, "emit"));
    let outcome = dispatcher.execute(&mut session, &command);
    let output = outcome.output().expect("pipeline should produce output");

    assert!(output.contains("PIPELINE:alpha"), "{output}");
    assert!(output.contains("PIPELINE:beta"), "{output}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn built_in_producer_and_consumer_exchange_data() {
    let directory = TempDirectory::new("builtin-both");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let outcome = dispatcher.execute(&mut session, "echo PIPELINE:hello | cat");

    assert_eq!(outcome.output(), Some("PIPELINE:hello\n"), "{outcome:?}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn built_in_in_the_middle_of_a_pipeline_is_supported() {
    let directory = TempDirectory::new("builtin-middle");
    let emit = install_helper(&directory, "emit-helper");
    let count = install_helper(&directory, "count-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!(
        "{} | cat | {}",
        helper_invocation(&emit, "emit"),
        helper_invocation(&count, "count")
    );
    let outcome = dispatcher.execute(&mut session, &command);
    let output = outcome.output().expect("pipeline should produce output");

    assert!(output.contains("PIPELINE:count=2"), "{output}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn pipeline_output_can_be_redirected_to_a_file() {
    let directory = TempDirectory::new("pipeline-redirect");
    let emit = install_helper(&directory, "emit-helper");
    let upper = install_helper(&directory, "upper-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!(
        "{} | {} > out.txt",
        helper_invocation(&emit, "emit"),
        helper_invocation(&upper, "upper")
    );
    let outcome = dispatcher.execute(&mut session, &command);

    assert!(outcome.success(), "{outcome:?}");
    assert!(
        !outcome.output().unwrap_or("").contains("PIPELINE:ALPHA"),
        "redirected output must not reach the REPL: {outcome:?}"
    );

    let output = read(&directory, "out.txt");
    assert!(output.contains("PIPELINE:ALPHA"), "{output}");
    assert!(output.contains("PIPELINE:BETA"), "{output}");
}

#[test]
fn output_redirection_truncates_and_append_redirection_preserves() {
    let directory = TempDirectory::new("append");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    dispatcher.execute(&mut session, "echo first > out.txt");
    assert_eq!(read(&directory, "out.txt"), "first\n");

    dispatcher.execute(&mut session, "echo second >> out.txt");
    assert_eq!(read(&directory, "out.txt"), "first\nsecond\n");

    dispatcher.execute(&mut session, "echo third > out.txt");
    assert_eq!(read(&directory, "out.txt"), "third\n");
}

#[test]
fn input_redirection_feeds_a_built_in() {
    let directory = TempDirectory::new("input-builtin");
    fs::write(directory.path().join("input.txt"), "alpha\nbeta\n")
        .expect("input file should be written");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let outcome = dispatcher.execute(&mut session, "cat < input.txt");

    assert_eq!(outcome.output(), Some("alpha\nbeta\n"), "{outcome:?}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn input_redirection_feeds_an_external_stage() {
    let directory = TempDirectory::new("input-external");
    fs::write(
        directory.path().join("input.txt"),
        "PIPELINE:one\nPIPELINE:two\n",
    )
    .expect("input file should be written");
    let upper = install_helper(&directory, "upper-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!("{} < input.txt", helper_invocation(&upper, "upper"));
    let outcome = dispatcher.execute(&mut session, &command);
    let output = outcome.output().expect("pipeline should produce output");

    assert!(output.contains("PIPELINE:ONE"), "{output}");
    assert!(output.contains("PIPELINE:TWO"), "{output}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn pipelines_and_redirections_combine() {
    let directory = TempDirectory::new("combined");
    fs::write(directory.path().join("input.txt"), "PIPELINE:one\n")
        .expect("input file should be written");
    let upper = install_helper(&directory, "upper-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!(
        "cat < input.txt | {} > out.txt",
        helper_invocation(&upper, "upper")
    );
    let outcome = dispatcher.execute(&mut session, &command);

    assert!(outcome.success(), "{outcome:?}");
    let output = read(&directory, "out.txt");
    assert!(output.contains("PIPELINE:ONE"), "{output}");
}

#[test]
fn a_fully_redirected_built_in_does_not_leak_the_upstream_pipe() {
    let directory = TempDirectory::new("redirected-middle");
    fs::write(directory.path().join("input.txt"), "from-file\n")
        .expect("input file should be written");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let outcome = dispatcher.execute(
        &mut session,
        "echo should-not-reach-the-last-stage | cat < input.txt > out.txt | cat",
    );

    assert!(outcome.success(), "{outcome:?}");
    assert_eq!(outcome.output(), None, "{outcome:?}");
    assert_eq!(read(&directory, "out.txt"), "from-file\n");
}

#[test]
fn error_redirection_writes_stderr_to_a_file() {
    let directory = TempDirectory::new("stderr");
    let fail = install_helper(&directory, "fail-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!("{} 2> errors.txt", helper_invocation(&fail, "fail"));
    let outcome = dispatcher.execute(&mut session, &command);

    assert!(!outcome.success(), "{outcome:?}");
    assert_eq!(outcome.exit_code(), Some(9));
    assert!(
        !outcome.output().unwrap_or("").contains("PIPELINE:failure"),
        "redirected stderr must not reach the REPL: {outcome:?}"
    );

    let errors = read(&directory, "errors.txt");
    assert!(errors.contains("PIPELINE:failure"), "{errors}");
}

#[test]
fn error_append_redirection_preserves_previous_content() {
    let directory = TempDirectory::new("stderr-append");
    let fail = install_helper(&directory, "fail-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!("{} 2>> errors.txt", helper_invocation(&fail, "fail"));
    dispatcher.execute(&mut session, &command);
    dispatcher.execute(&mut session, &command);

    let errors = read(&directory, "errors.txt");
    assert_eq!(errors.matches("PIPELINE:failure").count(), 2, "{errors}");
}

#[test]
fn built_in_error_redirection_keeps_streams_separate() {
    let directory = TempDirectory::new("builtin-stderr");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let outcome = dispatcher.execute(&mut session, "cat missing.txt 2> errors.txt");

    assert!(!outcome.success(), "{outcome:?}");
    assert!(
        !outcome.output().unwrap_or("").contains("cat: cannot read"),
        "redirected built-in stderr must not reach the REPL: {outcome:?}"
    );

    let errors = read(&directory, "errors.txt");
    assert!(errors.contains("cat: cannot read"), "{errors}");
}

#[test]
fn missing_input_file_reports_an_error_and_keeps_the_shell_alive() {
    let directory = TempDirectory::new("missing-input");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let outcome = dispatcher.execute(&mut session, "cat < missing.txt");
    let output = outcome.output().expect("error should be reported");

    assert!(!outcome.success(), "{outcome:?}");
    assert!(output.contains("cannot open"), "{output}");
    assert!(output.contains("missing.txt"), "{output}");

    let follow_up = dispatcher.execute(&mut session, "version");
    assert!(follow_up.success(), "{follow_up:?}");
}

#[test]
fn missing_external_in_a_pipeline_is_reported_without_hanging() {
    let directory = TempDirectory::new("pipeline-missing");
    let emit = install_helper(&directory, "emit-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!(
        "{} | definitely-not-a-real-command-xyz",
        helper_invocation(&emit, "emit")
    );
    let outcome = dispatcher.execute(&mut session, &command);
    let output = outcome.output().expect("error should be reported");

    assert!(!outcome.success(), "{outcome:?}");
    assert!(output.contains("Command not found"), "{output}");

    let follow_up = dispatcher.execute(&mut session, "version");
    assert!(follow_up.success(), "{follow_up:?}");
}

#[test]
fn missing_first_stage_in_a_pipeline_is_reported_without_running_later_stages() {
    let directory = TempDirectory::new("pipeline-missing-first");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let outcome = dispatcher.execute(
        &mut session,
        "definitely-not-a-real-command-xyz | echo should-not-run",
    );
    let output = outcome.output().expect("error should be reported");

    assert!(!outcome.success(), "{outcome:?}");
    assert!(output.contains("Command not found"), "{output}");
    assert!(!output.contains("should-not-run"), "{output}");
}

#[test]
fn output_redirection_to_a_missing_directory_reports_an_error() {
    let directory = TempDirectory::new("missing-directory");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let outcome = dispatcher.execute(&mut session, "echo hello > no-such-dir/out.txt");
    let output = outcome.output().expect("error should be reported");

    assert!(!outcome.success(), "{outcome:?}");
    assert!(output.contains("cannot write"), "{output}");

    let follow_up = dispatcher.execute(&mut session, "version");
    assert!(follow_up.success(), "{follow_up:?}");
}

#[test]
fn quoted_redirection_paths_keep_spaces() {
    let directory = TempDirectory::new("quoted-path");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let outcome = dispatcher.execute(&mut session, r#"echo hello > "out file.txt""#);

    assert!(outcome.success(), "{outcome:?}");
    assert_eq!(read(&directory, "out file.txt"), "hello\n");
}

#[cfg(unix)]
#[test]
fn absolute_redirection_paths_are_supported() {
    let directory = TempDirectory::new("absolute-path");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);
    let target = directory.path().join("absolute.txt");

    let command = format!("echo hello > '{}'", target.display());
    let outcome = dispatcher.execute(&mut session, &command);

    assert!(outcome.success(), "{outcome:?}");
    assert_eq!(
        fs::read_to_string(&target).expect("absolute output should exist"),
        "hello\n"
    );
}

#[cfg(unix)]
#[test]
fn permission_denied_output_redirection_is_reported() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDirectory::new("permission");
    let target = directory.path().join("read-only.txt");
    fs::write(&target, "original\n").expect("file should be written");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o400))
        .expect("permissions should be set");

    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);
    let outcome = dispatcher.execute(&mut session, "echo replaced > read-only.txt");

    if outcome.success() {
        // Running with elevated privileges: permission denial is not testable.
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600))
            .expect("permissions should be restored");
        return;
    }

    let output = outcome.output().expect("error should be reported");
    assert!(output.contains("cannot write"), "{output}");

    fs::set_permissions(&target, fs::Permissions::from_mode(0o600))
        .expect("permissions should be restored");
}

#[test]
fn logical_and_runs_the_second_command_only_on_success() {
    let directory = TempDirectory::new("and");
    let fail = install_helper(&directory, "fail-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let success = dispatcher.execute(&mut session, "echo first && echo second");
    assert_eq!(success.output(), Some("first\nsecond\n"), "{success:?}");
    assert!(success.success(), "{success:?}");

    let command = format!("{} && echo second", helper_invocation(&fail, "fail"));
    let failure = dispatcher.execute(&mut session, &command);
    let output = failure.output().expect("error should be reported");

    assert!(!failure.success(), "{failure:?}");
    assert!(!output.contains("second"), "{output}");
}

#[test]
fn logical_or_runs_the_second_command_only_on_failure() {
    let directory = TempDirectory::new("or");
    let fail = install_helper(&directory, "fail-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let recovered = dispatcher.execute(&mut session, "missing || echo recovered");
    let output = recovered.output().expect("output should be produced");

    assert!(output.contains("Command not found"), "{output}");
    assert!(output.contains("recovered"), "{output}");
    assert!(recovered.success(), "{recovered:?}");

    let command = format!("{} || echo second", helper_invocation(&fail, "fail"));
    let failure = dispatcher.execute(&mut session, &command);
    assert!(failure.output().unwrap_or("").contains("second"));

    let skipped = dispatcher.execute(&mut session, "echo first || echo second");
    assert_eq!(skipped.output(), Some("first\n"), "{skipped:?}");
    assert!(skipped.success(), "{skipped:?}");
}

#[test]
fn semicolons_always_continue() {
    let directory = TempDirectory::new("semicolon");
    let fail = install_helper(&directory, "fail-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!("{} ; echo after", helper_invocation(&fail, "fail"));
    let outcome = dispatcher.execute(&mut session, &command);
    let output = outcome.output().expect("output should be produced");

    assert!(output.contains("PIPELINE:failure"), "{output}");
    assert!(output.contains("after"), "{output}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn pipeline_exit_status_drives_logical_chaining() {
    let directory = TempDirectory::new("pipeline-status");
    let emit = install_helper(&directory, "emit-helper");
    let fail = install_helper(&directory, "fail-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!(
        "{} | {}",
        helper_invocation(&emit, "emit"),
        helper_invocation(&fail, "fail")
    );
    let outcome = dispatcher.execute(&mut session, &command);

    assert!(!outcome.success(), "{outcome:?}");
    assert_eq!(outcome.exit_code(), Some(9));

    let chained = format!("{command} || echo recovered");
    let recovered = dispatcher.execute(&mut session, &chained);
    let output = recovered.output().expect("output should be produced");

    assert!(output.contains("recovered"), "{output}");
    assert!(recovered.success(), "{recovered:?}");
    assert_eq!(recovered.exit_code(), None);
}

#[test]
fn built_ins_win_over_external_programs_in_pipelines() {
    let directory = TempDirectory::new("precedence");
    install_helper(&directory, "echo");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let outcome = dispatcher.execute(&mut session, "echo PIPELINE:builtin | cat");

    assert_eq!(outcome.output(), Some("PIPELINE:builtin\n"), "{outcome:?}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn built_in_pipeline_stages_do_not_leak_working_directory_changes() {
    let directory = TempDirectory::new("subshell");
    fs::create_dir(directory.path().join("sub")).expect("subdirectory should be created");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);
    let before = session.working_directory().to_path_buf();

    let outcome = dispatcher.execute(&mut session, "cd sub | cat");

    assert!(outcome.success(), "{outcome:?}");
    assert_eq!(session.working_directory(), before);
}

#[test]
fn quoted_operators_are_ordinary_argument_text() {
    let directory = TempDirectory::new("quoted-operators");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let outcome = dispatcher.execute(&mut session, r#"echo "a | b && c > d""#);

    assert_eq!(outcome.output(), Some("a | b && c > d\n"), "{outcome:?}");
    assert!(outcome.success(), "{outcome:?}");
}

#[test]
fn external_after_a_redirected_producer_receives_empty_input() {
    let directory = TempDirectory::new("redirected-external-input");
    let emit = install_helper(&directory, "emit-helper");
    let count = install_helper(&directory, "count-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!(
        "{} > out.txt | {}",
        helper_invocation(&emit, "emit"),
        helper_invocation(&count, "count")
    );
    let outcome = dispatcher.execute(&mut session, &command);

    assert!(outcome.success(), "{outcome:?}");
    let output = outcome.output().unwrap_or("");
    assert!(
        output.contains("PIPELINE:count=0"),
        "later stage must not read the terminal: {output}"
    );
    assert!(read(&directory, "out.txt").contains("PIPELINE:alpha"));
}

#[test]
fn redirected_external_status_is_still_reported() {
    let directory = TempDirectory::new("redirected-status");
    let emit = install_helper(&directory, "emit-helper");
    let mut dispatcher = dispatcher(&directory);
    let mut session = session(&directory);

    let command = format!("{} > out.txt", helper_invocation(&emit, "emit"));
    let outcome = dispatcher.execute(&mut session, &command);

    assert!(outcome.success(), "{outcome:?}");
    assert_eq!(outcome.exit_code(), Some(0));
    assert!(read(&directory, "out.txt").contains("PIPELINE:alpha"));
}
