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

    /// Combined display text, when the command produces output.
    ///
    /// Internal command results keep stdout and stderr separate so
    /// redirections can target them individually; this accessor returns the
    /// text the REPL should display after redirections are applied.
    #[must_use]
    pub fn output(&self) -> Option<&str> {
        self.output.as_deref()
    }

    /// Returns `true` when the final command evaluated successfully.
    ///
    /// This is the value `&&` and `||` use and the success component of the
    /// last status exposed through `$?`.
    #[must_use]
    pub const fn success(&self) -> bool {
        self.success
    }

    /// Exit code of the final stage, when it has one.
    ///
    /// Normally exited external processes and `exit <status>` provide a code.
    /// Other built-ins and signal-terminated processes use `None`; the
    /// evaluator maps signal termination to `128 + signal` for `$?`.
    #[must_use]
    pub const fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }
}
