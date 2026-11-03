//! Token definitions for the custom shell language.
//!
//! Token payloads contain decoded values. Quotes are removed and escape
//! sequences are resolved before a token is produced.

/// A single lexical token in the shell language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// An unquoted word, with escapes already decoded.
    Word(String),
    /// A quoted string, with quotes removed and escapes already decoded.
    String(String),
    /// A single pipe: `|`.
    Pipe,
    /// Output redirection: `>`.
    OutputRedirect,
    /// Append redirection: `>>`.
    AppendRedirect,
    /// Input redirection: `<`.
    InputRedirect,
    /// Command separator: `;`.
    Semicolon,
    /// Logical AND: `&&`.
    And,
    /// Logical OR: `||`.
    Or,
    /// Opening grouping parenthesis: `(`.
    LeftParenthesis,
    /// Closing grouping parenthesis: `)`.
    RightParenthesis,
    /// A variable reference, stored without the leading `$`.
    Variable(String),
    /// A physical newline (`\n`, `\r\n`, or a lone `\r`).
    Newline,
    /// The end of the input stream.
    ///
    /// This token is always appended, including for empty input.
    EndOfInput,
}
