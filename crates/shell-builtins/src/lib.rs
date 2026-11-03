//! Built-in command registry and Phase 2 command dispatcher.
//!
//! Commands return outcomes instead of writing to the terminal. This keeps the
//! dispatcher testable and lets the REPL own output and screen clearing.

use shell_runtime::Session;
use std::error::Error;
use std::fmt;

/// Static metadata for a native shell command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinCommand {
    name: &'static str,
    summary: &'static str,
}

impl BuiltinCommand {
    /// Creates command metadata.
    #[must_use]
    pub const fn new(name: &'static str, summary: &'static str) -> Self {
        Self { name, summary }
    }

    /// Name typed by the user.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// One-line description shown by `help`.
    #[must_use]
    pub const fn summary(self) -> &'static str {
        self.summary
    }
}

/// Commands available during Phase 2.
pub const DEFAULT_BUILTINS: [BuiltinCommand; 4] = [
    BuiltinCommand::new("help", "Show this help message"),
    BuiltinCommand::new("version", "Show the shell version"),
    BuiltinCommand::new("clear", "Clear the terminal screen"),
    BuiltinCommand::new("exit", "Exit the shell"),
];

/// Ordered collection of built-in commands.
#[derive(Debug, Default)]
pub struct BuiltinRegistry {
    commands: Vec<BuiltinCommand>,
}

impl BuiltinRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a registry containing the Phase 2 built-in commands.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            commands: DEFAULT_BUILTINS.to_vec(),
        }
    }

    /// Adds a command, rejecting empty names and duplicates.
    pub fn register(&mut self, command: BuiltinCommand) -> Result<(), RegistryError> {
        if command.name().trim().is_empty() {
            return Err(RegistryError::EmptyName);
        }

        if self.get(command.name()).is_some() {
            return Err(RegistryError::Duplicate {
                name: command.name(),
            });
        }

        self.commands.push(command);
        Ok(())
    }

    /// Looks up a command by its exact name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&BuiltinCommand> {
        self.commands.iter().find(|command| command.name() == name)
    }

    /// Iterates over commands in registration order.
    pub fn commands(&self) -> impl Iterator<Item = &BuiltinCommand> {
        self.commands.iter()
    }

    /// Number of registered commands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns `true` when no commands are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// Action the REPL should take after a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAction {
    /// Keep the shell running.
    Continue,
    /// Clear the terminal and keep the shell running.
    ClearScreen,
    /// Shut the shell down cleanly.
    Exit,
}

/// Result of dispatching one command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    action: CommandAction,
    output: Option<String>,
}

impl CommandOutcome {
    fn new(action: CommandAction, output: Option<String>) -> Self {
        Self { action, output }
    }

    fn continue_with(output: impl Into<String>) -> Self {
        Self::new(CommandAction::Continue, Some(output.into()))
    }

    fn continue_silently() -> Self {
        Self::new(CommandAction::Continue, None)
    }

    fn clear_screen() -> Self {
        Self::new(CommandAction::ClearScreen, None)
    }

    fn exit() -> Self {
        Self::new(CommandAction::Exit, Some("Goodbye.".to_owned()))
    }

    /// Action the REPL should perform.
    #[must_use]
    pub const fn action(&self) -> CommandAction {
        self.action
    }

    /// Text to display, when the command produces output.
    #[must_use]
    pub fn output(&self) -> Option<&str> {
        self.output.as_deref()
    }
}

/// Minimal Phase 2 command dispatcher.
#[derive(Debug)]
pub struct Dispatcher {
    registry: BuiltinRegistry,
}

impl Dispatcher {
    /// Creates a dispatcher with the default built-in command set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: BuiltinRegistry::with_defaults(),
        }
    }

    /// Registry used to build help output.
    #[must_use]
    pub fn registry(&self) -> &BuiltinRegistry {
        &self.registry
    }

    /// Dispatches a command line.
    ///
    /// Leading, trailing, and repeated whitespace is ignored. Unknown command
    /// names produce a friendly message and leave the session running.
    #[must_use]
    pub fn execute(&self, session: &Session, input: &str) -> CommandOutcome {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return CommandOutcome::continue_silently();
        }

        let Some(name) = trimmed.split_whitespace().next() else {
            return CommandOutcome::continue_silently();
        };

        match name {
            "help" => CommandOutcome::continue_with(self.help_text()),
            "version" => {
                CommandOutcome::continue_with(format!("Custom Shell v{}", session.version()))
            }
            "clear" => CommandOutcome::clear_screen(),
            "exit" => CommandOutcome::exit(),
            _ => CommandOutcome::continue_with(format!("Command not found: {name}")),
        }
    }

    /// Builds the help text from the command registry.
    #[must_use]
    pub fn help_text(&self) -> String {
        let mut output = String::from("Available commands:\n");

        for command in self.registry.commands() {
            output.push_str(&format!("  {:<8} {}\n", command.name(), command.summary()));
        }

        output.trim_end().to_owned()
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors produced while building a command registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    /// A command had a blank name.
    EmptyName,
    /// A command with the same name was already registered.
    Duplicate {
        /// The conflicting command name.
        name: &'static str,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("a built-in command name cannot be empty"),
            Self::Duplicate { name } => {
                write!(formatter, "built-in command `{name}` is already registered")
            }
        }
    }
}

impl Error for RegistryError {}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_COMMAND: BuiltinCommand = BuiltinCommand::new("version", "Print the shell version");

    fn session() -> Session {
        Session::new(Default::default()).expect("session should start")
    }

    #[test]
    fn commands_can_be_registered_and_looked_up() {
        let mut registry = BuiltinRegistry::new();

        registry
            .register(TEST_COMMAND)
            .expect("command should register");

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert_eq!(registry.get("version"), Some(&TEST_COMMAND));
        assert_eq!(registry.get("missing"), None);
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let mut registry = BuiltinRegistry::new();
        registry
            .register(TEST_COMMAND)
            .expect("first command should register");

        let error = registry
            .register(TEST_COMMAND)
            .expect_err("duplicate command should fail");

        assert_eq!(error, RegistryError::Duplicate { name: "version" });
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn empty_names_are_rejected() {
        let mut registry = BuiltinRegistry::new();

        let error = registry
            .register(BuiltinCommand::new("  ", "Blank command"))
            .expect_err("blank command should fail");

        assert_eq!(error, RegistryError::EmptyName);
        assert!(registry.is_empty());
    }

    #[test]
    fn commands_preserve_registration_order() {
        let mut registry = BuiltinRegistry::new();
        registry
            .register(TEST_COMMAND)
            .expect("first command should register");
        registry
            .register(BuiltinCommand::new("exit", "Exit the shell"))
            .expect("second command should register");

        let names: Vec<_> = registry.commands().map(|command| command.name()).collect();

        assert_eq!(names, ["version", "exit"]);
    }

    #[test]
    fn default_registry_contains_all_phase_two_commands() {
        let registry = BuiltinRegistry::with_defaults();

        assert_eq!(registry.len(), DEFAULT_BUILTINS.len());
        for expected in DEFAULT_BUILTINS {
            assert_eq!(registry.get(expected.name()), Some(&expected));
        }
    }

    #[test]
    fn empty_and_whitespace_input_is_silent() {
        let dispatcher = Dispatcher::new();
        let session = session();

        for input in ["", "   ", "\t\n"] {
            let outcome = dispatcher.execute(&session, input);

            assert_eq!(outcome.action(), CommandAction::Continue);
            assert_eq!(outcome.output(), None);
        }
    }

    #[test]
    fn help_lists_all_available_commands() {
        let dispatcher = Dispatcher::new();
        let outcome = dispatcher.execute(&session(), "help");

        assert_eq!(outcome.action(), CommandAction::Continue);
        let output = outcome.output().expect("help should produce output");
        for command in DEFAULT_BUILTINS {
            assert!(output.contains(command.name()));
        }
        assert!(output.starts_with("Available commands:"));
    }

    #[test]
    fn version_uses_the_session_version() {
        let dispatcher = Dispatcher::new();
        let session = session();

        let outcome = dispatcher.execute(&session, "version");

        assert_eq!(outcome.action(), CommandAction::Continue);
        assert_eq!(
            outcome.output(),
            Some(format!("Custom Shell v{}", session.version()).as_str())
        );
    }

    #[test]
    fn unknown_commands_produce_a_friendly_message() {
        let dispatcher = Dispatcher::new();

        let outcome = dispatcher.execute(&session(), "example");

        assert_eq!(outcome.action(), CommandAction::Continue);
        assert_eq!(outcome.output(), Some("Command not found: example"));
    }

    #[test]
    fn surrounding_and_repeated_whitespace_is_ignored() {
        let dispatcher = Dispatcher::new();

        let outcome = dispatcher.execute(&session(), "  version   extra  ");

        assert_eq!(outcome.action(), CommandAction::Continue);
        assert!(
            outcome
                .output()
                .is_some_and(|text| text.starts_with("Custom Shell v"))
        );
    }

    #[test]
    fn clear_requests_terminal_clear_without_output() {
        let dispatcher = Dispatcher::new();

        let outcome = dispatcher.execute(&session(), "clear");

        assert_eq!(outcome.action(), CommandAction::ClearScreen);
        assert_eq!(outcome.output(), None);
    }

    #[test]
    fn exit_requests_shutdown_with_a_farewell() {
        let dispatcher = Dispatcher::new();

        let outcome = dispatcher.execute(&session(), "exit");

        assert_eq!(outcome.action(), CommandAction::Exit);
        assert_eq!(outcome.output(), Some("Goodbye."));
    }
}
