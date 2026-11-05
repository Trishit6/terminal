//! Actions and results exchanged between commands and the REPL.

/// Action the REPL should take after a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAction {
    /// Keep the shell running.
    Continue,
    /// Clear the terminal and keep the shell running.
    ClearScreen,
    /// Clear the command history.
    ClearHistory,
    /// Shut the shell down cleanly.
    Exit,
}

/// Result of dispatching one command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    action: CommandAction,
    output: Option<String>,
    success: bool,
    exit_code: Option<i32>,
}

impl CommandOutcome {
    pub(crate) fn with_status(
        action: CommandAction,
        output: Option<String>,
        success: bool,
        exit_code: Option<i32>,
    ) -> Self {
        Self {
            action,
            output,
            success,
            exit_code,
        }
    }

    pub(crate) fn continue_with(output: impl Into<String>) -> Self {
        Self::with_status(CommandAction::Continue, Some(output.into()), false, None)
    }

    pub(crate) fn continue_silently() -> Self {
        Self::with_status(CommandAction::Continue, None, true, None)
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

    /// Returns `true` when the final command evaluated successfully.
    ///
    /// This is the value `&&` and `||` use today and the value a future
    /// scripting engine can expose through `$?`.
    #[must_use]
    pub const fn success(&self) -> bool {
        self.success
    }

    /// Exit code of the final external command, when it exited normally.
    ///
    /// Built-ins and signaled processes have no exit code.
    #[must_use]
    pub const fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }
}
