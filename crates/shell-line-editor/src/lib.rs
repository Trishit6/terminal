//! Interactive input primitives for the custom shell.
//!
//! Reedline supplies terminal input, key handling, and history navigation. This
//! crate keeps that dependency behind [`LineReader`] so the REPL remains
//! testable and the application does not depend on terminal details.

mod editor;
mod prompt;

pub use editor::{
    CompletionContext, EditorSettings, LineEditorError, LineEvent, LineReader, ShellCompleter,
    ShellEditor, clear_terminal,
};

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

/// Bounded command history with previous/next navigation.
#[derive(Debug)]
pub struct History {
    entries: VecDeque<String>,
    limit: usize,
    cursor: Option<usize>,
}

impl History {
    /// Creates an empty history that retains at most `limit` entries.
    pub fn new(limit: usize) -> Result<Self, HistoryError> {
        if limit == 0 {
            return Err(HistoryError::ZeroCapacity);
        }

        Ok(Self {
            entries: VecDeque::with_capacity(limit.min(64)),
            limit,
            cursor: None,
        })
    }

    /// Appends a command, ignoring empty input and consecutive duplicates.
    ///
    /// When capacity is exceeded, the oldest entry is discarded.
    pub fn push(&mut self, entry: impl Into<String>) {
        let entry = entry.into();
        self.cursor = None;

        if entry.is_empty() || self.entries.back() == Some(&entry) {
            return;
        }

        if self.entries.len() == self.limit {
            self.entries.pop_front();
        }

        self.entries.push_back(entry);
    }

    /// Moves to the previous entry and returns it.
    ///
    /// Repeated calls walk toward the oldest entry and stop there.
    pub fn previous_entry(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }

        let index = match self.cursor {
            None => self.entries.len() - 1,
            Some(0) => 0,
            Some(index) => index - 1,
        };
        self.cursor = Some(index);
        self.entries.get(index).map(String::as_str)
    }

    /// Moves to the next entry and returns it.
    ///
    /// Moving past the newest entry resets navigation and returns `None`.
    pub fn next_entry(&mut self) -> Option<&str> {
        let index = self.cursor?;

        if index + 1 >= self.entries.len() {
            self.cursor = None;
            return None;
        }

        let index = index + 1;
        self.cursor = Some(index);
        self.entries.get(index).map(String::as_str)
    }

    /// Leaves navigation mode without changing stored entries.
    pub fn reset_navigation(&mut self) {
        self.cursor = None;
    }

    /// Iterates over entries from oldest to newest.
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.entries.iter().map(String::as_str)
    }

    /// Number of stored entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when no entries are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Maximum number of entries this history retains.
    #[must_use]
    pub const fn limit(&self) -> usize {
        self.limit
    }
}

/// Errors produced when creating a history buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryError {
    /// History was configured with zero capacity.
    ZeroCapacity,
}

impl fmt::Display for HistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCapacity => formatter.write_str("history capacity must be greater than zero"),
        }
    }
}

impl Error for HistoryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_capacity_is_rejected() {
        let error = History::new(0).expect_err("zero capacity should fail");

        assert_eq!(error, HistoryError::ZeroCapacity);
    }

    #[test]
    fn oldest_entries_are_evicted_at_capacity() {
        let mut history = History::new(2).expect("history should be created");

        history.push("first");
        history.push("second");
        history.push("third");

        assert_eq!(history.len(), 2);
        assert_eq!(history.entries().collect::<Vec<_>>(), ["second", "third"]);
    }

    #[test]
    fn empty_and_consecutive_duplicate_entries_are_ignored() {
        let mut history = History::new(4).expect("history should be created");

        history.push("");
        history.push("ls");
        history.push("ls");
        history.push("pwd");

        assert_eq!(history.entries().collect::<Vec<_>>(), ["ls", "pwd"]);
    }

    #[test]
    fn navigation_walks_from_newest_to_oldest_and_back() {
        let mut history = History::new(4).expect("history should be created");
        history.push("one");
        history.push("two");
        history.push("three");

        assert_eq!(history.previous_entry(), Some("three"));
        assert_eq!(history.previous_entry(), Some("two"));
        assert_eq!(history.previous_entry(), Some("one"));
        assert_eq!(history.previous_entry(), Some("one"));
        assert_eq!(history.next_entry(), Some("two"));
        assert_eq!(history.next_entry(), Some("three"));
        assert_eq!(history.next_entry(), None);
        assert_eq!(history.next_entry(), None);
    }

    #[test]
    fn pushing_resets_navigation() {
        let mut history = History::new(4).expect("history should be created");
        history.push("one");
        history.push("two");
        assert_eq!(history.previous_entry(), Some("two"));

        history.push("three");

        assert_eq!(history.previous_entry(), Some("three"));
    }

    #[test]
    fn reset_navigation_returns_to_the_newest_position() {
        let mut history = History::new(4).expect("history should be created");
        history.push("one");
        history.push("two");
        assert_eq!(history.previous_entry(), Some("two"));
        assert_eq!(history.previous_entry(), Some("one"));

        history.reset_navigation();

        assert_eq!(history.previous_entry(), Some("two"));
    }
}

#[cfg(test)]
mod editor_tests {
    use super::*;
    use reedline::Completer;
    use std::path::PathBuf;
    use std::sync::Arc;

    struct MockContext;

    impl CompletionContext for MockContext {
        fn builtins(&self) -> &[String] {
            static BUILTINS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
            BUILTINS.get_or_init(|| vec!["help".to_string(), "exit".to_string()])
        }

        fn executables(&self) -> Vec<String> {
            vec!["ls".to_string(), "cat".to_string()]
        }

        fn variables(&self) -> Vec<String> {
            vec!["HOME".to_string(), "PATH".to_string()]
        }

        fn exported_variables(&self) -> Vec<String> {
            vec!["HOME".to_string()]
        }

        fn working_directory(&self) -> PathBuf {
            PathBuf::from("/tmp")
        }
    }

    struct TempContext {
        directory: PathBuf,
    }

    impl TempContext {
        fn new() -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after the Unix epoch")
                .as_nanos();
            let directory = std::env::temp_dir().join(format!(
                "kernext-completion-{}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir_all(directory.join("src"))
                .expect("temp directory should be created");
            std::fs::write(directory.join("src").join("main.rs"), "")
                .expect("temp file should be created");
            Self { directory }
        }
    }

    impl Drop for TempContext {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }

    impl CompletionContext for TempContext {
        fn builtins(&self) -> &[String] {
            static BUILTINS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
            BUILTINS.get_or_init(|| vec!["cat".to_string()])
        }

        fn executables(&self) -> Vec<String> {
            Vec::new()
        }

        fn variables(&self) -> Vec<String> {
            Vec::new()
        }

        fn exported_variables(&self) -> Vec<String> {
            Vec::new()
        }

        fn working_directory(&self) -> PathBuf {
            self.directory.clone()
        }
    }

    #[test]
    fn editor_can_be_created_with_configured_history() {
        let editor = ShellEditor::new("test > ", 25, false, None, vec!["help".to_string()]);

        assert!(editor.is_ok());
    }

    #[test]
    fn editor_can_be_created_with_history_file() {
        let temp_dir = std::env::temp_dir();
        let history_file = temp_dir.join("test_history.txt");
        let editor = ShellEditor::new(
            "test > ",
            25,
            false,
            Some(history_file.clone()),
            vec!["help".to_string()],
        );

        assert!(editor.is_ok());
        let _ = std::fs::remove_file(history_file);
    }

    #[test]
    fn editor_can_be_created_with_session() {
        let session = Arc::new(MockContext);
        let editor = ShellEditor::with_session(
            "test > ",
            25,
            false,
            None,
            vec!["help".to_string()],
            session,
        );

        assert!(editor.is_ok());
    }

    #[test]
    fn completer_completes_builtins() {
        let mut completer = ShellCompleter::new(vec![
            "help".to_string(),
            "history".to_string(),
            "exit".to_string(),
        ]);
        let suggestions = completer.complete("he", 2);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].value, "help");
    }

    #[test]
    fn completer_completes_variables() {
        let session = Arc::new(MockContext);
        let mut completer = ShellCompleter::new(vec!["help".to_string()]).with_session(session);
        let suggestions = completer.complete("$HO", 3);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].value, "$HOME");
    }

    #[test]
    fn command_completion_replaces_only_the_current_word() {
        let mut completer = ShellCompleter::new(vec!["help".to_string()]);

        let suggestions = completer.complete("echo x | he", 11);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].value, "help");
        assert_eq!(suggestions[0].span.start, 9);
        assert_eq!(suggestions[0].span.end, 11);
    }

    #[test]
    fn variable_completion_keeps_the_prefix_before_the_cursor() {
        let session = Arc::new(MockContext);
        let mut completer = ShellCompleter::new(vec![]).with_session(session);

        let suggestions = completer.complete("echo $HO", 8);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].value, "$HOME");
        assert_eq!(suggestions[0].span.start, 5);
        assert_eq!(suggestions[0].span.end, 8);
    }

    #[test]
    fn path_completion_preserves_the_typed_directory_and_span() {
        let context = TempContext::new();
        let mut completer = ShellCompleter::new(vec![]).with_session(Arc::new(context));

        let suggestions = completer.complete("cat src/ma", 10);

        assert!(
            suggestions
                .iter()
                .any(|suggestion| suggestion.value == "src/main.rs"),
            "{suggestions:?}"
        );
        let file = suggestions
            .iter()
            .find(|suggestion| suggestion.value == "src/main.rs")
            .expect("main.rs should be suggested");
        assert_eq!(file.span.start, 4);
        assert_eq!(file.span.end, 10);
    }

    #[test]
    fn path_completion_works_immediately_after_cd() {
        let context = TempContext::new();
        let mut completer = ShellCompleter::new(vec![]).with_session(Arc::new(context));

        let suggestions = completer.complete("cd ", 3);

        let expected = format!("src{}", std::path::MAIN_SEPARATOR);
        assert!(
            suggestions
                .iter()
                .any(|suggestion| suggestion.value == expected),
            "{suggestions:?}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn path_completion_accepts_windows_style_separators() {
        let context = TempContext::new();
        let mut completer = ShellCompleter::new(vec![]).with_session(Arc::new(context));

        let suggestions = completer.complete("cat src\\ma", 10);

        assert!(
            suggestions
                .iter()
                .any(|suggestion| suggestion.value == "src\\main.rs"),
            "{suggestions:?}"
        );
    }
}
