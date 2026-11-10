//! Headless KERNEXT worker used by the desktop application.
//!
//! The desktop app spawns its own executable with
//! `KERNEXT_DESKTOP_WORKER=1` on a pseudoterminal. This module starts the same
//! lexer, parser, evaluator, and line editor as the CLI, but without the
//! terminal startup presentation, because the desktop shell already owns the
//! window.

use crate::cli::SessionCompletionContext;
use crate::repl::Repl;
use shell_builtins::Dispatcher;
use shell_config::Config;
use shell_core::{PRODUCT_NAME, ShellError};
use shell_line_editor::{EditorSettings, ShellEditor};
use shell_runtime::Session;
use std::io::{self, IsTerminal};
use std::sync::{Arc, RwLock};

/// Starts a KERNEXT REPL with no startup presentation.
///
/// The caller owns the terminal (usually a pseudoterminal). Input, output,
/// history, completion, and command execution are identical to the CLI; only
/// the startup banner and animation are skipped.
///
/// # Errors
///
/// Returns a [`ShellError`] when the configuration, session, or line editor
/// cannot be initialized, or when the REPL cannot read or write its terminal.
pub fn run_embedded() -> Result<i32, ShellError> {
    if !io::stdin().is_terminal() {
        return Err(ShellError::message(
            "start embedded shell",
            "standard input is not a terminal; the embedded worker requires a pseudoterminal",
        ));
    }

    let config = load_config();
    let session = Session::new(config.clone())?;
    let dispatcher = Dispatcher::new();

    let builtins: Vec<String> = shell_builtins::DEFAULT_BUILTINS
        .iter()
        .map(|command| command.name().to_string())
        .collect();

    let completion_context = Arc::new(SessionCompletionContext {
        session: RwLock::new(session.clone()),
        builtins: builtins.clone(),
    });

    let editor_settings = EditorSettings {
        history_filter_secrets: config.history_filter_secrets,
        suggestions_enabled: config.suggestions_enabled,
        completion_hidden_files: config.completion_hidden_files,
    };

    let history_file = if config.history_enabled {
        config.history_path()
    } else {
        None
    };

    let editor = ShellEditor::with_session_settings(
        config.prompt,
        config.history_size,
        config.color,
        history_file,
        builtins,
        completion_context,
        editor_settings,
    )
    .map_err(|error| ShellError::message("start embedded shell", error.to_string()))?;

    let mut repl = Repl::with_dispatcher(session, editor, io::stdout(), dispatcher);

    repl.run()?;
    Ok(repl.exit_status().unwrap_or(0))
}

fn load_config() -> Config {
    let Some(path) = Config::default_path() else {
        return Config::default();
    };

    match Config::load_or_default(&path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{PRODUCT_NAME}: warning: {error}; using built-in defaults");
            Config::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_worker_reports_missing_terminal_input() {
        // The test binary's stdin is not a terminal, so the worker must refuse
        // to start instead of blocking on piped input.
        let error = run_embedded().expect_err("worker should require a terminal");

        assert!(error.to_string().contains("pseudoterminal"), "{error}");
    }
}
