//! Recursive-descent parser for the custom shell language.

use super::ast::{
    AndOr, Assignment, Ast, Command, Expression, Group, List, ListItem, LogicalOperator, Pipeline,
    PipelineStage, Redirection, RedirectionKind, Separator, Word, WordPart,
};
use super::error::{ParseError, ParseErrorKind};
use crate::lexer::{Position, QuoteKind, Span, SpannedToken, Token, is_valid_variable_name};

/// Parses a stream of spanned tokens into an [`Ast`].
///
/// The token stream is normally produced by [`crate::lex_spanned`]. Empty
/// input, or a stream containing only [`Token::EndOfInput`], produces an empty
/// program. A missing end-of-input token is treated as the end of the stream.
///
/// # Errors
///
/// Returns a [`ParseError`] with a source span for malformed syntax, including
/// missing operands, unexpected operators, and unbalanced parentheses.
pub fn parse(tokens: &[SpannedToken]) -> Result<Ast, ParseError> {
    Parser::new(tokens).parse_ast()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    EndOfInput,
    RightParenthesis,
}

struct Parser<'tokens> {
    tokens: &'tokens [SpannedToken],
    index: usize,
}

impl<'tokens> Parser<'tokens> {
    const fn new(tokens: &'tokens [SpannedToken]) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse_ast(mut self) -> Result<Ast, ParseError> {
        let start = self.current_position();
        let list = self.parse_list(Stop::EndOfInput)?;

        if !self.at_end() {
            return Err(ParseError::new(
                ParseErrorKind::InvalidCommandStructure {
                    message: "unexpected token after the command list",
                },
                self.current_span(),
            ));
        }

        let end = self.current_position();
        Ok(Ast::new(list, Span::new(start, end)))
    }

    fn parse_list(&mut self, stop: Stop) -> Result<List, ParseError> {
        let start = self.current_position();
        let mut items = Vec::new();
        self.skip_newlines();

        loop {
            if self.at_stop(stop) {
                break;
            }

            if self.check(&Token::Semicolon) {
                return Err(ParseError::new(
                    ParseErrorKind::UnexpectedOperator { operator: ";" },
                    self.current_span(),
                ));
            }

            if self.check(&Token::RightParenthesis) {
                return Err(ParseError::new(
                    ParseErrorKind::UnexpectedClosingParenthesis,
                    self.current_span(),
                ));
            }

            let expression = self.parse_and_or()?;

            let separator = if self.check(&Token::Semicolon) {
                self.advance();
                Some(Separator::Semicolon)
            } else if self.check(&Token::Newline) {
                self.advance();
                Some(Separator::Newline)
            } else {
                None
            };

            items.push(ListItem {
                expression,
                separator,
            });

            if separator.is_some() {
                self.skip_newlines();
                continue;
            }

            if self.at_stop(stop) {
                break;
            }

            if self.check(&Token::RightParenthesis) {
                return Err(ParseError::new(
                    ParseErrorKind::UnexpectedClosingParenthesis,
                    self.current_span(),
                ));
            }

            return Err(ParseError::new(
                ParseErrorKind::InvalidCommandStructure {
                    message: "expected `;`, a newline, or end of input",
                },
                self.current_span(),
            ));
        }

        let end = items
            .last()
            .map_or(start, |item| item.expression.span().end());
        Ok(List::new(items, Span::new(start, end)))
    }

    fn parse_and_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_operand()?;

        while let Some(operator) = self.current_token().and_then(logical_operator) {
            let operator_span = self.current_span();
            self.advance();
            self.skip_newlines();

            if self.at_end() {
                return Err(ParseError::new(
                    ParseErrorKind::UnexpectedEndOfInput,
                    operator_span,
                ));
            }

            if !self.starts_operand() {
                let name = match operator {
                    LogicalOperator::And => "&&",
                    LogicalOperator::Or => "||",
                };
                return Err(ParseError::new(
                    ParseErrorKind::MissingOperand { operator: name },
                    operator_span,
                ));
            }

            let right = self.parse_operand()?;
            let span = Span::new(left.span().start(), right.span().end());
            left = Expression::AndOr(Box::new(AndOr {
                left,
                operator,
                right,
                span,
            }));
        }

        Ok(left)
    }

    fn parse_operand(&mut self) -> Result<Expression, ParseError> {
        if self.looks_like_assignment() {
            Ok(Expression::Assignment(self.parse_assignment()?))
        } else {
            self.parse_pipeline()
        }
    }

    fn looks_like_assignment(&self) -> bool {
        let Some(SpannedToken {
            token: Token::Word(name),
            ..
        }) = self.peek()
        else {
            return false;
        };

        is_valid_variable_name(name)
            && self
                .tokens
                .get(self.index + 1)
                .is_some_and(|token| matches!(token.token, Token::Equals))
    }

    fn parse_assignment(&mut self) -> Result<Assignment, ParseError> {
        let name_token = self
            .advance()
            .expect("assignment lookahead checked the name");
        let Token::Word(name) = &name_token.token else {
            unreachable!("assignment lookahead only matches word tokens");
        };

        let name = name.clone();
        let name_span = name_token.span;
        let equals_span = self.current_span();
        self.advance();

        let value = if self.current_token().is_some_and(is_word_token) {
            self.parse_word()?
        } else {
            Word::new(Vec::new(), Span::empty(equals_span.end()))
        };

        if self.current_token().is_some_and(is_word_token) {
            return Err(ParseError::new(
                ParseErrorKind::InvalidCommandStructure {
                    message: "assignment values with spaces must be quoted",
                },
                self.current_span(),
            ));
        }

        if self.current_token().and_then(redirection_kind).is_some() {
            return Err(ParseError::new(
                ParseErrorKind::InvalidCommandStructure {
                    message: "an assignment cannot have a redirection",
                },
                self.current_span(),
            ));
        }

        if self.check(&Token::Pipe) {
            return Err(ParseError::new(
                ParseErrorKind::InvalidCommandStructure {
                    message: "an assignment cannot be a pipeline stage",
                },
                self.current_span(),
            ));
        }

        if self.check(&Token::LeftParenthesis) {
            return Err(ParseError::new(
                ParseErrorKind::InvalidCommandStructure {
                    message: "assignment values must be words",
                },
                self.current_span(),
            ));
        }

        let end = if value.parts.is_empty() {
            equals_span.end()
        } else {
            value.span.end()
        };

        Ok(Assignment {
            name,
            name_span,
            value,
            span: Span::new(name_span.start(), end),
        })
    }

    fn parse_pipeline(&mut self) -> Result<Expression, ParseError> {
        let first = self.parse_pipeline_stage()?;

        if !self.check(&Token::Pipe) {
            return Ok(first.into_expression());
        }

        let first_span = first.span();
        let mut stages = vec![first];

        while self.check(&Token::Pipe) {
            let pipe_span = self.current_span();
            self.advance();
            self.skip_newlines();

            if !self.starts_pipeline_stage() {
                return Err(ParseError::new(
                    ParseErrorKind::MissingPipeOperand,
                    pipe_span,
                ));
            }

            stages.push(self.parse_pipeline_stage()?);
        }

        let end = stages
            .last()
            .map_or(first_span.end(), |stage| stage.span().end());
        let span = Span::new(first_span.start(), end);
        Ok(Expression::Pipeline(Pipeline { stages, span }))
    }

    fn parse_pipeline_stage(&mut self) -> Result<PipelineStage, ParseError> {
        if self.looks_like_assignment() {
            return Err(ParseError::new(
                ParseErrorKind::InvalidCommandStructure {
                    message: "an assignment cannot be used as a pipeline stage",
                },
                self.current_span(),
            ));
        }

        if self.check(&Token::LeftParenthesis) {
            Ok(PipelineStage::Group(self.parse_group()?))
        } else {
            Ok(PipelineStage::Command(self.parse_simple_command()?))
        }
    }

    fn parse_group(&mut self) -> Result<Group, ParseError> {
        let open_span = self.current_span();
        self.advance();

        let list = self.parse_list(Stop::RightParenthesis)?;

        if !self.check(&Token::RightParenthesis) {
            return Err(ParseError::new(
                ParseErrorKind::UnclosedParenthesis,
                open_span,
            ));
        }

        let close_span = self.current_span();
        self.advance();
        let group_span = Span::new(open_span.start(), close_span.end());

        if list.is_empty() {
            return Err(ParseError::new(ParseErrorKind::MissingCommand, group_span));
        }

        Ok(Group {
            list,
            span: group_span,
        })
    }

    fn parse_simple_command(&mut self) -> Result<Command, ParseError> {
        if let Some(operator) = self.current_token().and_then(unexpected_operator) {
            return Err(ParseError::new(
                ParseErrorKind::UnexpectedOperator { operator },
                self.current_span(),
            ));
        }

        let mut words = Vec::new();
        let mut redirections = Vec::new();
        let mut first_start = None;
        let mut last_end = None;

        loop {
            if self.current_token().is_some_and(is_word_token) {
                let word = self.parse_word()?;
                first_start.get_or_insert(word.span.start());
                last_end = Some(word.span.end());
                words.push(word);
                continue;
            }

            let Some(kind) = self.current_token().and_then(redirection_kind) else {
                break;
            };

            let redirection = self.parse_redirection(kind)?;
            first_start.get_or_insert(redirection.span.start());
            last_end = Some(redirection.span.end());
            redirections.push(redirection);
        }

        let span = match (first_start, last_end) {
            (Some(start), Some(end)) => Span::new(start, end),
            _ => self.current_span(),
        };

        if words.is_empty() {
            let kind = if redirections.is_empty() && self.at_end() {
                ParseErrorKind::UnexpectedEndOfInput
            } else {
                ParseErrorKind::MissingCommand
            };
            return Err(ParseError::new(kind, span));
        }

        let mut words = words.into_iter();
        let Some(name) = words.next() else {
            return Err(ParseError::new(ParseErrorKind::MissingCommand, span));
        };
        let arguments = words.collect();

        Ok(Command {
            name,
            arguments,
            redirections,
            span,
        })
    }

    fn parse_redirection(&mut self, kind: RedirectionKind) -> Result<Redirection, ParseError> {
        let operator_span = self.current_span();
        self.advance();

        if !self.current_token().is_some_and(is_word_token) {
            return Err(ParseError::new(
                ParseErrorKind::MissingRedirectionTarget,
                operator_span,
            ));
        }

        let target = self.parse_word()?;
        let span = Span::new(operator_span.start(), target.span.end());

        Ok(Redirection {
            kind,
            target,
            span,
            operator_span,
        })
    }

    fn parse_word(&mut self) -> Result<Word, ParseError> {
        let Some(first) = self.advance() else {
            return Err(ParseError::new(
                ParseErrorKind::UnexpectedEndOfInput,
                self.current_span(),
            ));
        };

        let start = first.span.start();
        let mut end = first.span.end();
        let Some(first_part) = word_part(first) else {
            return Err(ParseError::new(
                ParseErrorKind::InvalidCommandStructure {
                    message: "expected a word",
                },
                first.span,
            ));
        };
        let mut parts = vec![first_part];

        while let Some(next) = self.peek() {
            if !is_word_token(&next.token) || end.offset() != next.span.start().offset() {
                break;
            }

            let next_span = next.span;
            let Some(part) = word_part(next) else {
                return Err(ParseError::new(
                    ParseErrorKind::InvalidCommandStructure {
                        message: "expected a word",
                    },
                    next_span,
                ));
            };

            self.advance();
            parts.push(part);
            end = next_span.end();
        }

        Ok(Word::new(parts, Span::new(start, end)))
    }

    fn peek(&self) -> Option<&SpannedToken> {
        self.tokens.get(self.index)
    }

    fn current_token(&self) -> Option<&Token> {
        self.peek().map(|token| &token.token)
    }

    fn current_span(&self) -> Span {
        self.peek()
            .map_or_else(|| Span::empty(Position::default()), |token| token.span)
    }

    fn current_position(&self) -> Position {
        self.current_span().start()
    }

    fn advance(&mut self) -> Option<&SpannedToken> {
        let token = self.tokens.get(self.index)?;
        self.index += 1;
        Some(token)
    }

    fn at_end(&self) -> bool {
        matches!(self.current_token(), None | Some(Token::EndOfInput))
    }

    fn check(&self, expected: &Token) -> bool {
        self.current_token() == Some(expected)
    }

    fn skip_newlines(&mut self) {
        while self.check(&Token::Newline) {
            self.advance();
        }
    }

    fn at_stop(&self, stop: Stop) -> bool {
        match stop {
            Stop::EndOfInput => self.at_end(),
            Stop::RightParenthesis => self.at_end() || self.check(&Token::RightParenthesis),
        }
    }

    fn starts_pipeline_stage(&self) -> bool {
        self.current_token().is_some_and(|token| {
            is_word_token(token)
                || matches!(token, Token::LeftParenthesis)
                || redirection_kind(token).is_some()
        })
    }

    fn starts_operand(&self) -> bool {
        self.looks_like_assignment() || self.starts_pipeline_stage()
    }
}

fn is_word_token(token: &Token) -> bool {
    matches!(
        token,
        Token::Word(_)
            | Token::String(_)
            | Token::Variable(_)
            | Token::SpecialVariable(_)
            | Token::Equals
    )
}

fn logical_operator(token: &Token) -> Option<LogicalOperator> {
    match token {
        Token::And => Some(LogicalOperator::And),
        Token::Or => Some(LogicalOperator::Or),
        _ => None,
    }
}

fn redirection_kind(token: &Token) -> Option<RedirectionKind> {
    match token {
        Token::InputRedirect => Some(RedirectionKind::Input),
        Token::OutputRedirect => Some(RedirectionKind::Output),
        Token::AppendRedirect => Some(RedirectionKind::Append),
        Token::ErrorRedirect => Some(RedirectionKind::Error),
        Token::ErrorAppendRedirect => Some(RedirectionKind::ErrorAppend),
        _ => None,
    }
}

fn unexpected_operator(token: &Token) -> Option<&'static str> {
    match token {
        Token::Pipe => Some("|"),
        Token::And => Some("&&"),
        Token::Or => Some("||"),
        Token::Semicolon => Some(";"),
        _ => None,
    }
}

fn word_part(token: &SpannedToken) -> Option<WordPart> {
    let span = token.span;

    match &token.token {
        Token::Word(text) => Some(WordPart::Unquoted {
            text: text.clone(),
            span,
        }),
        Token::String(text) => Some(match token.quote {
            Some(QuoteKind::Single) => WordPart::SingleQuoted {
                text: text.clone(),
                span,
            },
            Some(QuoteKind::Double) => WordPart::DoubleQuoted {
                text: text.clone(),
                span,
            },
            None => WordPart::Unquoted {
                text: text.clone(),
                span,
            },
        }),
        Token::Variable(name) => Some(WordPart::Variable {
            name: name.clone(),
            span,
        }),
        Token::SpecialVariable(kind) => Some(WordPart::Special { kind: *kind, span }),
        Token::Equals => Some(WordPart::Unquoted {
            text: "=".to_owned(),
            span,
        }),
        _ => None,
    }
}
