//! Reedline-backed interactive line editor.
//!
//! The rest of the shell depends only on [`LineReader`] and [`LineEvent`], so
//! command dispatch and REPL behavior can be tested without a terminal.

use crate::prompt::ShellPrompt;
use crossterm::cursor::MoveTo;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use reedline::{Completer, FileBackedHistory, Reedline, Signal, Span, Suggestion};
use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

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

/// A completer that provides context-aware tab completion for the shell.
#[derive(Clone)]
pub struct ShellCompleter {
    builtins: Vec<String>,
    session: Option<Arc<dyn CompletionContext>>,
}

impl ShellCompleter {
    pub fn new(builtins: Vec<String>) -> Self {
        Self {
            builtins,
            session: None,
        }
    }

    pub fn with_session(mut self, session: Arc<dyn CompletionContext>) -> Self {
        self.session = Some(session);
        self
    }
}

/// Trait for providing completion context (executables, variables, etc.)
pub trait CompletionContext: Send + Sync {
    /// Get all built-in command names
    fn builtins(&self) -> &[String];

    /// Get external executable names from PATH
    fn executables(&self) -> Vec<String>;

    /// Get variable names for completion
    fn variables(&self) -> Vec<String>;

    /// Get exported variable names for completion
    fn exported_variables(&self) -> Vec<String>;

    /// Get current working directory
    fn working_directory(&self) -> PathBuf;
}

impl Completer for ShellCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        // Determine what we're completing based on the cursor position
        let (completion_type, prefix) = self.analyze_input(line, pos);

        match completion_type {
            CompletionType::Command => self.complete_command(&prefix),
            CompletionType::Path => self.complete_path(&prefix),
            CompletionType::Variable => self.complete_variable(&prefix),
            CompletionType::ExportUnset => self.complete_export_unset(&prefix),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionType {
    Command,
    Path,
    Variable,
    ExportUnset,
}

/// Reedline-backed implementation of [`LineReader`].
pub struct ShellEditor {
    editor: Reedline,
    prompt: ShellPrompt,
}

impl ShellCompleter {
    fn analyze_input(&self, line: &str, pos: usize) -> (CompletionType, String) {
        // Find the word containing the cursor
        let before_cursor = &line[..pos.min(line.len())];

        // Check if we're completing a variable (starts with $)
        if let Some(dollar_pos) = before_cursor.rfind('$') {
            let after_dollar = &before_cursor[dollar_pos + 1..];
            if after_dollar
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '{' || c == '}')
            {
                return (CompletionType::Variable, after_dollar.to_string());
            }
        }

        // Split into words to determine position
        let words: Vec<&str> = before_cursor.split_whitespace().collect();

        // Check if we're in export/unset command
        if let Some(first) = words.first() {
            if (*first == "export" || *first == "unset") && words.len() >= 2 {
                let last_word = words.last().unwrap();
                if let Some(stripped) = last_word.strip_prefix('$') {
                    return (CompletionType::Variable, stripped.to_string());
                }
                return (CompletionType::ExportUnset, last_word.to_string());
            }
        }

        // If we're at the start of the line or after a pipe/semicolon/&&/||, we're completing a command
        let trimmed = before_cursor.trim_end();
        if trimmed.is_empty()
            || trimmed.ends_with('|')
            || trimmed.ends_with(';')
            || trimmed.ends_with("&&")
            || trimmed.ends_with("||")
        {
            return (
                CompletionType::Command,
                words.last().unwrap_or(&"").to_string(),
            );
        }

        // If the first word is a command, check if we're completing a path
        if let Some(first) = words.first() {
            let _is_builtin = self.builtins.contains(&first.to_string());
            let _is_export_unset = *first == "export" || *first == "unset";
            let is_cd = *first == "cd";

            // For cd, always complete paths
            if is_cd {
                return (
                    CompletionType::Path,
                    words.last().unwrap_or(&"").to_string(),
                );
            }

            // For other commands, if the last word looks like a path, complete it
            if let Some(last) = words.last() {
                if last.contains('/')
                    || last.starts_with('.')
                    || last.starts_with('~')
                    || last.starts_with('\\')
                {
                    return (CompletionType::Path, last.to_string());
                }
            }
        }

        // Default to command completion
        (
            CompletionType::Command,
            words.last().unwrap_or(&"").to_string(),
        )
    }

    fn complete_command(&self, prefix: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Add built-ins
        for builtin in &self.builtins {
            if builtin.starts_with(prefix) {
                suggestions.push(Suggestion {
                    value: builtin.clone(),
                    display_override: None,
                    description: Some("builtin".to_string()),
                    style: None,
                    extra: None,
                    span: Span::new(0, prefix.len()),
                    append_whitespace: true,
                    match_indices: None,
                });
            }
        }

        // Add executables if we have a session
        if let Some(session) = &self.session {
            for exe in session.executables() {
                if exe.starts_with(prefix) {
                    suggestions.push(Suggestion {
                        value: exe,
                        display_override: None,
                        description: Some("external".to_string()),
                        style: None,
                        extra: None,
                        span: Span::new(0, prefix.len()),
                        append_whitespace: true,
                        match_indices: None,
                    });
                }
            }
        }

        suggestions
    }

    fn complete_path(&self, prefix: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let path = PathBuf::from(prefix);

        let parent = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let file_name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let search_dir = if parent.as_os_str().is_empty() {
            PathBuf::from(".")
        } else if parent.is_absolute() {
            parent.clone()
        } else if let Some(session) = &self.session {
            session.working_directory().join(&parent)
        } else {
            parent.clone()
        };

        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&file_name) {
                    let full_path = if prefix.contains('/') {
                        format!("{}/{}", prefix.trim_end_matches(&file_name), name)
                    } else {
                        name.clone()
                    };

                    let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                    let description = if is_dir { "directory" } else { "file" };

                    suggestions.push(Suggestion {
                        value: full_path,
                        display_override: None,
                        description: Some(description.to_string()),
                        style: None,
                        extra: None,
                        span: Span::new(0, prefix.len()),
                        append_whitespace: !is_dir,
                        match_indices: None,
                    });
                }
            }
        }

        suggestions
    }

    fn complete_variable(&self, prefix: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        if let Some(session) = &self.session {
            for var in session.variables() {
                if var.starts_with(prefix) {
                    suggestions.push(Suggestion {
                        value: format!("${var}"),
                        display_override: None,
                        description: Some("variable".to_string()),
                        style: None,
                        extra: None,
                        span: Span::new(0, prefix.len()),
                        append_whitespace: true,
                        match_indices: None,
                    });
                }
            }
        }

        suggestions
    }

    fn complete_export_unset(&self, prefix: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        if let Some(session) = &self.session {
            for var in session.exported_variables() {
                if var.starts_with(prefix) {
                    suggestions.push(Suggestion {
                        value: var,
                        display_override: None,
                        description: Some("exported".to_string()),
                        style: None,
                        extra: None,
                        span: Span::new(0, prefix.len()),
                        append_whitespace: true,
                        match_indices: None,
                    });
                }
            }

            // Also include non-exported variables for export command
            for var in session.variables() {
                if var.starts_with(prefix) && !session.exported_variables().contains(&var) {
                    suggestions.push(Suggestion {
                        value: var,
                        display_override: None,
                        description: Some("local".to_string()),
                        style: None,
                        extra: None,
                        span: Span::new(0, prefix.len()),
                        append_whitespace: true,
                        match_indices: None,
                    });
                }
            }
        }

        suggestions
    }
}

impl ShellEditor {
    /// Creates an editor with persistent command history and completion.
    ///
    /// Arrow-key history navigation, backspace, delete, and standard Emacs
    /// keybindings are provided by Reedline's defaults.
    pub fn new(
        prompt: impl Into<String>,
        history_size: usize,
        ansi_colors: bool,
        history_file: Option<PathBuf>,
        builtins: Vec<String>,
    ) -> Result<Self, LineEditorError> {
        let history = if let Some(path) = history_file {
            FileBackedHistory::with_file(history_size, path).map_err(LineEditorError::History)?
        } else {
            FileBackedHistory::new(history_size).map_err(LineEditorError::History)?
        };

        let completer = ShellCompleter::new(builtins);

        Ok(Self {
            editor: Reedline::create()
                .with_history(Box::new(history))
                .with_ansi_colors(ansi_colors)
                .with_completer(Box::new(completer)),
            prompt: ShellPrompt::new(prompt),
        })
    }

    /// Creates an editor with a session for context-aware completion.
    pub fn with_session(
        prompt: impl Into<String>,
        history_size: usize,
        ansi_colors: bool,
        history_file: Option<PathBuf>,
        builtins: Vec<String>,
        session: Arc<dyn CompletionContext>,
    ) -> Result<Self, LineEditorError> {
        let history = if let Some(path) = history_file {
            FileBackedHistory::with_file(history_size, path).map_err(LineEditorError::History)?
        } else {
            FileBackedHistory::new(history_size).map_err(LineEditorError::History)?
        };

        let completer = ShellCompleter::new(builtins).with_session(session);

        Ok(Self {
            editor: Reedline::create()
                .with_history(Box::new(history))
                .with_ansi_colors(ansi_colors)
                .with_completer(Box::new(completer)),
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
        let editor = ShellEditor::new("test > ", 25, false, None, vec![]);

        assert!(editor.is_ok());
    }
}
