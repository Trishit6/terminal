//! Windows ConPTY integration coverage.
//!
//! These tests exercise the public pseudoterminal surface with real Windows
//! programs. They are intentionally separate from the unit tests so a failure
//! points at the ConPTY integration rather than the module's pure helpers.

#![cfg(windows)]

use shell_builtins::{ExternalCommand, PtyProcess, PtySize};
use std::io::Read;
use std::thread;

fn spawn_and_collect(command: &ExternalCommand) -> (Option<i32>, String) {
    let mut process =
        PtyProcess::spawn(command, PtySize::DEFAULT).expect("pseudoterminal should start");

    let reader = process.take_reader().expect("reader should be available");
    let output = thread::spawn(move || {
        let mut reader = reader;
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes);
        bytes
    });

    let status = process.wait().expect("child should be waited for");
    drop(process);

    let bytes = output.join().expect("reader thread should finish");
    (status.code(), String::from_utf8_lossy(&bytes).into_owned())
}

#[test]
fn cmd_exit_code_survives_the_pseudoterminal() {
    let command = ExternalCommand::new("cmd").args(["/d", "/c", "exit", "42"]);

    let (code, output) = spawn_and_collect(&command);

    assert_eq!(code, Some(42), "unexpected output: {output:?}");
}

#[test]
fn powershell_exit_code_survives_the_pseudoterminal() {
    let command = ExternalCommand::new("powershell").args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "exit 42",
    ]);

    let (code, output) = spawn_and_collect(&command);

    assert_eq!(code, Some(42), "unexpected output: {output:?}");
}

#[test]
fn cmd_stdout_is_captured_through_the_pseudoterminal() {
    let command = ExternalCommand::new("cmd").args(["/d", "/c", "echo", "kernext-pty-output"]);

    let (code, output) = spawn_and_collect(&command);

    assert_eq!(code, Some(0), "unexpected output: {output:?}");
    assert!(
        output.contains("kernext-pty-output"),
        "missing output: {output:?}"
    );
}

#[test]
fn a_batch_script_receives_input_through_the_pseudoterminal() {
    let directory = std::env::temp_dir().join(format!(
        "kernext-pty-batch-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    std::fs::create_dir_all(&directory).expect("temporary directory should be created");

    let script = directory.join("read.cmd");
    std::fs::write(&script, "@echo off\r\nset /p line=\r\necho got=%line%\r\n")
        .expect("script should be written");

    let command = ExternalCommand::new(script.to_string_lossy().into_owned());

    let mut process =
        PtyProcess::spawn(&command, PtySize::DEFAULT).expect("pseudoterminal should start");
    let mut writer = process.take_writer().expect("writer should be available");
    writer
        .write_all(b"typed-line\r\n")
        .expect("input should be accepted");
    writer.flush().expect("input should be flushed");

    let reader = process.take_reader().expect("reader should be available");
    let output = thread::spawn(move || {
        let mut reader = reader;
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes);
        bytes
    });

    let status = process.wait().expect("child should be waited for");
    drop(process);
    drop(writer);

    let bytes = output.join().expect("reader thread should finish");
    let text = String::from_utf8_lossy(&bytes);
    let _ = std::fs::remove_dir_all(&directory);

    assert!(status.success(), "{status:?}: {text}");
    assert!(text.contains("got=typed-line"), "missing output: {text:?}");
}

fn unique_suffix() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    NEXT.fetch_add(1, Ordering::Relaxed)
}
