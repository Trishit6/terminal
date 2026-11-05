//! Interactive Read-Eval-Print Loop.
//!
//! The REPL depends on abstractions rather than concrete terminal types, which
//! allows scripted integration tests to exercise command handling, interrupts,
//! EOF, and shutdown without a real terminal.

use shell_builtins::{CommandAction, Dispatcher};
use shell_core::ShellError;
use shell_line_editor::{LineEvent, LineReader};
use shell_runtime::Session;
use std::io::Write;

/// Drives input, command dispatch, output, and session shutdown.
pub struct Repl<R, W> {
    session: Session,
    reader: R,
    output: W,
    dispatcher: Dispatcher,
}

impl<R, W> Repl<R, W>
where
    R: LineReader,
    W: Write,
{
    /// Creates a REPL around an initialized session and line reader.
    pub fn new(session: Session, reader: R, output: W) -> Self {
        Self::with_dispatcher(session, reader, output, Dispatcher::new())
    }

    /// Creates a REPL around an already-initialized dispatcher.
    ///
    /// The CLI uses this so the startup checklist can report the command engine
    /// only after it has actually been constructed.
    pub fn with_dispatcher(session: Session, reader: R, output: W, dispatcher: Dispatcher) -> Self {
        Self {
            session,
            reader,
            output,
            dispatcher,
        }
    }

    /// Runs until `exit`, Ctrl+D/EOF, or an unrecoverable terminal error.
    pub fn run(&mut self) -> Result<(), ShellError> {
        while self.session.is_running() {
            match self.reader.read_line() {
                Ok(LineEvent::Line(input)) => self.handle_command(&input)?,
                Ok(LineEvent::Interrupted) => self.write_line("")?,
                Ok(LineEvent::Eof) => {
                    self.session.stop();
                    self.write_output("Goodbye.")?;
                }
                Err(error) => {
                    return Err(ShellError::message(
                        "read terminal input",
                        error.to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Session being driven by this REPL.
    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Line reader used by this REPL.
    #[must_use]
    pub fn reader(&self) -> &R {
        &self.reader
    }

    fn handle_command(&mut self, input: &str) -> Result<(), ShellError> {
        let outcome = self.dispatcher.execute(&mut self.session, input);

        match outcome.action() {
            CommandAction::Continue => {
                if let Some(output) = outcome.output() {
                    self.write_output(output)?;
                }
            }
            CommandAction::ClearScreen => {
                self.reader.clear_screen().map_err(|error| {
                    ShellError::message("clear the terminal", error.to_string())
                })?;

                if let Some(output) = outcome.output() {
                    self.write_output(output)?;
                }
            }
            CommandAction::ClearHistory => {
                self.dispatcher.clear_history();
                if let Some(output) = outcome.output() {
                    self.write_output(output)?;
                }
            }
            CommandAction::Exit => {
                if let Some(output) = outcome.output() {
                    self.write_output(output)?;
                }
                self.session.stop();
            }
        }

        Ok(())
    }

    /// Writes command output exactly, adding a final newline only when the
    /// output does not already end with one.
    fn write_output(&mut self, text: &str) -> Result<(), ShellError> {
        self.output
            .write_all(text.as_bytes())
            .and_then(|()| {
                if text.ends_with('\n') {
                    Ok(())
                } else {
                    writeln!(self.output)
                }
            })
            .map_err(|source| ShellError::io("write shell output", source))
    }

    fn write_line(&mut self, text: &str) -> Result<(), ShellError> {
        writeln!(self.output, "{text}")
            .map_err(|source| ShellError::io("write shell output", source))
    }
}
