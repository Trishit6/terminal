use crate::expand::{expand_assignment_value, expand_word};
use crate::outcome::{CommandAction, CommandOutcome};
use crate::pipeline::{self, PipelineOutcome, Stage};
use crate::process::{ExternalCommand, ProcessOptions, StdioMode};
use crate::registry::BuiltinRegistry;
use shell_core::{
    Assignment, Ast, Background, Block, Break, Command, Continue, Expression, ForLoop, FunctionDef,
    Group, IfElse, List, LogicalOperator, Pipeline, PipelineStage, Return, Separator, WhileLoop,
};
use shell_runtime::Session;
use std::io::Write;
use std::process::Stdio;

const MAX_FUNCTION_DEPTH: usize = 128;

const MAX_LOOP_ITERATIONS: usize = 1_000_000;

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
        flow: Flow::Normal,
        return_value: None,
        call_depth: 0,
        loop_depth: 0,
    };

    evaluator.run_list(&ast.list);
    evaluator.finish()
}

pub(crate) fn evaluate_expression(
    registry: &BuiltinRegistry,
    history: &[String],
    session: &mut Session,
    expression: &Expression,
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
        flow: Flow::Normal,
        return_value: None,
        call_depth: 0,
        loop_depth: 0,
    };

    evaluator.run_expression(expression);
    evaluator.finish()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Normal,
    Break,
    Continue,
    Return,
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
    flow: Flow,
    return_value: Option<String>,
    call_depth: usize,
    loop_depth: usize,
}

impl Evaluator<'_> {
    fn run_list(&mut self, list: &List) -> bool {
        let mut status = true;

        for item in &list.items {
            if self.stopped || self.flow != Flow::Normal {
                break;
            }

            if item.separator == Some(Separator::Background) {
                let stopped = self.stopped;
                let action = self.action;
                let flow = self.flow;

                status = self.spawn_background(&item.expression);

                self.stopped = stopped;
                self.action = action;
                self.flow = flow;

                if !stopped {
                    self.session.set_last_status(0);
                }

                self.success = status;
                continue;
            }

            status = self.run_expression(&item.expression);

            if self.stopped || self.flow != Flow::Normal {
                break;
            }

            debug_assert!(
                matches!(
                    item.separator,
                    None | Some(Separator::Semicolon)
                        | Some(Separator::Newline)
                        | Some(Separator::Background)
                ),
                "the parser only produces known separators"
            );

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

                if self.stopped || self.flow != Flow::Normal {
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
        let stopped = self.stopped;
        let action = self.action;
        let flow = self.flow;
        let status = self.spawn_background(&bg.expression);
        self.stopped = stopped;
        self.action = action;
        self.flow = flow;

        if !stopped {
            self.session.set_last_status(0);
        }

        status
    }

    fn spawn_background(&mut self, expression: &Expression) -> bool {
        let label = self.expression_label(expression);

        if let Expression::Command(command) = expression {
            let name = expand_word(&command.name, self.session);

            if self.registry.get(&name).is_none()
                && self.session.get_function(&name).is_none()
                && command.redirections.is_empty()
            {
                let arguments: Vec<String> = command
                    .arguments
                    .iter()
                    .map(|argument| expand_word(argument, self.session))
                    .collect();
                let external = ExternalCommand::new(&name)
                    .args(arguments)
                    .current_dir(self.session.working_directory().to_path_buf())
                    .clear_env()
                    .envs(self.session.child_environment(self.options.environment()));

                return match external.spawn_with(Stdio::null(), Stdio::inherit(), Stdio::inherit())
                {
                    Ok(child) => {
                        self.session.add_external_job(label, child);
                        self.session.set_last_status(0);
                        true
                    }
                    Err(error) => self.fail(format!("{name}: {error}")),
                };
            }
        }

        let expression = expression.clone();
        let mut session = self.session.clone();
        let registry = self.registry.clone();
        let history = self.history.to_vec();
        let options = self.options.clone();
        let id = self.session.add_shell_job(label);
        let jobs = self.session.jobs();

        std::thread::spawn(move || {
            let outcome =
                evaluate_expression(&registry, &history, &mut session, &expression, &options);

            if options.stdio() == StdioMode::Inherit
                && let Some(output) = outcome.output()
            {
                let mut stdout = std::io::stdout().lock();
                let _ = stdout.write_all(output.as_bytes());
                if !output.ends_with('\n') {
                    let _ = stdout.write_all(b"\n");
                }
                let _ = stdout.flush();
            }

            if let Ok(mut jobs) = jobs.lock() {
                jobs.complete(id, outcome.success(), outcome.exit_code());
            }
        });

        self.session.set_last_status(0);
        true
    }

    fn expression_label(&self, expression: &Expression) -> String {
        match expression {
            Expression::Command(command) => {
                let mut words = Vec::with_capacity(command.arguments.len() + 1);
                words.push(expand_word(&command.name, self.session));
                words.extend(
                    command
                        .arguments
                        .iter()
                        .map(|argument| expand_word(argument, self.session)),
                );
                words.join(" ")
            }
            Expression::Pipeline(pipeline) => pipeline
                .stages
                .iter()
                .map(|stage| match stage {
                    PipelineStage::Command(command) => {
                        let mut words = Vec::with_capacity(command.arguments.len() + 1);
                        words.push(expand_word(&command.name, self.session));
                        words.extend(
                            command
                                .arguments
                                .iter()
                                .map(|argument| expand_word(argument, self.session)),
                        );
                        words.join(" ")
                    }
                    PipelineStage::Group(_) => "(...)".to_owned(),
                })
                .collect::<Vec<_>>()
                .join(" | "),
            _ => "shell expression".to_owned(),
        }
    }

    fn run_if_else(&mut self, if_else: &IfElse) -> bool {
        let condition_result = self.eval_condition(&if_else.condition);

        if self.stopped || self.flow == Flow::Return {
            return condition_result;
        }

        if condition_result {
            self.run_block(&if_else.then_branch)
        } else if let Some(else_branch) = &if_else.else_branch {
            self.run_block(else_branch)
        } else {
            true
        }
    }

    fn run_while_loop(&mut self, while_loop: &WhileLoop) -> bool {
        let mut last_status = true;
        let mut iterations = 0usize;

        loop {
            if self.stopped || self.flow == Flow::Return {
                return last_status;
            }

            let condition_result = self.eval_condition(&while_loop.condition);

            if self.stopped || self.flow == Flow::Return {
                return last_status;
            }

            if !condition_result {
                return last_status;
            }

            iterations += 1;
            if iterations > MAX_LOOP_ITERATIONS {
                return self.fail(format!(
                    "while: iteration limit of {MAX_LOOP_ITERATIONS} exceeded"
                ));
            }

            self.loop_depth += 1;
            last_status = self.run_block(&while_loop.body);
            self.loop_depth -= 1;

            match self.flow {
                Flow::Break => {
                    self.flow = Flow::Normal;
                    return true;
                }
                Flow::Continue => {
                    self.flow = Flow::Normal;
                }
                Flow::Return => return last_status,
                Flow::Normal => {}
            }

            if self.stopped {
                return last_status;
            }
        }
    }

    fn run_for_loop(&mut self, for_loop: &ForLoop) -> bool {
        let values = match self.collect_for_values(&for_loop.iterable) {
            Ok(values) => values,
            Err(message) => return self.fail(message),
        };

        if values.len() > MAX_LOOP_ITERATIONS {
            return self.fail(format!(
                "for: iteration limit of {MAX_LOOP_ITERATIONS} exceeded"
            ));
        }

        let mut last_status = true;

        for value in values {
            if self.stopped || self.flow == Flow::Return {
                return last_status;
            }

            self.session.set_variable(&for_loop.variable, value, false);

            self.loop_depth += 1;
            last_status = self.run_block(&for_loop.body);
            self.loop_depth -= 1;

            match self.flow {
                Flow::Break => {
                    self.flow = Flow::Normal;
                    return true;
                }
                Flow::Continue => {
                    self.flow = Flow::Normal;
                }
                Flow::Return => return last_status,
                Flow::Normal => {}
            }

            if self.stopped {
                return last_status;
            }
        }

        last_status
    }

    /// Turns the iterable of a `for` loop into concrete string values.
    ///
    /// Supported forms:
    /// - a list of words: `for name in alice bob`
    /// - a numeric range: `for i in 1..5` or `for i in 5..1`
    /// - a variable whose value contains whitespace-separated words
    fn collect_for_values(&self, iterable: &Expression) -> Result<Vec<String>, String> {
        let Expression::Command(command) = iterable else {
            return Err(
                "for: iterable must be a word list, a variable, or a numeric range".to_owned(),
            );
        };

        let mut values = Vec::new();

        for word in std::iter::once(&command.name).chain(command.arguments.iter()) {
            let expanded = expand_word(word, self.session);

            if word.has_variable() && expanded.split_whitespace().count() > 1 {
                values.extend(expanded.split_whitespace().map(str::to_owned));
            } else {
                values.push(expanded);
            }
        }

        if values.len() == 1
            && let Some((start, end)) = parse_range_endpoints(&values[0])
        {
            let length = start.abs_diff(end) as usize + 1;
            if length > MAX_LOOP_ITERATIONS {
                return Err(format!(
                    "for: range length exceeds the iteration limit of {MAX_LOOP_ITERATIONS}"
                ));
            }

            return Ok(expand_range(start, end));
        }

        Ok(values)
    }

    fn run_function_def(&mut self, func_def: &FunctionDef) -> bool {
        // Store the function definition in the session for later calls.
        self.session
            .set_function(func_def.name.clone(), func_def.clone());
        self.session.set_last_status(0);
        true
    }

    fn run_return(&mut self, ret: &Return) -> bool {
        if self.call_depth == 0 {
            return self.fail("return: not inside a function");
        }

        self.return_value = ret.value.as_ref().map(|value| self.eval_value(value));
        self.flow = Flow::Return;
        true
    }

    fn run_break(&mut self, _brk: &Break) -> bool {
        if self.loop_depth == 0 {
            return self.fail("break: not inside a loop");
        }

        self.flow = Flow::Break;
        true
    }

    fn run_continue(&mut self, _cont: &Continue) -> bool {
        if self.loop_depth == 0 {
            return self.fail("continue: not inside a loop");
        }

        self.flow = Flow::Continue;
        true
    }

    fn eval_condition(&mut self, expr: &Expression) -> bool {
        let result = self.run_expression(expr);

        if self.stopped || self.flow == Flow::Return {
            return result;
        }

        // A condition is true when the expression succeeds (exit code 0).
        self.session.last_status() == 0
    }

    /// Evaluates a value-like expression without running it as a command.
    ///
    /// `return 5` and `return $name` are not commands; they are values. This
    /// helper expands their words and joins them with single spaces.
    fn eval_value(&self, expr: &Expression) -> String {
        match expr {
            Expression::Command(command) => {
                let mut words = Vec::with_capacity(command.arguments.len() + 1);
                words.push(expand_word(&command.name, self.session));
                words.extend(
                    command
                        .arguments
                        .iter()
                        .map(|argument| expand_word(argument, self.session)),
                );
                words.join(" ")
            }
            Expression::Assignment(assignment) => expand_assignment_value(assignment, self.session),
            _ => String::new(),
        }
    }

    fn run_command(&mut self, command: &Command) -> bool {
        let name = expand_word(&command.name, self.session);

        // Built-ins always win. Functions are checked next, before external
        // process resolution.
        if self.registry.get(&name).is_none()
            && let Some(function) = self.session.get_function(&name).cloned()
        {
            if !command.redirections.is_empty() {
                return self.fail(format!(
                    "{name}: redirections on function calls are not supported yet"
                ));
            }

            let arguments: Vec<String> = command
                .arguments
                .iter()
                .map(|argument| expand_word(argument, self.session))
                .collect();

            return self.call_function(&name, &function, &arguments);
        }

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

    fn call_function(&mut self, name: &str, function: &FunctionDef, arguments: &[String]) -> bool {
        if self.call_depth >= MAX_FUNCTION_DEPTH {
            return self.fail(format!(
                "{name}: maximum function recursion depth of {MAX_FUNCTION_DEPTH} exceeded"
            ));
        }

        // Parameters are scoped to the call. Capture any existing local values
        // so they can be restored after the function returns.
        let locals = self.session.local_variables();
        let mut previous = Vec::with_capacity(function.parameters.len());

        for parameter in &function.parameters {
            let existing = locals
                .iter()
                .find(|(local, _, _)| local == parameter)
                .map(|(_, value, exported)| (value.clone(), *exported));
            previous.push((parameter.clone(), existing));
        }

        for (index, parameter) in function.parameters.iter().enumerate() {
            let value = arguments.get(index).cloned().unwrap_or_default();
            self.session.set_variable(parameter, value, false);
        }

        self.call_depth += 1;
        self.flow = Flow::Normal;
        let body_status = self.run_block(&function.body);

        let returned = if self.flow == Flow::Return {
            let value = self.return_value.take();
            self.flow = Flow::Normal;
            value
        } else {
            None
        };

        self.call_depth -= 1;

        for (parameter, existing) in previous {
            match existing {
                Some((value, exported)) => self.session.set_variable(&parameter, value, exported),
                None => {
                    self.session.remove_local(&parameter);
                }
            }
        }

        if let Some(value) = returned {
            let status = value.trim().parse::<i32>().unwrap_or(0);
            self.session.set_last_status(status);
            return status == 0;
        }

        body_status
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

        // Update $? from the pipeline outcome.
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

    fn fail(&mut self, message: impl Into<String>) -> bool {
        self.append(&message.into());
        self.success = false;
        self.exit_code = None;
        self.session.set_last_status(1);
        false
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

/// Parses `start..end` or `start..=end` into its endpoints.
fn parse_range_endpoints(value: &str) -> Option<(i64, i64)> {
    let (start, end) = value.split_once("..")?;
    let start = start.trim().parse::<i64>().ok()?;
    let end = end.trim().strip_prefix('=').unwrap_or(end.trim());
    let end = end.parse::<i64>().ok()?;
    Some((start, end))
}

/// Expands an inclusive numeric range, descending when `start > end`.
fn expand_range(start: i64, end: i64) -> Vec<String> {
    if start <= end {
        (start..=end).map(|value| value.to_string()).collect()
    } else {
        (end..=start).rev().map(|value| value.to_string()).collect()
    }
}
