//! Reedline-backed interactive line editor.
//!
//! The rest of the shell depends only on [`LineReader`] and [`LineEvent`], so
//! command dispatch and REPL behavior can be tested without a terminal.

use crate::prompt::ShellPrompt;
use crossterm::cursor::MoveTo;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use reedline::{FileBackedHistory, Reedline, Signal};
use std::error::Error;
use std::fmt;
use std::io::{self, Write};

/// A single result from reading interactive input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineEvent {
    /// The user submitted a line.
    Line(String),
    /// The user pressed Ctrl+C while editing.
    Interrupted,
    /// The user requested end-of-file with Ctrl+D.
    Eof,
}

/// Interactive line input used by the REPL.
pub trait LineReader {
    /// Reads one line or interruption event.
    fn read_line(&mut self) -> Result<LineEvent, LineEditorError>;

    /// Clears the visible terminal and moves the cursor to the origin.
    fn clear_screen(&mut self) -> Result<(), LineEditorError>;
}

/// Reedline-backed implementation of [`LineReader`].
pub struct ShellEditor {
    editor: Reedline,
    prompt: ShellPrompt,
}

impl ShellEditor {
    /// Creates an editor with in-memory command history.
    ///
    /// Arrow-key history navigation, backspace, delete, and standard Emacs
    /// keybindings are provided by Reedline's defaults.
    pub fn new(
        prompt: impl Into<String>,
        history_size: usize,
        ansi_colors: bool,
    ) -> Result<Self, LineEditorError> {
        let history = FileBackedHistory::new(history_size).map_err(LineEditorError::History)?;

        Ok(Self {
            editor: Reedline::create()
                .with_history(Box::new(history))
                .with_ansi_colors(ansi_colors),
            prompt: ShellPrompt::new(prompt),
        })
    }
}

impl LineReader for ShellEditor {
    fn read_line(&mut self) -> Result<LineEvent, LineEditorError> {
        let signal = self
            .editor
            .read_line(&self.prompt)
            .map_err(LineEditorError::Read)?;

        Ok(match signal {
            Signal::Success(line) | Signal::HostCommand(line) => LineEvent::Line(line),
            Signal::CtrlC | Signal::ExternalBreak(_) => LineEvent::Interrupted,
            Signal::CtrlD => LineEvent::Eof,
            _ => LineEvent::Interrupted,
        })
    }

    fn clear_screen(&mut self) -> Result<(), LineEditorError> {
        let mut stdout = io::stdout();

        execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))
            .and_then(|()| stdout.flush())
            .map_err(LineEditorError::Clear)
    }
}

/// Errors produced by the interactive line editor.
#[derive(Debug)]
pub enum LineEditorError {
    /// History initialization failed.
    History(reedline::ReedlineError),
    /// Terminal input could not be read.
    Read(io::Error),
    /// The terminal could not be cleared.
    Clear(io::Error),
}

impl fmt::Display for LineEditorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::History(source) => {
                write!(formatter, "could not initialize command history: {source}")
            }
            Self::Read(source) => write!(formatter, "could not read terminal input: {source}"),
            Self::Clear(source) => write!(formatter, "could not clear the terminal: {source}"),
        }
    }
}

impl Error for LineEditorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::History(source) => Some(source),
            Self::Read(source) => Some(source),
            Self::Clear(source) => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_can_be_created_with_configured_history() {
        let editor = ShellEditor::new("test > ", 25, false);

        assert!(editor.is_ok());
    }
}
