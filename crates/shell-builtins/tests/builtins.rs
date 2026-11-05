//! Phase 5 integration tests for native built-in commands.
//!
//! Every filesystem test uses its own temporary directory and changes only the
//! session working directory, so tests remain isolated even when run in
//! parallel.

use shell_builtins::{CommandAction, CommandOutcome, Dispatcher};
use shell_runtime::Session;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "custom-shell-builtins-{}-{unique}-{sequence}",
            std::process::id()
        ));

        fs::create_dir_all(&path).expect("temporary directory should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct Harness {
    directory: TempDir,
    dispatcher: Dispatcher,
    session: Session,
}

impl Harness {
    fn new() -> Self {
        let directory = TempDir::new();
        let mut session = Session::new(Default::default()).expect("session should start");
        session.set_working_directory(directory.path().to_path_buf());

        Self {
            directory,
            dispatcher: Dispatcher::new(),
            session,
        }
    }

    fn run(&mut self, input: &str) -> CommandOutcome {
        self.dispatcher.execute(&mut self.session, input)
    }

    fn output(&mut self, input: &str) -> String {
        self.run(input).output().unwrap_or("").to_owned()
    }

    fn path(&self) -> &Path {
        self.directory.path()
    }
}

#[test]
fn pwd_reports_the_current_working_directory() {
    let mut harness = Harness::new();

    let output = harness.output("pwd");

    assert_eq!(output, harness.path().display().to_string());
}

#[test]
fn cd_changes_directory_and_relative_parent_navigation_works() {
    let mut harness = Harness::new();
    fs::create_dir(harness.path().join("project")).expect("project directory should be created");

    let outcome = harness.run("cd project");
    assert_eq!(outcome.action(), CommandAction::Continue);
    assert_eq!(outcome.output(), None);
    assert_eq!(
        harness.session.working_directory().file_name(),
        Some(std::ffi::OsStr::new("project"))
    );

    let output = harness.output("pwd");
    assert!(output.ends_with("project"));

    harness.run("cd ..");
    assert_eq!(
        harness.session.working_directory().file_name(),
        harness.path().file_name()
    );
}

#[test]
fn cd_accepts_an_absolute_path() {
    let mut harness = Harness::new();
    let target = harness.path().join("absolute-target");
    fs::create_dir(&target).expect("absolute target should be created");

    harness.run(&format!("cd \"{}\"", target.display()));

    assert!(
        harness
            .session
            .working_directory()
            .ends_with("absolute-target")
    );
    assert!(harness.session.working_directory().is_dir());
}

#[test]
fn cd_reports_missing_and_non_directory_targets() {
    let mut harness = Harness::new();
    fs::write(harness.path().join("plain-file.txt"), "data").expect("file should be created");

    let missing = harness.output("cd missing-directory");
    assert!(missing.contains("no such file or directory"), "{missing}");

    let file = harness.output("cd plain-file.txt");
    assert!(file.contains("not a directory"), "{file}");
}

#[test]
fn cd_without_arguments_uses_the_home_directory() {
    let mut harness = Harness::new();

    harness.run("cd");

    assert!(harness.session.working_directory().is_dir());
    assert_ne!(harness.session.working_directory(), harness.path());
}

#[test]
fn mkdir_touch_and_cat_work_together() {
    let mut harness = Harness::new();

    harness.run("mkdir docs");
    harness.run("touch \"docs/note.txt\"");

    let note = harness.path().join("docs").join("note.txt");
    assert!(note.is_file());

    let empty = harness.run("cat \"docs/note.txt\"");
    assert_eq!(empty.output(), None);

    fs::write(&note, "hello from the shell\n").expect("note should be updated");
    let content = harness.output("cat \"docs/note.txt\"");
    assert_eq!(content, "hello from the shell\n");

    harness.run("touch \"docs/note.txt\"");
    assert_eq!(
        fs::read_to_string(&note).expect("note should remain readable"),
        "hello from the shell\n"
    );
}

#[test]
fn mkdir_p_creates_nested_directories() {
    let mut harness = Harness::new();

    let outcome = harness.run("mkdir -p nested/deep/tree");

    assert_eq!(outcome.output(), None);
    assert!(harness.path().join("nested/deep/tree").is_dir());
}

#[test]
fn echo_preserves_quoted_spaces_and_prints_a_blank_line() {
    let mut harness = Harness::new();

    assert_eq!(harness.output("echo hello world"), "hello world\n");
    assert_eq!(
        harness.output("echo \"hello   spaced   world\""),
        "hello   spaced   world\n"
    );
    assert_eq!(harness.output("echo"), "\n");
}

#[test]
fn paths_containing_spaces_work_for_directory_and_file_commands() {
    let mut harness = Harness::new();

    harness.run("mkdir \"space dir\"");
    harness.run("cd \"space dir\"");
    harness.run("touch \"file with spaces.txt\"");
    fs::write(
        harness
            .session
            .working_directory()
            .join("file with spaces.txt"),
        "spaced content\n",
    )
    .expect("spaced file should be writable");

    let output = harness.output("cat \"file with spaces.txt\"");
    let listing = harness.output("ls");

    assert_eq!(output, "spaced content\n");
    assert!(listing.contains("file with spaces.txt"), "{listing}");
    assert_eq!(
        harness.session.working_directory().file_name(),
        Some(std::ffi::OsStr::new("space dir"))
    );
}

#[test]
fn ls_lists_files_and_marks_directories() {
    let mut harness = Harness::new();
    fs::write(harness.path().join("alpha.txt"), "a").expect("file should be created");
    fs::write(harness.path().join("beta.txt"), "b").expect("file should be created");
    fs::create_dir(harness.path().join("child")).expect("directory should be created");

    let output = harness.output("ls");

    assert!(output.contains("alpha.txt"), "{output}");
    assert!(output.contains("beta.txt"), "{output}");
    assert!(
        output.contains(&format!("child{}", std::path::MAIN_SEPARATOR)),
        "{output}"
    );
}

#[test]
fn invalid_commands_are_reported_without_stopping_the_shell() {
    let mut harness = Harness::new();

    let outcome = harness.run("hi");

    assert_eq!(outcome.action(), CommandAction::Continue);
    assert_eq!(outcome.output(), Some("Command not found: hi"));
}

#[test]
fn cp_mv_and_rm_operate_on_files() {
    let mut harness = Harness::new();
    fs::write(harness.path().join("source.txt"), "payload").expect("source should be created");

    harness.run("cp source.txt copy.txt");
    assert_eq!(
        fs::read_to_string(harness.path().join("copy.txt")).expect("copy should exist"),
        "payload"
    );

    harness.run("mv copy.txt renamed.txt");
    assert!(!harness.path().join("copy.txt").exists());
    assert_eq!(
        fs::read_to_string(harness.path().join("renamed.txt")).expect("renamed should exist"),
        "payload"
    );

    harness.run("rm renamed.txt");
    assert!(!harness.path().join("renamed.txt").exists());
}

#[test]
fn cp_r_and_rm_r_handle_directories() {
    let mut harness = Harness::new();
    fs::create_dir(harness.path().join("source-dir")).expect("source directory should be created");
    fs::write(harness.path().join("source-dir").join("item.txt"), "item")
        .expect("item should be created");

    harness.run("cp -r source-dir copied-dir");
    assert_eq!(
        fs::read_to_string(harness.path().join("copied-dir/item.txt")).expect("copied item"),
        "item"
    );

    harness.run("rm -r copied-dir");
    assert!(!harness.path().join("copied-dir").exists());
}

#[test]
fn rm_without_recursive_refuses_directories() {
    let mut harness = Harness::new();
    fs::create_dir(harness.path().join("keep-me")).expect("directory should be created");

    let output = harness.output("rm keep-me");

    assert!(output.contains("is a directory"), "{output}");
    assert!(harness.path().join("keep-me").is_dir());
}

#[test]
fn cat_concatenates_multiple_files() {
    let mut harness = Harness::new();
    fs::write(harness.path().join("one.txt"), "one\n").expect("first file should be created");
    fs::write(harness.path().join("two.txt"), "two\n").expect("second file should be created");

    let output = harness.output("cat one.txt two.txt");

    assert_eq!(output, "one\ntwo\n");
}

#[test]
fn history_lists_commands_oldest_first() {
    let mut harness = Harness::new();

    harness.run("pwd");
    harness.run("echo recorded");
    let output = harness.output("history");

    assert!(output.contains("1  pwd"), "{output}");
    assert!(output.contains("2  echo recorded"), "{output}");
    assert!(output.contains("3  history"), "{output}");
}

#[test]
fn whoami_date_and_version_produce_output() {
    let mut harness = Harness::new();

    assert!(!harness.output("whoami").trim().is_empty());
    assert!(!harness.output("date").trim().is_empty());
    assert!(harness.output("version").starts_with("Custom Shell v"));
}

#[test]
fn clear_requests_a_screen_clear() {
    let mut harness = Harness::new();

    let outcome = harness.run("clear");

    assert_eq!(outcome.action(), CommandAction::ClearScreen);
    assert_eq!(outcome.output(), None);
}

#[test]
fn exit_requests_shutdown_and_prints_a_farewell() {
    let mut harness = Harness::new();

    let outcome = harness.run("exit");

    assert_eq!(outcome.action(), CommandAction::Exit);
    assert_eq!(outcome.output(), Some("Goodbye."));
}

#[test]
fn semicolon_lists_and_logical_operators_run_through_the_ast() {
    let mut harness = Harness::new();

    assert_eq!(harness.output("echo one; echo two"), "one\ntwo\n");
    assert_eq!(harness.output("echo left && echo right"), "left\nright\n");
    assert_eq!(
        harness.output("missing || echo recovered"),
        "Command not found: missing\nrecovered\n"
    );
}

#[test]
fn exit_stops_later_commands_in_the_same_list() {
    let mut harness = Harness::new();

    let outcome = harness.run("echo before; exit; echo after");

    assert_eq!(outcome.action(), CommandAction::Exit);
    assert_eq!(outcome.output(), Some("before\nGoodbye."));
}
