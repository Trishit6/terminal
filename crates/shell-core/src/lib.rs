//! Shared primitives for the custom shell.
//!
//! This crate deliberately contains no I/O policy of its own. It owns the
//! application identity, the error type that crosses crate boundaries, and the
//! language lexer.

pub mod lexer;

pub use lexer::{LexerError, LexerErrorKind, Token, lex};

use std::error::Error;
use std::fmt;
use std::io;

/// Canonical application name used in user-facing output and configuration.
pub const SHELL_NAME: &str = "custom-shell";

/// Workspace version, inherited from the package that compiled this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Error type shared by shell crates that can fail during startup or runtime.
#[derive(Debug)]
pub enum ShellError {
    /// An operating-system operation failed.
    Io {
        /// What the shell was trying to do when the operation failed.
        context: String,
        /// The underlying operating-system error.
        source: io::Error,
    },
    /// The shell reached an invalid state or rejected an operation.
    Message {
        /// What the shell was trying to do.
        context: String,
        /// Human-readable explanation.
        message: String,
    },
}

impl ShellError {
    /// Creates an error describing a failed I/O operation.
    #[must_use]
    pub fn io(context: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    /// Creates an error from a human-readable message.
    #[must_use]
    pub fn message(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Message {
            context: context.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ShellError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { context, source } => write!(formatter, "failed to {context}: {source}"),
            Self::Message { context, message } => {
                write!(formatter, "failed to {context}: {message}")
            }
        }
    }
}

impl Error for ShellError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Message { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_includes_context_and_source() {
        let error = ShellError::io(
            "read configuration",
            io::Error::new(io::ErrorKind::NotFound, "file not found"),
        );

        assert_eq!(
            error.to_string(),
            "failed to read configuration: file not found"
        );
        assert!(error.source().is_some());
    }

    #[test]
    fn message_error_includes_context() {
        let error = ShellError::message("start shell", "unsupported platform");

        assert_eq!(
            error.to_string(),
            "failed to start shell: unsupported platform"
        );
        assert!(error.source().is_none());
    }
}
