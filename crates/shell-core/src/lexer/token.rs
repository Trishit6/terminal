//! Token definitions for the custom shell language.
//!
//! Token payloads contain decoded values. Quotes are removed and escape
//! sequences are resolved before a token is produced.
//!
//! [`Token`] is the compact Phase 3 token representation. [`SpannedToken`]
//! wraps the same token kind with source positions and, for quoted strings,
//! the quote style that produced it. The parser consumes [`SpannedToken`] values
//! so the AST can retain precise source locations and quote metadata.
//!
//! Double-quoted strings that contain variable references are emitted as
//! several adjacent tokens (text and variable segments) whose spans include the
//! surrounding quote characters. The parser merges adjacent segments back into
//! one word, preserving the single-argument boundary.

/// A one-based position in the original input.
///
/// Columns count Unicode scalar values, not bytes, so error positions remain
/// intuitive for non-ASCII input. `offset` is a zero-based byte offset and is
/// useful for checking whether two tokens are directly adjacent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    offset: usize,
    line: usize,
    column: usize,
}

impl Position {
    /// Creates a position from a byte offset and one-based line/column.
    #[must_use]
    pub const fn new(offset: usize, line: usize, column: usize) -> Self {
        Self {
            offset,
            line,
            column,
        }
    }

    /// The position at the beginning of an input stream.
    #[must_use]
    pub const fn start() -> Self {
        Self::new(0, 1, 1)
    }

    /// Zero-based byte offset into the original input.
    #[must_use]
    pub const fn offset(self) -> usize {
        self.offset
    }

    /// One-based line number.
    #[must_use]
    pub const fn line(self) -> usize {
        self.line
    }

    /// One-based Unicode-scalar column.
    #[must_use]
    pub const fn column(self) -> usize {
        self.column
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::start()
    }
}

/// A half-open source range: `start` is inclusive and `end` is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    start: Position,
    end: Position,
}

impl Span {
    /// Creates a span from its start and end positions.
    #[must_use]
    pub const fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Creates a zero-width span at a single position.
    #[must_use]
    pub const fn empty(position: Position) -> Self {
        Self::new(position, position)
    }

    /// The first position covered by this span.
    #[must_use]
    pub const fn start(self) -> Position {
        self.start
    }

    /// The first position after this span.
    #[must_use]
    pub const fn end(self) -> Position {
        self.end
    }

    /// Returns `true` when the span covers no input.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start.offset == self.end.offset
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::empty(Position::start())
    }
}

/// The style of quote used to produce a string token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuoteKind {
    /// A single-quoted string: `'text'`.
    Single,
    /// A double-quoted string: `"text"`.
    Double,
}

/// Special variable references that do not have an ordinary name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialVariable {
    /// `$?`: exit status of the previous command or command chain.
    LastStatus,
    /// `$$`: process identifier of the shell.
    ProcessId,
}

/// Returns `true` when `character` can start a variable name.
///
/// Variable names begin with a letter or an underscore. Unicode letters are
/// accepted, matching the lexer's existing `$name` rules.
#[must_use]
pub fn is_variable_name_start(character: char) -> bool {
    character == '_' || character.is_alphabetic()
}

/// Returns `true` when `character` can continue a variable name.
///
/// Continuation characters are letters, digits, and underscores.
#[must_use]
pub fn is_variable_name_continue(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

/// Returns `true` when `name` is a complete valid variable name.
#[must_use]
pub fn is_valid_variable_name(name: &str) -> bool {
    let mut characters = name.chars();

    match characters.next() {
        Some(first) if is_variable_name_start(first) => characters.all(is_variable_name_continue),
        _ => false,
    }
}

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
    /// Error-output redirection: `2>`.
    ErrorRedirect,
    /// Error-output append redirection: `2>>`.
    ErrorAppendRedirect,
    /// Input redirection: `<`.
    InputRedirect,
    /// Command separator: `;`.
    Semicolon,
    /// Assignment operator: `=`.
    Equals,
    /// Logical AND: `&&`.
    And,
    /// Logical OR: `||`.
    Or,
    /// Opening grouping parenthesis: `(`.
    LeftParenthesis,
    /// Closing grouping parenthesis: `)`.
    RightParenthesis,
    /// A named variable reference such as `$name` or `${name}`.
    Variable(String),
    /// A special variable reference such as `$?` or `$$`.
    SpecialVariable(SpecialVariable),
    /// A physical newline (`\n`, `\r\n`, or a lone `\r`).
    Newline,
    /// The end of the input stream.
    ///
    /// This token is always appended, including for empty input.
    EndOfInput,
    /// Opening brace: `{`.
    LeftBrace,
    /// Closing brace: `}`.
    RightBrace,
    /// Comma: `,`.
    Comma,
    /// Keyword: `if`.
    If,
    /// Keyword: `else`.
    Else,
    /// Keyword: `while`.
    While,
    /// Keyword: `for`.
    For,
    /// Keyword: `in`.
    In,
    /// Keyword: `function`.
    Function,
    /// Keyword: `return`.
    Return,
    /// Keyword: `break`.
    Break,
    /// Keyword: `continue`.
    Continue,
    /// Keyword: `run`.
    Run,
    /// Range operator: `..`.
    Range,
    /// Background execution operator: `&`.
    Background,
}

/// A [`Token`] together with its source span and quote metadata.
///
/// The parser uses this richer representation. The original [`Token`] stream
/// produced by [`crate::lex`] remains available for callers that do not need
/// locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedToken {
    /// The token value.
    pub token: Token,
    /// The source range that produced the token.
    pub span: Span,
    /// Quote style for [`Token::String`], or `None` for every other token.
    pub quote: Option<QuoteKind>,
}

impl SpannedToken {
    /// Creates a token with location and quote metadata.
    #[must_use]
    pub const fn new(token: Token, span: Span, quote: Option<QuoteKind>) -> Self {
        Self { token, span, quote }
    }
}
