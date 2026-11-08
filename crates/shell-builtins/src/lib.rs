//! Native built-in command registry, external process execution, evaluator, and
//! dispatcher.
//!
//! Phase 5 parses every command line with the existing lexer and parser, then
//! walks the resulting AST. Phase 6 adds direct external process execution:
//! native built-ins are checked first, and any other command is resolved and
//! launched with [`std::process::Command`]. Phase 7 adds pipelines connected by
//! real operating-system pipes, redirections, and built-in I/O interop. Commands
//! return outcomes instead of writing to the terminal, which keeps dispatch
//! testable and lets the REPL own output and screen clearing.

mod commands;
mod eval;
mod expand;
mod outcome;
mod pipeline;
mod process;
mod pty;
mod registry;

pub use outcome::{CommandAction, CommandOutcome};
pub use process::{
    ExternalCommand, ProcessError, ProcessOptions, ProcessReport, ProcessStatus, StdioMode,
    resolve_executable,
};
pub use pty::{PtyProcess, PtySize, terminal_size};
pub use registry::{BuiltinCommand, BuiltinRegistry, DEFAULT_BUILTINS, RegistryError};

use shell_core::{lex_spanned, parse};
use shell_runtime::Session;

/// Native command dispatcher backed by the shell lexer, parser, and AST.
#[derive(Debug)]
pub struct Dispatcher {
    registry: BuiltinRegistry,
    history: Vec<String>,
    options: ProcessOptions,
}

impl Dispatcher {
    /// Creates a dispatcher with the default built-in command set and terminal
    /// process I/O.
    #[must_use]
    pub fn new() -> Self {
        Self::with_options(ProcessOptions::default())
    }

    /// Creates a dispatcher with explicit process options.
    ///
    /// The REPL uses the defaults (inherit the terminal). Tests and
    /// non-interactive callers can capture output or override child environment
    /// entries without touching the shell process's global environment.
    #[must_use]
    pub fn with_options(options: ProcessOptions) -> Self {
        Self {
            registry: BuiltinRegistry::with_defaults(),
            history: Vec::new(),
            options,
        }
    }

    /// Registry used to build help output.
    #[must_use]
    pub fn registry(&self) -> &BuiltinRegistry {
        &self.registry
    }

    /// Process options applied to external commands.
    #[must_use]
    pub const fn process_options(&self) -> &ProcessOptions {
        &self.options
    }

    /// Commands recorded so far, oldest first.
    #[must_use]
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Clears the in-memory command history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Dispatches a command line through the lexer, parser, and AST evaluator.
    ///
    /// Empty input is silent. Lexical and syntax errors produce a readable
    /// message and leave the session running. Unknown command names are treated
    /// as external programs and produce a friendly message when resolution
    /// fails.
    pub fn execute(&mut self, session: &mut Session, input: &str) -> CommandOutcome {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return CommandOutcome::continue_silently();
        }

        self.record_history(session, trimmed);

        let tokens = match lex_spanned(trimmed) {
            Ok(tokens) => tokens,
            Err(error) => {
                session.set_last_status(2);
                return CommandOutcome::continue_with(format!("Syntax error: {error}"));
            }
        };

        let ast = match parse(&tokens) {
            Ok(ast) => ast,
            Err(error) => {
                session.set_last_status(2);
                return CommandOutcome::continue_with(format!("Syntax error: {error}"));
            }
        };

        if ast.is_empty() {
            return CommandOutcome::continue_silently();
        }

        eval::evaluate(&self.registry, &self.history, session, &ast, &self.options)
    }

    /// Builds the help text from the command registry.
    #[must_use]
    pub fn help_text(&self) -> String {
        self.registry.help_text()
    }

    fn record_history(&mut self, session: &Session, input: &str) {
        if session.config().history_filter_secrets
            && shell_runtime::history::is_sensitive_command(input)
        {
            return;
        }

        let limit = session
            .history_limit()
            .unwrap_or(session.config().history_size)
            .max(1);
        self.history.push(input.to_owned());

        if self.history.len() > limit {
            let excess = self.history.len() - limit;
            self.history.drain(0..excess);
        }
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session::new(Default::default()).expect("session should start")
    }

    #[test]
    fn default_registry_contains_every_phase_five_command() {
        let dispatcher = Dispatcher::new();

        assert_eq!(dispatcher.registry().len(), DEFAULT_BUILTINS.len());
    }

    #[test]
    fn empty_and_whitespace_input_is_silent() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        for input in ["", "   ", "\t\n"] {
            let outcome = dispatcher.execute(&mut session, input);

            assert_eq!(outcome.action(), CommandAction::Continue);
            assert_eq!(outcome.output(), None);
        }

        assert!(dispatcher.history().is_empty());
    }

    #[test]
    fn help_lists_all_available_commands() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "help");

        assert_eq!(outcome.action(), CommandAction::Continue);
        let output = outcome.output().expect("help should produce output");
        for command in DEFAULT_BUILTINS {
            assert!(output.contains(command.name()));
        }
        assert!(output.starts_with("KERNEXT — Available commands:"));
    }

    #[test]
    fn version_uses_the_session_version() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "version");

        assert_eq!(outcome.action(), CommandAction::Continue);
        assert_eq!(
            outcome.output(),
            Some(format!("KERNEXT v{}", session.version()).as_str())
        );
    }

    #[test]
    fn unknown_commands_produce_a_friendly_message() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "example");

        assert_eq!(outcome.action(), CommandAction::Continue);
        assert_eq!(outcome.output(), Some("Command not found: example"));
    }

    #[test]
    fn surrounding_and_repeated_whitespace_is_ignored() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "  version   extra  ");

        assert_eq!(outcome.action(), CommandAction::Continue);
        assert!(
            outcome
                .output()
                .is_some_and(|text| text.starts_with("KERNEXT v"))
        );
    }

    #[test]
    fn clear_requests_terminal_clear_without_output() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "clear");

        assert_eq!(outcome.action(), CommandAction::ClearScreen);
        assert_eq!(outcome.output(), None);
    }

    #[test]
    fn exit_requests_shutdown_with_a_farewell() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "exit");

        assert_eq!(outcome.action(), CommandAction::Exit);
        assert_eq!(outcome.output(), Some("Goodbye."));
    }

    #[test]
    fn pwd_reports_the_session_working_directory() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "pwd");

        assert_eq!(
            outcome.output(),
            Some(session.working_directory().display().to_string().as_str())
        );
    }

    #[test]
    fn history_records_commands_in_order() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        dispatcher.execute(&mut session, "version");
        let outcome = dispatcher.execute(&mut session, "history");

        let output = outcome.output().expect("history should produce output");
        assert!(output.contains("1  version"), "{output}");
        assert!(output.contains("2  history"), "{output}");
    }

    #[test]
    fn semicolons_and_logical_operators_use_the_ast() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "echo one; echo two");

        assert_eq!(outcome.output(), Some("one\ntwo\n"));
    }

    #[test]
    fn logical_or_runs_the_right_side_after_an_unknown_command() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "missing || echo recovered");

        let output = outcome.output().expect("pipeline should produce output");
        assert!(output.contains("Command not found: missing"));
        assert!(output.contains("recovered"));
    }

    #[test]
    fn groups_evaluate_their_command_list() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "(echo a; echo b)");

        assert_eq!(outcome.output(), Some("a\nb\n"));
    }

    #[test]
    fn unterminated_quotes_report_a_syntax_error_without_panicking() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "echo \"unterminated");

        assert_eq!(outcome.action(), CommandAction::Continue);
        assert!(
            outcome
                .output()
                .is_some_and(|text| text.starts_with("Syntax error:"))
        );
        assert_eq!(session.last_status(), 2);
    }

    #[test]
    fn parser_errors_set_the_status_to_two() {
        let mut dispatcher = Dispatcher::new();
        let mut session = session();

        let outcome = dispatcher.execute(&mut session, "if true");

        assert_eq!(outcome.action(), CommandAction::Continue);
        assert!(
            outcome
                .output()
                .is_some_and(|text| text.starts_with("Syntax error:"))
        );
        assert_eq!(session.last_status(), 2);
    }
}
