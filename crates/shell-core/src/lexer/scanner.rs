//! Deterministic, UTF-8-safe scanner for the custom shell language.

use super::error::{LexerError, LexerErrorKind};
use super::token::{
    Position, QuoteKind, Span, SpannedToken, SpecialVariable, Token, is_variable_name_continue,
    is_variable_name_start,
};

/// Converts raw shell input into compact tokens.
///
/// The returned vector always ends with [`Token::EndOfInput`]. Whitespace is
/// skipped, but physical newlines are emitted as [`Token::Newline`].
///
/// Use [`lex_spanned`] when source locations or quote metadata are needed.
///
/// # Errors
///
/// Returns a [`LexerError`] for unterminated quotes, invalid or dangling
/// escapes, invalid variable references, and rejected characters.
pub fn lex(input: &str) -> Result<Vec<Token>, LexerError> {
    Ok(lex_spanned(input)?
        .into_iter()
        .map(|spanned| spanned.token)
        .collect())
}

/// Converts raw shell input into tokens with source spans and quote metadata.
///
/// The token values are identical to those produced by [`lex`]. The returned
/// vector always ends with a zero-width [`Token::EndOfInput`] token.
///
/// # Errors
///
/// Returns a [`LexerError`] for unterminated quotes, invalid or dangling
/// escapes, invalid variable references, and rejected characters.
pub fn lex_spanned(input: &str) -> Result<Vec<SpannedToken>, LexerError> {
    Lexer::new(input).tokenize()
}

struct Lexer<'a> {
    input: &'a str,
    offset: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    fn tokenize(mut self) -> Result<Vec<SpannedToken>, LexerError> {
        let mut tokens = Vec::new();

        while let Some(character) = self.peek() {
            if character.is_whitespace() {
                if character == '\n' || character == '\r' {
                    let start = self.position();
                    self.consume_newline();
                    let end = self.position();
                    tokens.push(SpannedToken::new(
                        Token::Newline,
                        Span::new(start, end),
                        None,
                    ));
                } else {
                    self.bump();
                }
                continue;
            }

            let start = self.position();
            let (token, quote) = match character {
                '|' => {
                    self.bump();
                    if self.peek() == Some('|') {
                        self.bump();
                        (Token::Or, None)
                    } else {
                        (Token::Pipe, None)
                    }
                }
                '&' => {
                    self.bump();
                    if self.peek() == Some('&') {
                        self.bump();
                        (Token::And, None)
                    } else {
                        (Token::Background, None)
                    }
                }
                '>' => {
                    self.bump();
                    if self.peek() == Some('>') {
                        self.bump();
                        (Token::AppendRedirect, None)
                    } else {
                        (Token::OutputRedirect, None)
                    }
                }
                // `2>` and `2>>` are recognised only when the digit starts a
                // token. A digit embedded in a word (`file2>log`) stays part of
                // that word, matching standard shell behaviour.
                '2' if self.peek_second() == Some('>') => {
                    self.bump();
                    self.bump();
                    if self.peek() == Some('>') {
                        self.bump();
                        (Token::ErrorAppendRedirect, None)
                    } else {
                        (Token::ErrorRedirect, None)
                    }
                }
                '<' => {
                    self.bump();
                    (Token::InputRedirect, None)
                }
                ';' => {
                    self.bump();
                    (Token::Semicolon, None)
                }
                '(' => {
                    self.bump();
                    (Token::LeftParenthesis, None)
                }
                ')' => {
                    self.bump();
                    (Token::RightParenthesis, None)
                }
                '{' => {
                    self.bump();
                    (Token::LeftBrace, None)
                }
                '}' => {
                    self.bump();
                    (Token::RightBrace, None)
                }
                ',' => {
                    self.bump();
                    (Token::Comma, None)
                }
                '.' if self.peek_second() == Some('.') => {
                    self.bump();
                    self.bump();
                    (Token::Range, None)
                }
                // A single dot is part of a word, including a leading dot in
                // hidden-file names and relative paths such as `./file`.
                '.' => (self.lex_word()?, None),
                '\'' => {
                    let token = self.lex_string('\'', start.line(), start.column())?;
                    let end = self.position();
                    tokens.push(SpannedToken::new(
                        token,
                        Span::new(start, end),
                        Some(QuoteKind::Single),
                    ));
                    continue;
                }
                '"' => {
                    tokens.extend(self.lex_double_quoted(start)?);
                    continue;
                }
                '=' => {
                    self.bump();
                    (Token::Equals, None)
                }
                '$' => (self.lex_variable()?, None),
                '\\' => (self.lex_word()?, None),
                _ if character.is_control() => {
                    return Err(LexerError::new(
                        LexerErrorKind::UnexpectedCharacter { character },
                        start.line(),
                        start.column(),
                    ));
                }
                _ => (self.lex_word()?, None),
            };

            let end = self.position();
            tokens.push(SpannedToken::new(token, Span::new(start, end), quote));
        }

        let end = self.position();
        tokens.push(SpannedToken::new(Token::EndOfInput, Span::empty(end), None));
        Ok(tokens)
    }

    fn lex_word(&mut self) -> Result<Token, LexerError> {
        let mut value = String::new();

        while let Some(character) = self.peek() {
            if character.is_whitespace()
                || matches!(
                    character,
                    '|' | '&'
                        | '>'
                        | '<'
                        | ';'
                        | '('
                        | ')'
                        | '{'
                        | '}'
                        | ','
                        | '\''
                        | '"'
                        | '$'
                        | '='
                )
            {
                break;
            }

            match character {
                '\\' if looks_like_windows_path(&value) && !self.next_is_shell_escape() => {
                    // Preserve path separators in unquoted Windows paths such
                    // as `C:\temp` or `\\server\share`. Explicit shell escapes
                    // (spaces, quotes, operators) still take precedence.
                    let (line, column) = self.position_pair();
                    self.bump();
                    let Some(escaped) = self.peek() else {
                        return Err(LexerError::new(
                            LexerErrorKind::DanglingEscape,
                            line,
                            column,
                        ));
                    };
                    self.bump();
                    value.push('\\');
                    value.push(escaped);
                }
                '\\' => {
                    let (line, column) = self.position_pair();
                    self.bump();
                    let Some(escaped) = self.peek() else {
                        return Err(LexerError::new(
                            LexerErrorKind::DanglingEscape,
                            line,
                            column,
                        ));
                    };
                    self.bump();
                    value.push(self.decode_escape(escaped, line, column)?);
                }
                _ if character.is_control() => {
                    let (line, column) = self.position_pair();
                    return Err(LexerError::new(
                        LexerErrorKind::UnexpectedCharacter { character },
                        line,
                        column,
                    ));
                }
                _ => {
                    self.bump();
                    value.push(character);
                }
            }
        }

        // Check if the word is a keyword
        let token = match value.as_str() {
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "in" => Token::In,
            "function" => Token::Function,
            "return" => Token::Return,
            "break" => Token::Break,
            "continue" => Token::Continue,
            _ => Token::Word(value),
        };

        Ok(token)
    }

    fn lex_string(
        &mut self,
        quote: char,
        start_line: usize,
        start_column: usize,
    ) -> Result<Token, LexerError> {
        self.bump();
        let mut value = String::new();

        loop {
            let Some(character) = self.peek() else {
                let kind = if quote == '\'' {
                    LexerErrorKind::UnterminatedSingleQuote
                } else {
                    LexerErrorKind::UnterminatedDoubleQuote
                };
                return Err(LexerError::new(kind, start_line, start_column));
            };

            if character == quote {
                self.bump();
                return Ok(Token::String(value));
            }

            match character {
                '\\' if looks_like_windows_path(&value) && !self.next_is_shell_escape() => {
                    let (line, column) = self.position_pair();
                    self.bump();
                    let Some(escaped) = self.peek() else {
                        return Err(LexerError::new(
                            LexerErrorKind::DanglingEscape,
                            line,
                            column,
                        ));
                    };
                    self.bump();
                    value.push('\\');
                    value.push(escaped);
                }
                '\\' => {
                    let (line, column) = self.position_pair();
                    self.bump();
                    let Some(escaped) = self.peek() else {
                        return Err(LexerError::new(
                            LexerErrorKind::DanglingEscape,
                            line,
                            column,
                        ));
                    };
                    self.bump();
                    // Inside quotes, an unrecognised escape is a literal
                    // backslash. This is required for Windows paths such as
                    // "C:\Users\name" while preserving explicit escapes.
                    match self.decode_escape(escaped, line, column) {
                        Ok(decoded) => value.push(decoded),
                        Err(error)
                            if matches!(error.kind(), LexerErrorKind::InvalidEscape { .. }) =>
                        {
                            value.push('\\');
                            value.push(escaped);
                        }
                        Err(error) => return Err(error),
                    }
                }
                '\n' => {
                    self.bump();
                    value.push('\n');
                    self.line += 1;
                    self.column = 1;
                }
                '\r' => {
                    self.bump();
                    value.push('\r');
                    if self.peek() == Some('\n') {
                        self.bump();
                        value.push('\n');
                    }
                    self.line += 1;
                    self.column = 1;
                }
                _ => {
                    self.bump();
                    value.push(character);
                }
            }
        }
    }

    fn lex_double_quoted(
        &mut self,
        quote_start: Position,
    ) -> Result<Vec<SpannedToken>, LexerError> {
        self.bump();

        let mut pieces: Vec<(Token, Span, Option<QuoteKind>)> = Vec::new();
        let mut text = String::new();
        let mut text_start = self.position();

        loop {
            let Some(character) = self.peek() else {
                return Err(LexerError::new(
                    LexerErrorKind::UnterminatedDoubleQuote,
                    quote_start.line(),
                    quote_start.column(),
                ));
            };

            match character {
                '"' => {
                    self.bump();
                    let close_end = self.position();
                    push_double_text(&mut pieces, &mut text, text_start, close_end);

                    if pieces.is_empty() {
                        pieces.push((
                            Token::String(String::new()),
                            Span::new(quote_start, close_end),
                            Some(QuoteKind::Double),
                        ));
                    }

                    // Attach the quote characters to the outermost segments so
                    // adjacent parts of the same shell word remain contiguous.
                    if let Some((_, span, _)) = pieces.first_mut() {
                        *span = Span::new(quote_start, span.end());
                    }
                    if let Some((_, span, _)) = pieces.last_mut() {
                        *span = Span::new(span.start(), close_end);
                    }

                    return Ok(pieces
                        .into_iter()
                        .map(|(token, span, quote)| SpannedToken::new(token, span, quote))
                        .collect());
                }
                '$' => {
                    let variable_start = self.position();
                    push_double_text(&mut pieces, &mut text, text_start, variable_start);

                    let token = self.lex_variable()?;
                    let variable_end = self.position();
                    pieces.push((token, Span::new(variable_start, variable_end), None));
                    text_start = self.position();
                }
                '\\' if looks_like_windows_path(&text) && !self.next_is_shell_escape() => {
                    let (line, column) = self.position_pair();
                    self.bump();
                    let Some(escaped) = self.peek() else {
                        return Err(LexerError::new(
                            LexerErrorKind::DanglingEscape,
                            line,
                            column,
                        ));
                    };
                    self.bump();
                    text.push('\\');
                    text.push(escaped);
                }
                '\\' => {
                    let (line, column) = self.position_pair();
                    self.bump();
                    let Some(escaped) = self.peek() else {
                        return Err(LexerError::new(
                            LexerErrorKind::DanglingEscape,
                            line,
                            column,
                        ));
                    };
                    self.bump();
                    // Keep unknown escapes literal inside double quotes so
                    // Windows paths such as "C:\Users\name" survive lexing.
                    match self.decode_escape(escaped, line, column) {
                        Ok(decoded) => text.push(decoded),
                        Err(error)
                            if matches!(error.kind(), LexerErrorKind::InvalidEscape { .. }) =>
                        {
                            text.push('\\');
                            text.push(escaped);
                        }
                        Err(error) => return Err(error),
                    }
                }
                '\n' => {
                    self.bump();
                    text.push('\n');
                    self.line += 1;
                    self.column = 1;
                }
                '\r' => {
                    self.bump();
                    text.push('\r');
                    if self.peek() == Some('\n') {
                        self.bump();
                        text.push('\n');
                    }
                    self.line += 1;
                    self.column = 1;
                }
                _ => {
                    self.bump();
                    text.push(character);
                }
            }
        }
    }

    fn lex_variable(&mut self) -> Result<Token, LexerError> {
        let (line, column) = self.position_pair();
        self.bump();

        match self.peek() {
            Some('?') => {
                self.bump();
                return Ok(Token::SpecialVariable(SpecialVariable::LastStatus));
            }
            Some('$') => {
                self.bump();
                return Ok(Token::SpecialVariable(SpecialVariable::ProcessId));
            }
            Some('{') => return self.lex_braced_variable(line, column),
            _ => {}
        }

        let Some(first) = self.peek() else {
            return Err(LexerError::new(
                LexerErrorKind::InvalidVariable,
                line,
                column,
            ));
        };

        if !is_variable_name_start(first) {
            return Err(LexerError::new(
                LexerErrorKind::InvalidVariable,
                line,
                column,
            ));
        }

        let mut name = String::new();
        while let Some(character) = self.peek() {
            if is_variable_name_continue(character) {
                self.bump();
                name.push(character);
            } else {
                break;
            }
        }

        Ok(Token::Variable(name))
    }

    fn lex_braced_variable(&mut self, line: usize, column: usize) -> Result<Token, LexerError> {
        self.bump();

        let Some(first) = self.peek() else {
            return Err(LexerError::new(
                LexerErrorKind::UnterminatedBracedVariable,
                line,
                column,
            ));
        };

        if first == '}' {
            self.bump();
            return Err(LexerError::new(
                LexerErrorKind::EmptyBracedVariable,
                line,
                column,
            ));
        }

        if !is_variable_name_start(first) {
            return Err(LexerError::new(
                LexerErrorKind::InvalidBracedVariable,
                line,
                column,
            ));
        }

        let mut name = String::new();
        while let Some(character) = self.peek() {
            if is_variable_name_continue(character) {
                self.bump();
                name.push(character);
            } else {
                break;
            }
        }

        match self.peek() {
            Some('}') => {
                self.bump();
                Ok(Token::Variable(name))
            }
            None => Err(LexerError::new(
                LexerErrorKind::UnterminatedBracedVariable,
                line,
                column,
            )),
            Some(_) => Err(LexerError::new(
                LexerErrorKind::InvalidBracedVariable,
                line,
                column,
            )),
        }
    }

    /// Returns true when the next character has an explicit shell meaning
    /// after a backslash. Used to keep Windows path separators literal while
    /// still allowing `C:\Program\ Files` and escaped operators.
    fn next_is_shell_escape(&self) -> bool {
        matches!(
            self.peek_second(),
            Some('\\' | '"' | '\'' | '$' | ' ' | '|' | '&' | '>' | '<' | ';' | '(' | ')' | '=')
        )
    }

    fn decode_escape(
        &self,
        character: char,
        line: usize,
        column: usize,
    ) -> Result<char, LexerError> {
        match character {
            '\\' => Ok('\\'),
            '"' => Ok('"'),
            '\'' => Ok('\''),
            '$' => Ok('$'),
            'n' => Ok('\n'),
            't' => Ok('\t'),
            'r' => Ok('\r'),
            ' ' => Ok(' '),
            '|' | '&' | '>' | '<' | ';' | '(' | ')' | '=' => Ok(character),
            _ => Err(LexerError::new(
                LexerErrorKind::InvalidEscape { character },
                line,
                column,
            )),
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.offset..].chars().next()
    }

    fn peek_second(&self) -> Option<char> {
        self.input[self.offset..].chars().nth(1)
    }

    fn bump(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.offset += character.len_utf8();
        self.column += 1;
        Some(character)
    }

    fn consume_newline(&mut self) {
        if self.peek() == Some('\r') {
            self.bump();
            if self.peek() == Some('\n') {
                self.bump();
            }
        } else if self.peek() == Some('\n') {
            self.bump();
        } else {
            return;
        }

        self.line += 1;
        self.column = 1;
    }

    const fn position(&self) -> Position {
        Position::new(self.offset, self.line, self.column)
    }

    const fn position_pair(&self) -> (usize, usize) {
        (self.line, self.column)
    }
}

/// Heuristic for unquoted Windows paths. Once a word contains a drive prefix
/// (`C:`) or has started with a UNC separator, later backslashes are treated as
/// path separators rather than escapes.
fn looks_like_windows_path(value: &str) -> bool {
    if value.starts_with('\\') || value.contains(":\\") {
        return true;
    }

    let bytes = value.as_bytes();
    bytes.len() >= 2
        && bytes[bytes.len() - 1] == b':'
        && bytes[bytes.len() - 2].is_ascii_alphabetic()
}

fn push_double_text(
    pieces: &mut Vec<(Token, Span, Option<QuoteKind>)>,
    text: &mut String,
    start: Position,
    end: Position,
) {
    if text.is_empty() {
        return;
    }

    pieces.push((
        Token::String(std::mem::take(text)),
        Span::new(start, end),
        Some(QuoteKind::Double),
    ));
}

#[cfg(test)]
mod path_heuristic_tests {
    use super::looks_like_windows_path;

    #[test]
    fn drive_prefix_is_recognised() {
        assert!(looks_like_windows_path("C:"));
        assert!(looks_like_windows_path("--path=C:"));
        assert!(looks_like_windows_path(r"C:\Users"));
    }
}
