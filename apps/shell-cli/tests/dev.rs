//! Integration tests for the KERNEXT project commands.
//!
//! These run the real `kernext` binary, so they cover argument parsing, exit
//! codes, and the wiring between the CLI and the developer module. Commands
//! that would run a full build use `--dry-run` or a deliberately missing
//! toolchain so the test suite stays fast.

use std::path::PathBuf;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_kernext");
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn kernext(arguments: &[&str]) -> Output {
    Command::new(BIN)
        .args(arguments)
        .output()
        .expect("the kernext binary should run")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn version_reports_the_product_and_the_workspace_version() {
    let output = kernext(&["version"]);
    let text = stdout(&output);

    assert!(
        output.status.success(),
        "version should succeed: {}",
        stderr(&output)
    );
    assert!(text.contains("KERNEXT"), "missing product name: {text}");
    assert!(
        text.contains(&format!("v{VERSION}")),
        "missing the compiled version {VERSION}: {text}"
    );
    assert!(
        text.contains("The Next Core of Computing."),
        "missing the tagline: {text}"
    );
    assert!(
        text.contains("Independent. Native. Engineered."),
        "missing the brand statement: {text}"
    );
}

#[test]
fn help_lists_every_project_command() {
    let output = kernext(&["help"]);
    let text = stdout(&output);

    assert!(output.status.success());
    for command in [
        "build", "check", "test", "fmt", "lint", "run", "clean", "doctor", "version", "help",
    ] {
        assert!(
            text.contains(&format!("kernext {command}")),
            "help should mention `kernext {command}`: {text}"
        );
    }
    assert!(
        text.contains("Cargo"),
        "help should say that Cargo is the underlying build tool"
    );
}

#[test]
fn doctor_reports_every_section_and_never_prints_secrets() {
    let output = kernext(&["doctor"]);
    let text = stdout(&output);

    // Doctor exits non-zero when it finds something actionable (a missing
    // toolchain or front-end dependencies, for example), which is expected on
    // machines that only build part of the project.
    assert!(
        matches!(output.status.code(), Some(0) | Some(1)),
        "unexpected doctor exit code: {:?}",
        output.status.code()
    );

    for section in [
        "Environment",
        "Rust tooling",
        "Frontend tooling",
        "Project",
        "Git",
        "Build environment",
        "Platform capabilities",
    ] {
        assert!(
            text.contains(section),
            "doctor should report `{section}`: {text}"
        );
    }

    // Statuses are rendered per finding; `Available` is guaranteed because
    // KERNEXT always reports its own version and platform.
    assert!(
        text.contains("Available"),
        "doctor should render a status column: {text}"
    );
    assert!(
        text.contains("Suggestions")
            || text.contains("No problems detected.")
            || text.contains("need attention"),
        "doctor should summarize its findings: {text}"
    );

    for secret in ["TOKEN", "SECRET", "PASSWORD", "API_KEY"] {
        assert!(
            !text.contains(secret),
            "doctor output should not expose `{secret}`"
        );
    }
}

#[test]
fn dry_run_prints_the_cargo_invocation_without_running_it() {
    let output = kernext(&["build", "--release", "--dry-run"]);
    let text = stdout(&output);

    assert!(
        output.status.success(),
        "dry run should succeed: {}",
        stderr(&output)
    );
    assert!(
        text.contains("build --release --workspace"),
        "dry run should show the Cargo arguments: {text}"
    );
    assert!(
        text.contains("dry run"),
        "dry run should say that nothing was executed: {text}"
    );
}

#[test]
fn check_dry_run_includes_all_targets() {
    let output = kernext(&["check", "--dry-run"]);
    let text = stdout(&output);

    assert!(output.status.success());
    assert!(
        text.contains("check --workspace --all-targets"),
        "check should type-check every target: {text}"
    );
}

#[test]
fn a_missing_cargo_reports_a_clear_failure_and_a_127_exit_code() {
    let missing = PathBuf::from("kernext-missing-cargo-does-not-exist");
    let output = Command::new(BIN)
        .args(["build"])
        .env("CARGO", &missing)
        .output()
        .expect("the kernext binary should run");

    assert_eq!(output.status.code(), Some(127));
    assert!(
        stderr(&output).contains("rustup"),
        "the failure should suggest installing Rust: {}",
        stderr(&output)
    );
}

#[test]
fn an_invalid_project_root_is_rejected_instead_of_guessed() {
    let directory =
        std::env::temp_dir().join(format!("kernext-not-a-workspace-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("temporary directory");

    let output = Command::new(BIN)
        .args(["build"])
        .env("KERNEXT_PROJECT_ROOT", &directory)
        .output()
        .expect("the kernext binary should run");

    std::fs::remove_dir_all(&directory).ok();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("KERNEXT_PROJECT_ROOT"),
        "the error should name the override: {}",
        stderr(&output)
    );
}
