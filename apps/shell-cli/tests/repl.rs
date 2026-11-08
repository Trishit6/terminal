//! End-to-end REPL tests using a scripted line reader.

use shell_cli::repl::Repl;
use shell_config::Config;
use shell_core::VERSION;
use shell_line_editor::{LineEditorError, LineEvent, LineReader};
use shell_runtime::Session;
use std::collections::VecDeque;

#[derive(Debug)]
struct ScriptedReader {
    events: VecDeque<LineEvent>,
    reads: usize,
    clears: usize,
    history_clears: usize,
}

impl ScriptedReader {
    fn new(events: impl IntoIterator<Item = LineEvent>) -> Self {
        Self {
            events: events.into_iter().collect(),
            reads: 0,
            clears: 0,
            history_clears: 0,
        }
    }
}

impl LineReader for ScriptedReader {
    fn read_line(&mut self) -> Result<LineEvent, LineEditorError> {
        self.reads += 1;
        Ok(self.events.pop_front().unwrap_or(LineEvent::Eof))
    }

    fn clear_screen(&mut self) -> Result<(), LineEditorError> {
        self.clears += 1;
        Ok(())
    }

    fn clear_history(&mut self) -> Result<(), LineEditorError> {
        self.history_clears += 1;
        Ok(())
    }
}

fn session() -> Session {
    Session::new(Config::default()).expect("session should start")
}

fn line(text: &str) -> LineEvent {
    LineEvent::Line(text.to_owned())
}

#[test]
fn repl_handles_help_version_unknown_and_exit() {
    let reader = ScriptedReader::new([
        line("help"),
        line("version"),
        line("unknowncommand"),
        line("exit"),
    ]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert!(repl.session().is_stopped());
    assert_eq!(repl.reader().reads, 4);

    drop(repl);
    let output = String::from_utf8(output).expect("output should be UTF-8");

    assert!(output.contains("Available commands:"));
    assert!(output.contains("help"));
    assert!(output.contains(&format!("KERNEXT v{VERSION}")));
    assert!(output.contains("Command not found: unknowncommand"));
    assert!(output.contains("Goodbye."));
}

#[test]
fn exit_stops_the_repl_without_reading_another_line() {
    let reader = ScriptedReader::new([line("exit"), line("help")]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert!(repl.session().is_stopped());
    assert_eq!(repl.reader().reads, 1);
    assert_eq!(repl.exit_status(), Some(0));

    drop(repl);
    let output = String::from_utf8(output).expect("output should be UTF-8");

    assert!(output.contains("Goodbye."));
    assert!(!output.contains("Available commands:"));
}

#[test]
fn exit_preserves_the_requested_process_status() {
    let reader = ScriptedReader::new([line("exit 7")]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert_eq!(repl.exit_status(), Some(7));
    assert_eq!(repl.session().last_status(), 7);
    assert!(repl.session().is_stopped());
}

#[test]
fn eof_stops_the_repl_cleanly() {
    let reader = ScriptedReader::new([LineEvent::Eof]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert!(repl.session().is_stopped());
    assert_eq!(repl.reader().reads, 1);

    drop(repl);
    let output = String::from_utf8(output).expect("output should be UTF-8");

    assert_eq!(output, "Goodbye.\n");
}

#[test]
fn ctrl_c_cancels_the_current_line_and_continues() {
    let reader = ScriptedReader::new([LineEvent::Interrupted, line("version"), line("exit")]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert!(repl.session().is_stopped());
    assert_eq!(repl.reader().reads, 3);

    drop(repl);
    let output = String::from_utf8(output).expect("output should be UTF-8");

    assert!(output.starts_with('\n'));
    assert!(output.contains(&format!("KERNEXT v{VERSION}")));
    assert!(output.contains("Goodbye."));
}

#[test]
fn empty_and_whitespace_lines_produce_no_command_output() {
    let reader = ScriptedReader::new([line(""), line("   "), line("\t"), line("exit")]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    drop(repl);
    let output = String::from_utf8(output).expect("output should be UTF-8");

    assert_eq!(output, "Goodbye.\n");
}

#[test]
fn repl_continues_after_a_failed_external_command() {
    let reader = ScriptedReader::new([
        line("definitely-not-a-real-command-xyz"),
        line("version"),
        line("exit"),
    ]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert!(repl.session().is_stopped());
    assert_eq!(repl.reader().reads, 3);

    drop(repl);
    let output = String::from_utf8(output).expect("output should be UTF-8");

    assert!(output.contains("Command not found: definitely-not-a-real-command-xyz"));
    assert!(output.contains(&format!("KERNEXT v{VERSION}")));
    assert!(output.contains("Goodbye."));
}

#[test]
fn repl_continues_after_a_failed_pipeline() {
    let reader = ScriptedReader::new([
        line("definitely-not-a-real-command-xyz | echo nope"),
        line("version"),
        line("exit"),
    ]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert!(repl.session().is_stopped());
    assert_eq!(repl.reader().reads, 3);

    drop(repl);
    let output = String::from_utf8(output).expect("output should be UTF-8");

    assert!(output.contains("Command not found: definitely-not-a-real-command-xyz"));
    assert!(output.contains(&format!("KERNEXT v{VERSION}")));
    assert!(output.contains("Goodbye."));
}

#[test]
fn clear_requests_a_terminal_clear_and_continues() {
    let reader = ScriptedReader::new([line("clear"), line("exit")]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert_eq!(repl.reader().clears, 1);
    assert!(repl.session().is_stopped());
}

#[test]
fn clear_can_run_repeatedly() {
    let reader = ScriptedReader::new([line("clear"), line("clear"), line("clear"), line("exit")]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert_eq!(repl.reader().clears, 3);
    assert!(repl.session().is_stopped());
}

#[test]
fn clear_after_command_output_still_requests_a_clear() {
    let reader = ScriptedReader::new([line("version"), line("clear"), line("exit")]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert_eq!(repl.reader().clears, 1);
    assert!(repl.session().is_stopped());

    drop(repl);
    let output = String::from_utf8(output).expect("output should be UTF-8");
    assert!(output.contains(&format!("KERNEXT v{VERSION}")));
}

#[test]
fn history_clear_clears_the_dispatcher_and_line_editor_history() {
    let reader = ScriptedReader::new([line("history clear"), line("exit")]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert_eq!(repl.reader().history_clears, 1);
    assert!(repl.session().is_stopped());

    drop(repl);
    let output = String::from_utf8(output).expect("output should be UTF-8");
    assert!(output.contains("History cleared."));
}

#[test]
fn clear_in_a_pipeline_does_not_clear_the_parent_terminal() {
    let reader = ScriptedReader::new([line("clear | cat"), line("exit")]);
    let mut output = Vec::new();
    let mut repl = Repl::new(session(), reader, &mut output);

    repl.run().expect("REPL should shut down cleanly");

    assert_eq!(repl.reader().clears, 0);
    assert!(repl.session().is_stopped());
}
