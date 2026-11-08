//! Native command metadata and registration.

use shell_core::PRODUCT_NAME;
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

/// Commands available in the native command engine.
pub const DEFAULT_BUILTINS: [BuiltinCommand; 53] = [
    BuiltinCommand::new("help", "Show this help message"),
    BuiltinCommand::new("pwd", "Print the current working directory"),
    BuiltinCommand::new("cd", "Change the working directory"),
    BuiltinCommand::new("ls", "List directory contents"),
    BuiltinCommand::new("tree", "Display a directory tree"),
    BuiltinCommand::new("find", "Search for files and directories"),
    BuiltinCommand::new("stat", "Show file metadata"),
    BuiltinCommand::new("mkdir", "Create directories"),
    BuiltinCommand::new("touch", "Create files or update timestamps"),
    BuiltinCommand::new("cat", "Print file contents"),
    BuiltinCommand::new("head", "Show the first lines of input"),
    BuiltinCommand::new("tail", "Show the last lines of input"),
    BuiltinCommand::new("wc", "Count lines, words, and bytes"),
    BuiltinCommand::new("grep", "Search text for a literal pattern"),
    BuiltinCommand::new("sort", "Sort lines of text"),
    BuiltinCommand::new("uniq", "Report or omit repeated lines"),
    BuiltinCommand::new("echo", "Print arguments"),
    BuiltinCommand::new("print", "Print arguments (scripting form of echo)"),
    BuiltinCommand::new("input", "Read one line of input into a variable"),
    BuiltinCommand::new("len", "Report the character length of a value"),
    BuiltinCommand::new("type", "Report a value's type"),
    BuiltinCommand::new("str", "Convert a value to a string"),
    BuiltinCommand::new("int", "Convert a value to an integer"),
    BuiltinCommand::new("float", "Convert a value to a floating-point number"),
    BuiltinCommand::new("bool", "Convert a value to a boolean"),
    BuiltinCommand::new("cp", "Copy files and directories"),
    BuiltinCommand::new("mv", "Move or rename files and directories"),
    BuiltinCommand::new("rm", "Remove files and directories"),
    BuiltinCommand::new("which", "Locate a built-in or executable"),
    BuiltinCommand::new("basename", "Strip directory and suffix from a path"),
    BuiltinCommand::new("dirname", "Strip the last component from a path"),
    BuiltinCommand::new("realpath", "Resolve a path to an absolute path"),
    BuiltinCommand::new("clear", "Clear the terminal screen"),
    BuiltinCommand::new("history", "Show command history"),
    BuiltinCommand::new("jobs", "List background jobs"),
    BuiltinCommand::new("wait", "Wait for background jobs"),
    BuiltinCommand::new("kill", "Terminate a background job"),
    BuiltinCommand::new("fg", "Wait for a background job"),
    BuiltinCommand::new("bg", "Report a background job's state"),
    BuiltinCommand::new("run", "Run a KERNEXT script file"),
    BuiltinCommand::new("test", "Evaluate a conditional expression"),
    BuiltinCommand::new("expr", "Evaluate an arithmetic expression"),
    BuiltinCommand::new("sleep", "Pause for a number of seconds"),
    BuiltinCommand::new("true", "Return a successful status"),
    BuiltinCommand::new("false", "Return a failing status"),
    BuiltinCommand::new("whoami", "Show the current user name"),
    BuiltinCommand::new("date", "Show the current date and time"),
    BuiltinCommand::new("version", "Show the shell version"),
    BuiltinCommand::new("exit", "Exit the shell"),
    BuiltinCommand::new("export", "Mark variables for export to child processes"),
    BuiltinCommand::new("unset", "Remove shell variables"),
    BuiltinCommand::new("env", "Show the environment for child processes"),
    BuiltinCommand::new("set", "Show shell variables"),
];

/// Ordered collection of built-in commands.
#[derive(Debug, Default, Clone)]
pub struct BuiltinRegistry {
    commands: Vec<BuiltinCommand>,
}

impl BuiltinRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a registry containing the default built-in commands.
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

    /// Builds the help text from the registered command metadata.
    #[must_use]
    pub fn help_text(&self) -> String {
        let mut output = format!("{PRODUCT_NAME} — Available commands:\n");

        for command in &self.commands {
            output.push_str(&format!("  {:<10} {}\n", command.name(), command.summary()));
        }

        output.push_str("\nCommands not listed above are executed as external programs.");

        output.trim_end().to_owned()
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
    fn default_registry_contains_all_phase_five_commands() {
        let registry = BuiltinRegistry::with_defaults();

        assert_eq!(registry.len(), DEFAULT_BUILTINS.len());
        for expected in DEFAULT_BUILTINS {
            assert_eq!(registry.get(expected.name()), Some(&expected));
        }
    }

    #[test]
    fn help_text_lists_every_registered_command() {
        let registry = BuiltinRegistry::with_defaults();
        let help = registry.help_text();

        assert!(help.starts_with("KERNEXT — Available commands:"));
        for command in DEFAULT_BUILTINS {
            assert!(help.contains(command.name()));
        }
    }
}
