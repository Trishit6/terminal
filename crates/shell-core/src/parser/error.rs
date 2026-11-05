//! Parser errors with source locations.

use crate::lexer::Span;
use std::error::Error;
use std::fmt;

/// A syntax error and the source range where it was detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    kind: ParseErrorKind,
    span: Span,
}

impl ParseError {
    pub(crate) const fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// What kind of syntax error occurred.
    #[must_use]
    pub const fn kind(&self) -> &ParseErrorKind {
        &self.kind
    }

    /// Source range associated with the error.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// One-based line where the error was detected.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.span.start().line()
    }

    /// One-based Unicode-scalar column where the error was detected.
    #[must_use]
    pub const fn column(&self) -> usize {
        self.span.start().column()
    }
}

/// Specific categories of syntax failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// A command was required but no word was found.
    MissingCommand,
    /// A pipe was not followed by a command.
    MissingPipeOperand,
    /// A redirection operator was not followed by a target word.
    MissingRedirectionTarget,
    /// A logical operator was not followed by a command.
    MissingOperand {
        /// The operator that needs a right-hand operand.
        operator: &'static str,
    },
    /// An operator appeared where a command was expected.
    UnexpectedOperator {
        /// The rejected operator.
        operator: &'static str,
    },
    /// A closing parenthesis appeared outside a group.
    UnexpectedClosingParenthesis,
    /// A group was opened but never closed.
    UnclosedParenthesis,
    /// The token sequence does not form a valid command structure.
    InvalidCommandStructure {
        /// Human-readable explanation.
        message: &'static str,
    },
    /// Input ended where more tokens were required.
    UnexpectedEndOfInput,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "parse error at line {}, column {}: {}",
            self.line(),
            self.column(),
            self.kind
        )
    }
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => formatter.write_str("expected a command"),
            Self::MissingPipeOperand => formatter.write_str("expected a command after `|`"),
            Self::MissingRedirectionTarget => formatter.write_str("expected a redirection target"),
            Self::MissingOperand { operator } => {
                write!(formatter, "expected a command after `{operator}`")
            }
            Self::UnexpectedOperator { operator } => {
                write!(formatter, "unexpected operator `{operator}`")
            }
            Self::UnexpectedClosingParenthesis => {
                formatter.write_str("unexpected closing parenthesis `)`")
            }
            Self::UnclosedParenthesis => formatter.write_str("unclosed opening parenthesis `(`"),
            Self::InvalidCommandStructure { message } => {
                write!(formatter, "invalid command structure: {message}")
            }
            Self::UnexpectedEndOfInput => formatter.write_str("unexpected end of input"),
        }
    }
}

impl Error for ParseError {}
