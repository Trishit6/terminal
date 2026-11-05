//! Custom parser and abstract syntax tree for the shell language.
//!
//! The parser consumes [`SpannedToken`](crate::lexer::SpannedToken) values from
//! the Phase 3 lexer and produces an [`Ast`]. It performs no execution,
//! expansion, or I/O.
//!
//! # Grammar
//!
//! The language is described by the following grammar. `NEWLINE` and `;` are
//! list separators; blank lines and trailing separators are accepted.
//!
//! ```text
//! program        := list EOF
//! list           := (NEWLINE)* expression (separator expression)* separator?
//! separator      := ';' | NEWLINE
//!
//! expression     := operand (('&&' | '||') operand)*
//!                |  if_else
//!                |  while_loop
//!                |  for_loop
//!                |  function_def
//!                |  return_stmt
//!                |  break_stmt
//!                |  continue_stmt
//!                |  block
//! operand        := assignment | pipeline
//! assignment     := NAME '=' word?
//! pipeline       := stage ('|' stage)*
//! stage          := simple_command | group
//!
//! if_else        := 'if' expression '{' list '}' ('else' '{' list '}')?
//! while_loop     := 'while' expression '{' list '}'
//! for_loop       := 'for' NAME 'in' expression '{' list '}'
//! function_def   := 'function' NAME '(' parameter_list? ')' '{' list '}'
//! parameter_list := NAME (',' NAME)*
//! return_stmt    := 'return' expression?
//! break_stmt     := 'break'
//! continue_stmt  := 'continue'
//! block          := '{' list '}'
//!
//! group          := '(' list ')'
//! simple_command := command_element+
//! command_element:= word | redirection
//! redirection    := ('<' | '>' | '>>' | '2>' | '2>>') word
//! word           := WORD | STRING | VARIABLE | SPECIAL | '=' | adjacent word segments
//! ```
//!
//! Newlines may also appear immediately after `|`, `&&`, `||`, `in`, or `{`; they are
//! treated as line continuations there rather than as list separators.
//!
//! # Precedence and associativity
//!
//! From lowest to highest precedence:
//!
//! 1. List sequencing (`;` and newlines).
//! 2. Logical AND and OR (`&&`, `||`), equal precedence and left-associative.
//! 3. Pipelines (`|`), left-associative.
//! 4. Simple commands, redirections, and grouped expressions.
//! 5. Control flow constructs (`if`, `while`, `for`, `function`).
//!
//! Because `&&` and `||` have equal precedence, `a && b || c` parses as
//! `((a && b) || c)`, matching the left-to-right short-circuit behavior of
//! POSIX-style shells.
//!
//! A group is a complete list inside parentheses and may be used anywhere a
//! pipeline stage may appear, including in pipelines such as
//! `(echo a ; echo b) | sort`.
//!
//! # Word concatenation
//!
//! Adjacent lexical segments are combined into one [`Word`]. For example,
//! `pre$name"post"` becomes a single word with three [`WordPart`] values. This
//! keeps quote and variable metadata available for a future evaluator.
//!
//! # Example
//!
//! ```
//! use shell_core::{lex_spanned, parse, Expression};
//!
//! let tokens = lex_spanned("echo \"Hello World\"").expect("valid input");
//! let ast = parse(&tokens).expect("valid syntax");
//!
//! assert_eq!(ast.list.items.len(), 1);
//! assert!(matches!(
//!     ast.list.items[0].expression,
//!     Expression::Command(_)
//! ));
//! ```

mod ast;
mod error;
mod parse;

pub use ast::{
    AndOr, Assignment, Ast, Background, Block, Break, Command, Continue, Expression, ForLoop,
    FunctionDef, Group, IfElse, List, ListItem, LogicalOperator, Pipeline, PipelineStage,
    Redirection, RedirectionKind, Return, Separator, WhileLoop, Word, WordPart,
};
pub use error::{ParseError, ParseErrorKind};
pub use parse::parse;

#[cfg(test)]
mod tests;
