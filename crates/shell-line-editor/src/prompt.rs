//! Prompt rendering for the interactive editor.

use reedline::{Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus};
use std::borrow::Cow;

/// Prompt that displays the configured shell prompt verbatim.
#[derive(Debug, Clone)]
pub(crate) struct ShellPrompt {
    text: String,
    color: Option<String>,
}

impl ShellPrompt {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
        }
    }

    /// Wraps the prompt in an ANSI sequence.
    ///
    /// Reedline measures prompt width after stripping ANSI, so a colored prompt
    /// keeps correct cursor math.
    pub(crate) fn with_color(mut self, color: Option<String>) -> Self {
        self.color = color;
        self
    }
}

impl Prompt for ShellPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        match &self.color {
            Some(color) => Cow::Owned(format!("{color}{}\x1b[0m", self.text)),
            None => Cow::Borrowed(&self.text),
        }
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("::: ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };

        Cow::Owned(format!(
            "({prefix}reverse-search: {}) ",
            history_search.term
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_prompt_is_rendered_verbatim() {
        let prompt = ShellPrompt::new("kernext ~ $ ");

        assert_eq!(prompt.render_prompt_left(), "kernext ~ $ ");
        assert_eq!(prompt.render_prompt_indicator(PromptEditMode::Default), "");
        assert_eq!(prompt.render_prompt_right(), "");
    }

    #[test]
    fn colored_prompt_wraps_the_text_and_resets() {
        let prompt = ShellPrompt::new("kernext ~ $ ").with_color(Some("\x1b[96m".to_owned()));
        let rendered = prompt.render_prompt_left();

        assert!(rendered.starts_with("\x1b[96m"));
        assert!(rendered.contains("kernext ~ $ "));
        assert!(rendered.ends_with("\x1b[0m"));
    }

    #[test]
    fn history_search_indicator_contains_the_term() {
        let prompt = ShellPrompt::new("kernext ~ $ ");
        let search =
            PromptHistorySearch::new(PromptHistorySearchStatus::Passing, "help".to_owned());

        let indicator = prompt.render_prompt_history_search_indicator(search);

        assert!(indicator.contains("help"));
        assert!(indicator.contains("reverse-search"));
    }
}
