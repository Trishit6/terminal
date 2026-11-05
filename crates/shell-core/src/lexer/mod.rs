//! Custom lexer and tokenizer for the shell language.
//!
//! This module is intentionally independent of command execution. It converts
//! raw input into a deterministic token stream and reports lexical errors with
//! one-based line and Unicode-scalar column positions.
//!
//! # Language syntax
//!
//! ## Words
//!
//! Unquoted words end at whitespace or at one of the operator/quoting
//! characters ``| & > < ; ( ) ' " $ =``. Other Unicode text, including
//! punctuation, is allowed in a word. There are no comments or glob expansions
//! in Phase 3; `#`, `*`, and `?` are ordinary word characters. `=` produces
//! [`Token::Equals`] so the parser can distinguish assignment syntax from
//! ordinary arguments without losing escape information.
//!
//! ## Quoting
//!
//! - `"double quoted"` and `'single quoted'` strings produce [`Token::String`].
//! - Quotes are removed from the token payload.
//! - A quoted string may span physical lines.
//! - Quote style is retained by the spanned-token API.
//! - Adjacent word/string segments are separate tokens. For example,
//!   `foo"bar"` becomes `Word("foo")`, `String("bar")`.
//! - Double-quoted variable references are emitted as adjacent text and
//!   variable tokens, so `"Hello $name"` still forms one shell word. Single
//!   quotes suppress expansion entirely.
//!
//! ## Escaping
//!
//! Backslash escapes are processed both inside and outside quotes. The
//! supported escapes are:
//!
//! | Escape | Value |
//! | --- | --- |
//! | `\\` | backslash |
//! | `\"` | double quote |
//! | `\'` | single quote |
//! | `\$` | dollar sign |
//! | `\n` | newline |
//! | `\t` | tab |
//! | `\r` | carriage return |
//! | `\ ` | space |
//! | `\|` `\&` `\>` `\<` `\;` `\(` `\)` `\=` | literal operator character |
//!
//! Any other escape is an error. A trailing backslash is also an error.
//! Backslash-newline is not treated as line continuation in Phase 3.
//!
//! ## Operators
//!
//! Operators are recognized with or without surrounding whitespace:
//!
//! | Input | Token |
//! | --- | --- |
//! | `|` | [`Token::Pipe`] |
//! | `||` | [`Token::Or`] |
//! | `&&` | [`Token::And`] |
//! | `>` | [`Token::OutputRedirect`] |
//! | `>>` | [`Token::AppendRedirect`] |
//! | `<` | [`Token::InputRedirect`] |
//! | `;` | [`Token::Semicolon`] |
//! | `=` | [`Token::Equals`] |
//! | `(` | [`Token::LeftParenthesis`] |
//! | `)` | [`Token::RightParenthesis`] |
//!
//! A single `&` is rejected as an unexpected character.
//!
//! ## Variables
//!
//! `$name` and `${name}` produce [`Token::Variable`] containing `name` without
//! the `$`. Variable names begin with an ASCII underscore or a Unicode
//! alphabetic character, followed by underscores or Unicode alphanumeric
//! characters. `$?` and `$$` produce [`Token::SpecialVariable`] values.
//! `$` inside single-quoted strings is literal text. Inside double quotes,
//! variables are tokenized for expansion.
//!
//! ## Whitespace and end of input
//!
//! Spaces, tabs, and other Unicode whitespace are skipped. Physical newlines
//! produce [`Token::Newline`]. [`Token::EndOfInput`] is always the final token,
//! including for empty input.
//!
//! # Example
//!
//! ```
//! use shell_core::{lex, Token};
//!
//! let tokens = lex(r#"echo "Hello World""#).expect("valid input");
//! assert_eq!(
//!     tokens,
//!     vec![
//!         Token::Word("echo".to_owned()),
//!         Token::String("Hello World".to_owned()),
//!         Token::EndOfInput,
//!     ]
//! );
//! ```
//!
//! # Source metadata
//!
//! [`lex_spanned`] returns [`SpannedToken`] values that pair each token with a
//! [`Span`] and, for quoted strings, the active [`QuoteKind`]. This preserves
//! quote style and exact source locations for the parser without changing the
//! Phase 3 [`lex`] API.

mod error;
mod scanner;
mod token;

pub use error::{LexerError, LexerErrorKind};
pub use scanner::{lex, lex_spanned};
pub use token::{
    Position, QuoteKind, Span, SpannedToken, SpecialVariable, Token, is_valid_variable_name,
    is_variable_name_continue, is_variable_name_start,
};

#[cfg(test)]
mod tests;
