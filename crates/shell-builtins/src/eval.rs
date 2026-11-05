//! AST evaluator that connects parsed commands to built-ins and external
//! programs.
//!
//! The evaluator walks the Phase 4 AST directly. It supports command lists
//! (`;` and newlines), groups, short-circuiting `&&`/`||`, pipelines, and
//! redirections. Every simple command is checked against the native registry
//! first; only names that are not built-ins are handed to the process executor
//! through the pipeline engine.

use crate::expand::expand_assignment_value;
use crate::outcome::{CommandAction, CommandOutcome};
use crate::pipeline::{self, PipelineOutcome, Stage};
use crate::process::ProcessOptions;
use crate::registry::BuiltinRegistry;
use shell_core::{Assignment, Ast, Command, Expression, Group, List, LogicalOperator, Pipeline};
use shell_runtime::Session;

pub(crate) fn evaluate(
    registry: &BuiltinRegistry,
    history: &[String],
    session: &mut Session,
    ast: &Ast,
    options: &ProcessOptions,
) -> CommandOutcome {
    let mut evaluator = Evaluator {
        session,
        registry,
        history,
        options,
        output: String::new(),
        action: CommandAction::Continue,
        success: true,
        exit_code: None,
        stopped: false,
    };

    evaluator.run_list(&ast.list);
    evaluator.finish()
}

struct Evaluator<'a> {
    session: &'a mut Session,
    registry: &'a BuiltinRegistry,
    history: &'a [String],
    options: &'a ProcessOptions,
    output: String,
    action: CommandAction,
    success: bool,
    exit_code: Option<i32>,
    stopped: bool,
}

impl Evaluator<'_> {
    fn run_list(&mut self, list: &List) -> bool {
        let mut status = true;

        for item in &list.items {
            if self.stopped {
                break;
            }

            status = self.run_expression(&item.expression);
            self.success = status;
        }

        status
    }

    fn run_expression(&mut self, expression: &Expression) -> bool {
        match expression {
            Expression::Command(command) => self.run_command(command),
            Expression::Group(group) => self.run_group(group),
            Expression::Pipeline(pipeline) => self.run_pipeline(pipeline),
            Expression::AndOr(expression) => {
                let left = self.run_expression(&expression.left);

                if self.stopped {
                    return left;
                }

                let should_run_right = match expression.operator {
                    LogicalOperator::And => left,
                    LogicalOperator::Or => !left,
                };

                if should_run_right {
                    self.run_expression(&expression.right)
                } else {
                    left
                }
            }
            Expression::Assignment(assignment) => self.run_assignment(assignment),
        }
    }

    fn run_assignment(&mut self, assignment: &Assignment) -> bool {
        let value = expand_assignment_value(assignment, self.session);
        self.session.set_variable(&assignment.name, value, false);
        self.session.set_last_status(0);
        true
    }

    fn run_group(&mut self, group: &Group) -> bool {
        self.run_list(&group.list)
    }

    fn run_command(&mut self, command: &Command) -> bool {
        let outcome = pipeline::execute(
            &[Stage::Command(command)],
            self.session,
            self.registry,
            self.history,
            self.options,
            true,
        );

        self.apply(outcome)
    }

    fn run_pipeline(&mut self, pipeline: &Pipeline) -> bool {
        let stages: Vec<Stage<'_>> = pipeline.stages.iter().map(Stage::from).collect();

        // Built-ins inside a real pipeline run in a child-like role; their
        // `exit` and `clear` actions must not control the parent shell.
        let outcome = pipeline::execute(
            &stages,
            self.session,
            self.registry,
            self.history,
            self.options,
            false,
        );

        self.apply(outcome)
    }

    fn apply(&mut self, outcome: PipelineOutcome) -> bool {
        self.append(&outcome.stdout);
        self.append(&outcome.stderr);

        if outcome.stopped {
            self.action = CommandAction::Exit;
            self.stopped = true;
        } else if outcome.action == CommandAction::ClearScreen && self.action != CommandAction::Exit
        {
            self.action = CommandAction::ClearScreen;
        }

        // Update $? from the pipeline outcome
        if let Some(code) = outcome.exit_code {
            self.session.set_last_status(code);
        } else if outcome.success {
            self.session.set_last_status(0);
        } else {
            self.session.set_last_status(1);
        }

        self.exit_code = outcome.exit_code;
        self.success = outcome.success;
        outcome.success
    }

    fn append(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }

        self.output.push_str(text);
    }

    fn finish(self) -> CommandOutcome {
        let output = if self.output.is_empty() {
            None
        } else {
            Some(self.output)
        };

        CommandOutcome::with_status(self.action, output, self.success, self.exit_code)
    }
}
