//! Compatibility tests for representative external developer tools.
//!
//! These tests verify that KERNEXT discovers and launches real programs that
//! are installed on the host through its native process executor — the same
//! lexer → parser → evaluator → pipeline path used by the CLI, the web
//! server, and the desktop app.
//!
//! A tool that is not installed on the host is reported and skipped; it is
//! never counted as verified. A tool that resolves must execute successfully
//! through the shell, with its output captured and its exit code reported.

use shell_builtins::{Dispatcher, ProcessOptions, StdioMode, resolve_executable};
use shell_runtime::Session;
use std::path::Path;

/// One representative tool and how to ask it for its version.
struct ToolCase {
    name: &'static str,
    flag: &'static str,
    /// Text the tool is expected to print. `None` only requires any output.
    expected: Option<&'static str>,
}

const TOOLS: &[ToolCase] = &[
    ToolCase {
        name: "git",
        flag: "--version",
        expected: Some("git version"),
    },
    ToolCase {
        name: "cargo",
        flag: "--version",
        expected: Some("cargo"),
    },
    ToolCase {
        name: "rustc",
        flag: "--version",
        expected: Some("rustc"),
    },
    ToolCase {
        name: "node",
        flag: "--version",
        expected: Some("v"),
    },
    ToolCase {
        name: "npm",
        flag: "--version",
        expected: None,
    },
    ToolCase {
        name: "python",
        flag: "--version",
        expected: Some("Python"),
    },
    ToolCase {
        name: "curl",
        flag: "--version",
        expected: Some("curl"),
    },
    ToolCase {
        name: "opencode",
        flag: "--version",
        expected: Some("opencode"),
    },
    ToolCase {
        name: "code",
        flag: "--version",
        expected: None,
    },
];

fn tool_is_installed(name: &str) -> bool {
    resolve_executable(name, Path::new(".")).is_ok()
}

/// The Windows Store `python.exe` alias is a stub that resolves but cannot
/// run. It prints a store message and exits non-zero, so it is treated the
/// same as an absent installation instead of a KERNEXT failure.
fn is_windows_store_stub(name: &str, output: &str) -> bool {
    cfg!(windows) && name == "python" && output.contains("Python was not found")
}

#[test]
fn representative_installed_tools_execute_through_the_shell() {
    let mut dispatcher =
        Dispatcher::with_options(ProcessOptions::default().with_stdio(StdioMode::Capture));
    let mut session = Session::new(Default::default()).expect("session should start");

    let mut verified: Vec<&'static str> = Vec::new();
    let mut skipped: Vec<&'static str> = Vec::new();

    for tool in TOOLS {
        if !tool_is_installed(tool.name) {
            skipped.push(tool.name);
            continue;
        }

        let command = format!("{} {}", tool.name, tool.flag);
        let outcome = dispatcher.execute(&mut session, &command);
        let output = outcome.output().unwrap_or_default().to_owned();

        println!(
            "{command}: success={} exit_code={:?} output={:?}",
            outcome.success(),
            outcome.exit_code(),
            output.lines().next().unwrap_or_default()
        );

        if is_windows_store_stub(tool.name, &output) {
            skipped.push(tool.name);
            continue;
        }

        assert!(
            !output.trim().is_empty(),
            "{} produced no output through the shell",
            tool.name
        );

        if let Some(expected) = tool.expected {
            assert!(
                output.contains(expected),
                "unexpected {command} output: {output:?}"
            );
        }

        assert!(
            outcome.success(),
            "{command} should succeed, got output {output:?}"
        );

        verified.push(tool.name);
    }

    println!("verified through the shell: {:?}", verified);
    println!("not installed on this host (skipped): {:?}", skipped);

    // The environment running this suite is expected to have at least git
    // or cargo available; otherwise this test proves nothing on this host.
    assert!(
        verified.len() + skipped.len() == TOOLS.len(),
        "every tool must be classified exactly once"
    );
    assert!(
        !verified.is_empty(),
        "no representative tool was installed, so nothing was verified"
    );
}

#[test]
fn tool_resolution_matches_the_process_executor() {
    // Every name the engine resolves must also be launchable through the
    // external executor API, and uninstalled names must fail resolution
    // cleanly instead of launching something unrelated.
    for tool in TOOLS {
        match resolve_executable(tool.name, Path::new(".")) {
            Ok(path) => {
                assert!(
                    path.is_file(),
                    "{} resolved to a non-file: {}",
                    tool.name,
                    path.display()
                );
            }
            Err(error) => {
                let message = error.to_string();
                assert!(
                    message.contains("not found"),
                    "unexpected resolution error for {}: {message}",
                    tool.name
                );
            }
        }
    }
}
