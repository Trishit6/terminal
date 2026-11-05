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
use shell_core::{
    Assignment, Ast, Background, Block, Break, Command, Continue, Expression, ForLoop, FunctionDef,
    Group, IfElse, List, LogicalOperator, Pipeline, Return, Separator, WhileLoop,
};
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

            // Handle background separator - return immediately without waiting
            if matches!(item.separator, Some(Separator::Background)) {
                // Background execution - return immediately without waiting for completion
                // The background job has already been started in run_background
                return true;
            }

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
            Expression::IfElse(if_else) => self.run_if_else(if_else),
            Expression::WhileLoop(while_loop) => self.run_while_loop(while_loop),
            Expression::ForLoop(for_loop) => self.run_for_loop(for_loop),
            Expression::FunctionDef(func_def) => self.run_function_def(func_def),
            Expression::Return(ret) => self.run_return(ret),
            Expression::Break(brk) => self.run_break(brk),
            Expression::Continue(cont) => self.run_continue(cont),
            Expression::Block(block) => self.run_block(block),
            Expression::Background(bg) => self.run_background(bg),
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

    fn run_block(&mut self, block: &Block) -> bool {
        self.run_list(&block.list)
    }

    fn run_background(&mut self, bg: &Background) -> bool {
        // For background execution, we run the expression but don't wait for it
        // In a full implementation, we'd track the job and return immediately
        // For now, we run it but don't wait (simulate background by not blocking)
        // In a real implementation, we'd track the job ID and process
        let _ = self.run_expression(&bg.expression);
        // Background jobs always return success immediately
        self.session.set_last_status(0);
        true
    }

    fn run_if_else(&mut self, if_else: &IfElse) -> bool {
        let condition_result = self.eval_condition(&if_else.condition);
        if condition_result {
            self.run_block(&if_else.then_branch)
        } else if let Some(else_branch) = &if_else.else_branch {
            self.run_block(else_branch)
        } else {
            true
        }
    }

    fn run_while_loop(&mut self, while_loop: &WhileLoop) -> bool {
        loop {
            if self.stopped {
                return false;
            }
            let condition_result = self.eval_condition(&while_loop.condition);
            if !condition_result {
                return true;
            }
            let body_result = self.run_block(&while_loop.body);
            if self.stopped {
                return false;
            }
            if !body_result && matches!(self.action, CommandAction::Exit) {
                return false;
            }
        }
    }

    fn run_for_loop(&mut self, for_loop: &ForLoop) -> bool {
        // For now, we only support range iteration (e.g., 1..5)
        // In a full implementation, we'd evaluate the iterable and iterate over it
        // For now, we'll just run the body once as a placeholder
        self.run_block(&for_loop.body)
    }

    fn run_function_def(&mut self, func_def: &FunctionDef) -> bool {
        // Store the function definition in the session for later calls
        self.session
            .set_function(func_def.name.clone(), func_def.clone());
        self.session.set_last_status(0);
        true
    }

    fn run_return(&mut self, ret: &Return) -> bool {
        // Set the return value in the session
        if let Some(value) = &ret.value {
            // Evaluate the return value
            let evaluated = self.eval_expression(value);
            self.session.set_return_value(Some(evaluated));
        } else {
            self.session.set_return_value(None);
        }
        // Signal that a return occurred
        self.stopped = true;
        true
    }

    fn run_break(&mut self, _brk: &Break) -> bool {
        // Signal that a break occurred
        // In a real implementation, we'd need to track loop nesting
        self.stopped = true;
        true
    }

    fn run_continue(&mut self, _cont: &Continue) -> bool {
        // Signal that a continue occurred
        self.stopped = true;
        true
    }

    fn eval_condition(&mut self, expr: &Expression) -> bool {
        // For condition evaluation, we run the expression and check its success
        let _result = self.run_expression(expr);
        // A condition is true if the expression succeeds (exit code 0)
        self.session.last_status() == 0
    }

    fn eval_expression(&self, _expr: &Expression) -> String {
        // For now, just return a placeholder - in a full implementation,
        // this would evaluate the expression and return its string value
        // For now, we'll just return an empty string for unsupported expressions
        String::new()
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
