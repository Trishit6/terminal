//! Deterministic, UTF-8-safe scanner for the custom shell language.

use super::error::{LexerError, LexerErrorKind};
use super::token::Token;

/// Converts raw shell input into tokens.
///
/// The returned vector always ends with [`Token::EndOfInput`]. Whitespace is
/// skipped, but physical newlines are emitted as [`Token::Newline`].
///
/// # Errors
///
/// Returns a [`LexerError`] for unterminated quotes, invalid or dangling
/// escapes, invalid variable references, and rejected characters.
pub fn lex(input: &str) -> Result<Vec<Token>, LexerError> {
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

    fn tokenize(mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while let Some(character) = self.peek() {
            if character.is_whitespace() {
                if character == '\n' || character == '\r' {
                    self.consume_newline();
                    tokens.push(Token::Newline);
                } else {
                    self.bump();
                }
                continue;
            }

            match character {
                '|' => {
                    self.bump();
                    if self.peek() == Some('|') {
                        self.bump();
                        tokens.push(Token::Or);
                    } else {
                        tokens.push(Token::Pipe);
                    }
                }
                '&' => {
                    let (line, column) = self.position();
                    self.bump();
                    if self.peek() == Some('&') {
                        self.bump();
                        tokens.push(Token::And);
                    } else {
                        return Err(LexerError::new(
                            LexerErrorKind::UnexpectedCharacter { character: '&' },
                            line,
                            column,
                        ));
                    }
                }
                '>' => {
                    self.bump();
                    if self.peek() == Some('>') {
                        self.bump();
                        tokens.push(Token::AppendRedirect);
                    } else {
                        tokens.push(Token::OutputRedirect);
                    }
                }
                '<' => {
                    self.bump();
                    tokens.push(Token::InputRedirect);
                }
                ';' => {
                    self.bump();
                    tokens.push(Token::Semicolon);
                }
                '(' => {
                    self.bump();
                    tokens.push(Token::LeftParenthesis);
                }
                ')' => {
                    self.bump();
                    tokens.push(Token::RightParenthesis);
                }
                '\'' => {
                    let (line, column) = self.position();
                    tokens.push(self.lex_string('\'', line, column)?);
                }
                '"' => {
                    let (line, column) = self.position();
                    tokens.push(self.lex_string('"', line, column)?);
                }
                '$' => tokens.push(self.lex_variable()?),
                '\\' => tokens.push(self.lex_word()?),
                _ if character.is_control() => {
                    let (line, column) = self.position();
                    return Err(LexerError::new(
                        LexerErrorKind::UnexpectedCharacter { character },
                        line,
                        column,
                    ));
                }
                _ => tokens.push(self.lex_word()?),
            }
        }

        tokens.push(Token::EndOfInput);
        Ok(tokens)
    }

    fn lex_word(&mut self) -> Result<Token, LexerError> {
        let mut value = String::new();

        while let Some(character) = self.peek() {
            if character.is_whitespace()
                || matches!(
                    character,
                    '|' | '&' | '>' | '<' | ';' | '(' | ')' | '\'' | '"' | '$'
                )
            {
                break;
            }

            match character {
                '\\' => {
                    let (line, column) = self.position();
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
                    let (line, column) = self.position();
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

        Ok(Token::Word(value))
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
                '\\' => {
                    let (line, column) = self.position();
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

    fn lex_variable(&mut self) -> Result<Token, LexerError> {
        let (line, column) = self.position();
        self.bump();

        let Some(first) = self.peek() else {
            return Err(LexerError::new(
                LexerErrorKind::InvalidVariable,
                line,
                column,
            ));
        };

        if first != '_' && !first.is_alphabetic() {
            return Err(LexerError::new(
                LexerErrorKind::InvalidVariable,
                line,
                column,
            ));
        }

        let mut name = String::new();
        while let Some(character) = self.peek() {
            if character == '_' || character.is_alphanumeric() {
                self.bump();
                name.push(character);
            } else {
                break;
            }
        }

        Ok(Token::Variable(name))
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
            '|' | '&' | '>' | '<' | ';' | '(' | ')' => Ok(character),
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

    const fn position(&self) -> (usize, usize) {
        (self.line, self.column)
    }
}
