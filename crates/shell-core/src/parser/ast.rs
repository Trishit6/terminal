//! Abstract syntax tree for the custom shell language.
//!
//! The AST preserves the structure of command lists, logical operators,
//! pipelines, redirections, grouped expressions, word quoting, variables, and
//! source spans. It contains no evaluation logic.
//!
//! Words are represented as a sequence of [`WordPart`] values. Adjacent lexical
//! segments such as `pre$name"post"` form a single word so later phases can
//! perform shell-style concatenation and variable expansion without guessing
//! where the original quotes were.

use crate::lexer::{Span, SpecialVariable};

/// A parsed shell program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ast {
    /// Top-level command list.
    pub list: List,
    /// Source range covered by the program.
    pub span: Span,
}

impl Ast {
    /// Creates a parsed program.
    #[must_use]
    pub const fn new(list: List, span: Span) -> Self {
        Self { list, span }
    }

    /// Returns `true` when the program contains no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

/// A sequence of expressions separated by `;` or physical newlines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List {
    /// Expressions in source order.
    pub items: Vec<ListItem>,
    /// Source range covered by the list.
    pub span: Span,
}

impl List {
    /// Creates a command list.
    #[must_use]
    pub const fn new(items: Vec<ListItem>, span: Span) -> Self {
        Self { items, span }
    }

    /// Returns `true` when no expressions are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Number of expressions in the list.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

/// One expression in a [`List`] and the separator that follows it, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    /// The parsed expression.
    pub expression: Expression,
    /// Separator following the expression, including a trailing separator.
    pub separator: Option<Separator>,
}

/// A command-list separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Separator {
    /// A semicolon: `;`.
    Semicolon,
    /// A physical newline.
    Newline,
    /// Background execution: `&`.
    Background,
}

/// A shell expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    /// A simple command.
    Command(Command),
    /// Two or more commands (or groups) connected by pipes.
    Pipeline(Pipeline),
    /// A left-associative `&&`/`||` expression.
    AndOr(Box<AndOr>),
    /// A parenthesized command list.
    Group(Group),
    /// A variable assignment such as `name = value`.
    Assignment(Assignment),
    /// An if-else expression.
    IfElse(IfElse),
    /// A while loop expression.
    WhileLoop(WhileLoop),
    /// A for loop expression.
    ForLoop(ForLoop),
    /// A function definition.
    FunctionDef(FunctionDef),
    /// A return statement.
    Return(Return),
    /// A break statement.
    Break(Break),
    /// A continue statement.
    Continue(Continue),
    /// A block expression (for function bodies, loop bodies, etc.).
    Block(Block),
    /// A background execution expression.
    Background(Background),
}

impl Expression {
    /// Source range covered by this expression.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Command(command) => command.span,
            Self::Pipeline(pipeline) => pipeline.span,
            Self::AndOr(expression) => expression.span,
            Self::Group(group) => group.span,
            Self::Assignment(assignment) => assignment.span,
            Self::IfElse(if_else) => if_else.span,
            Self::WhileLoop(while_loop) => while_loop.span,
            Self::ForLoop(for_loop) => for_loop.span,
            Self::FunctionDef(func_def) => func_def.span,
            Self::Return(ret) => ret.span,
            Self::Break(brk) => brk.span,
            Self::Continue(cont) => cont.span,
            Self::Block(block) => block.span,
            Self::Background(bg) => bg.span,
        }
    }
}

/// A shell variable assignment.
///
/// The value is a single [`Word`], so quoted values keep their spaces as one
/// value and variable references inside the value retain their quote metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    /// Variable name.
    pub name: String,
    /// Source range of the name.
    pub name_span: Span,
    /// Assignment value.
    pub value: Word,
    /// Source range covering the name, `=`, and value.
    pub span: Span,
}

/// A left-associative logical expression.
///
/// For `a && b || c`, the tree is `((a && b) || c)` because `&&` and `||` have
/// equal precedence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndOr {
    /// Left-hand expression.
    pub left: Expression,
    /// Logical operator.
    pub operator: LogicalOperator,
    /// Right-hand expression.
    pub right: Expression,
    /// Source range covered by the whole expression.
    pub span: Span,
}

/// Logical command-chaining operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalOperator {
    /// `&&`: evaluate the right side only when the left side succeeds.
    And,
    /// `||`: evaluate the right side only when the left side fails.
    Or,
}

/// Commands connected by one or more pipe operators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    /// Pipeline stages in source order.
    pub stages: Vec<PipelineStage>,
    /// Source range covered by the pipeline.
    pub span: Span,
}

/// A single pipeline stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineStage {
    /// A simple command.
    Command(Command),
    /// A parenthesized command list used as a pipeline stage.
    Group(Group),
}

impl PipelineStage {
    /// Source range covered by this stage.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Command(command) => command.span,
            Self::Group(group) => group.span,
        }
    }

    pub(crate) fn into_expression(self) -> Expression {
        match self {
            Self::Command(command) => Expression::Command(command),
            Self::Group(group) => Expression::Group(group),
        }
    }
}

/// A simple command: a name, arguments, and redirections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// Command name (the first shell word).
    pub name: Word,
    /// Arguments following the command name.
    pub arguments: Vec<Word>,
    /// Redirections attached to the command, in source order.
    pub redirections: Vec<Redirection>,
    /// Source range covered by the command.
    pub span: Span,
}

impl Command {
    /// Source range covered by this command.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// A file redirection attached to a simple command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirection {
    /// Redirection operator.
    pub kind: RedirectionKind,
    /// Target word, retaining quote and variable metadata.
    pub target: Word,
    /// Source range covering the operator and target.
    pub span: Span,
    /// Source range of the operator itself.
    pub operator_span: Span,
}

/// Supported redirection operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedirectionKind {
    /// Input redirection: `<`.
    Input,
    /// Output redirection: `>`.
    Output,
    /// Append redirection: `>>`.
    Append,
    /// Error-output redirection: `2>`.
    Error,
    /// Error-output append redirection: `2>>`.
    ErrorAppend,
}

/// A parenthesized command list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// Commands inside the parentheses.
    pub list: List,
    /// Source range including the opening and closing parentheses.
    pub span: Span,
}

/// A block expression (for function bodies, loop bodies, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// Statements in the block.
    pub list: List,
    /// Source range including the opening and closing braces.
    pub span: Span,
}

/// A background execution expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Background {
    /// The expression to run in the background.
    pub expression: Box<Expression>,
    /// Source range covered by the background expression.
    pub span: Span,
}

/// An if-else expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfElse {
    /// The condition expression.
    pub condition: Box<Expression>,
    /// The then branch.
    pub then_branch: Block,
    /// The else branch (if present).
    pub else_branch: Option<Block>,
    /// Source range covered by the whole expression.
    pub span: Span,
}

/// A while loop expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhileLoop {
    /// The condition expression.
    pub condition: Box<Expression>,
    /// The loop body.
    pub body: Block,
    /// Source range covered by the whole expression.
    pub span: Span,
}

/// A for loop expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForLoop {
    /// The loop variable name.
    pub variable: String,
    /// The iterable expression (e.g., a range or list).
    pub iterable: Box<Expression>,
    /// The loop body.
    pub body: Block,
    /// Source range covered by the whole expression.
    pub span: Span,
}

/// A function definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDef {
    /// Function name.
    pub name: String,
    /// Parameter names.
    pub parameters: Vec<String>,
    /// Function body.
    pub body: Block,
    /// Source range covered by the whole definition.
    pub span: Span,
}

/// A return statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Return {
    /// Optional return value expression.
    pub value: Option<Box<Expression>>,
    /// Source range covered by the return statement.
    pub span: Span,
}

/// A break statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Break {
    /// Source range covered by the break statement.
    pub span: Span,
}

/// A continue statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Continue {
    /// Source range covered by the continue statement.
    pub span: Span,
}

/// A shell word, made of one or more adjacent lexical segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    /// Word segments in source order.
    pub parts: Vec<WordPart>,
    /// Source range covering every part.
    pub span: Span,
}

impl Word {
    /// Creates a word from its parts and source range.
    #[must_use]
    pub const fn new(parts: Vec<WordPart>, span: Span) -> Self {
        Self { parts, span }
    }

    /// Source range covered by this word.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Returns `true` when the word contains at least one variable reference.
    #[must_use]
    pub fn has_variable(&self) -> bool {
        self.parts
            .iter()
            .any(|part| matches!(part, WordPart::Variable { .. } | WordPart::Special { .. }))
    }

    /// Returns the literal value of a word with no variable references.
    ///
    /// Quoted and unquoted text are concatenated exactly as the lexer decoded
    /// them. No expansion or globbing is performed.
    #[must_use]
    pub fn literal_text(&self) -> Option<String> {
        let mut text = String::new();

        for part in &self.parts {
            match part {
                WordPart::Unquoted {
                    text: part_text, ..
                }
                | WordPart::SingleQuoted {
                    text: part_text, ..
                }
                | WordPart::DoubleQuoted {
                    text: part_text, ..
                } => text.push_str(part_text),
                WordPart::Variable { .. } | WordPart::Special { .. } => return None,
            }
        }

        Some(text)
    }
}

/// One lexical segment of a shell word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordPart {
    /// Unquoted text.
    Unquoted {
        /// Decoded text.
        text: String,
        /// Source range of this segment.
        span: Span,
    },
    /// Single-quoted text.
    SingleQuoted {
        /// Decoded text, without the quotes.
        text: String,
        /// Source range including the quotes.
        span: Span,
    },
    /// Double-quoted text.
    DoubleQuoted {
        /// Decoded text, without the quotes.
        text: String,
        /// Source range including the quotes.
        span: Span,
    },
    /// A variable reference such as `$name`.
    Variable {
        /// Variable name without the leading `$`.
        name: String,
        /// Source range including the `$`.
        span: Span,
    },
    /// A special variable reference such as `$?` or `$$`.
    Special {
        /// Which special variable was referenced.
        kind: SpecialVariable,
        /// Source range including the `$`.
        span: Span,
    },
}

impl WordPart {
    /// Source range covered by this word segment.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Unquoted { span, .. }
            | Self::SingleQuoted { span, .. }
            | Self::DoubleQuoted { span, .. }
            | Self::Variable { span, .. }
            | Self::Special { span, .. } => *span,
        }
    }
}
