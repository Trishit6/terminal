//! Lexer error types with source locations.

use std::error::Error;
use std::fmt;

/// A lexical error and the position where it was detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    kind: LexerErrorKind,
    line: usize,
    column: usize,
}

impl LexerError {
    pub(crate) const fn new(kind: LexerErrorKind, line: usize, column: usize) -> Self {
        Self { kind, line, column }
    }

    /// What kind of lexical error occurred.
    #[must_use]
    pub const fn kind(&self) -> &LexerErrorKind {
        &self.kind
    }

    /// One-based line number where the error was detected.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    /// One-based Unicode-scalar column where the error was detected.
    #[must_use]
    pub const fn column(&self) -> usize {
        self.column
    }
}

/// Specific categories of lexical failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexerErrorKind {
    /// A single-quoted string reached the end of input before its closing quote.
    UnterminatedSingleQuote,
    /// A double-quoted string reached the end of input before its closing quote.
    UnterminatedDoubleQuote,
    /// A backslash was followed by a character that is not a supported escape.
    InvalidEscape {
        /// The character following the backslash.
        character: char,
    },
    /// A backslash appeared at the end of input.
    DanglingEscape,
    /// `$` was not followed by a valid variable name.
    InvalidVariable,
    /// `${}` contained no variable name.
    EmptyBracedVariable,
    /// A `${name` reference reached the end of input without `}`.
    UnterminatedBracedVariable,
    /// A `${...}` reference contained an invalid variable name.
    InvalidBracedVariable,
    /// A character cannot appear at the current position.
    UnexpectedCharacter {
        /// The rejected character.
        character: char,
    },
}

impl fmt::Display for LexerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "lex error at line {}, column {}: {}",
            self.line, self.column, self.kind
        )
    }
}

impl fmt::Display for LexerErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnterminatedSingleQuote => {
                formatter.write_str("unterminated single-quoted string")
            }
            Self::UnterminatedDoubleQuote => {
                formatter.write_str("unterminated double-quoted string")
            }
            Self::InvalidEscape { character } => write!(
                formatter,
                "invalid escape sequence `\\{}`",
                character.escape_default()
            ),
            Self::DanglingEscape => formatter.write_str("backslash at end of input"),
            Self::UnexpectedCharacter { character } => {
                write!(
                    formatter,
                    "unexpected character `{}`",
                    character.escape_default()
                )
            }
            Self::InvalidVariable => formatter.write_str("expected a variable name after `$`"),
            Self::EmptyBracedVariable => formatter.write_str("empty variable name inside `${}`"),
            Self::UnterminatedBracedVariable => {
                formatter.write_str("unterminated `${` variable reference")
            }
            Self::InvalidBracedVariable => {
                formatter.write_str("invalid variable name inside `${...}`")
            }
        }
    }
}

impl Error for LexerError {}
