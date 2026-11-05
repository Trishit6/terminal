//! Pipeline, redirection, and built-in I/O execution.
//!
//! The evaluator hands this module already-parsed pipeline stages. Simple
//! commands are resolved against the native registry first; built-ins execute
//! in-process, while everything else is launched directly with
//! [`std::process::Command`].
//!
//! External stages are connected with real operating-system pipes: a stage's
//! [`std::process::ChildStdout`] becomes the next stage's standard input, so
//! `command1 | command2` never buffers the whole stream in memory. Built-ins
//! run in-process and therefore exchange [`String`] data with their neighbours;
//! intermediate output from a built-in is held only until the next stage starts.
//!
//! The module also applies `<`, `>`, `>>`, `2>`, and `2>>` redirections. Paths
//! are resolved against the session working directory by the same helper the
//! native commands use.

use crate::commands::{self, CommandContext};
use crate::expand::{expand_redirection_target, expand_word};
use crate::outcome::CommandAction;
use crate::process::{ExternalCommand, ProcessError, ProcessOptions, ProcessStatus, StdioMode};
use crate::registry::BuiltinRegistry;
use shell_core::{Command, PipelineStage, RedirectionKind};
use shell_runtime::Session;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};

/// A pipeline stage borrowed from the AST.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Stage<'a> {
    /// A simple command.
    Command(&'a Command),
    /// A parenthesized command list. Grouped pipeline stages are not executed
    /// yet; the evaluator reports a clear error for them.
    Group,
}

impl<'a> From<&'a PipelineStage> for Stage<'a> {
    fn from(stage: &'a PipelineStage) -> Self {
        match stage {
            PipelineStage::Command(command) => Self::Command(command),
            PipelineStage::Group(_) => Self::Group,
        }
    }
}

/// Result of executing one command or pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PipelineOutcome {
    /// Data the REPL should display on standard output.
    pub stdout: String,
    /// Data the REPL should display on standard error.
    pub stderr: String,
    /// Status of the final stage.
    pub success: bool,
    /// Exit code of the final external stage, if it exited normally.
    pub exit_code: Option<i32>,
    /// Action requested by a single built-in command.
    pub action: CommandAction,
    /// Whether a single built-in requested shell shutdown.
    pub stopped: bool,
}

impl PipelineOutcome {
    fn failure(message: impl Into<String>) -> Self {
        Self {
            stdout: String::new(),
            stderr: message.into(),
            success: false,
            exit_code: None,
            action: CommandAction::Continue,
            stopped: false,
        }
    }
}

/// Executes one or more stages, including redirections.
///
/// `propagate_actions` is true for a single simple command, where `exit` and
/// `clear` must affect the parent shell. Stages inside a real pipeline never
/// control the parent shell.
pub(crate) fn execute(
    stages: &[Stage<'_>],
    session: &mut Session,
    registry: &BuiltinRegistry,
    history: &[String],
    options: &ProcessOptions,
    propagate_actions: bool,
) -> PipelineOutcome {
    debug_assert!(
        !stages.is_empty(),
        "the parser never produces an empty pipeline"
    );

    let mut children: Vec<Spawned> = Vec::new();
    let mut writers: Vec<JoinHandle<()>> = Vec::new();
    let mut previous = Previous::None;
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut success = true;
    let mut exit_code = None;
    let mut action = CommandAction::Continue;
    let mut stopped = false;
    let mut final_child_index = None;
    let stage_count = stages.len();

    for (index, stage) in stages.iter().enumerate() {
        let is_last = index + 1 == stage_count;

        let command = match stage {
            Stage::Command(command) => *command,
            Stage::Group => {
                cleanup(&mut children);
                return PipelineOutcome::failure("Grouped pipelines are not supported yet.");
            }
        };

        let spec = match prepare_stage(command, session, registry) {
            Ok(spec) => spec,
            Err(message) => {
                cleanup(&mut children);
                return PipelineOutcome::failure(message);
            }
        };

        if spec.is_builtin {
            let stdin_data = match &spec.input {
                Some(path) => {
                    // An explicit `<` overrides the previous stage's output.
                    drop(previous.take());

                    match std::fs::read(path) {
                        Ok(bytes) => Some(bytes),
                        Err(error) => {
                            cleanup(&mut children);
                            return PipelineOutcome::failure(format!(
                                "{}: cannot open '{}': {error}",
                                spec.name,
                                path.display()
                            ));
                        }
                    }
                }
                None => match previous.take() {
                    // Only the first stage can inherit the terminal. A later
                    // stage whose producer redirected its output away receives
                    // an empty stream, not the terminal and not an error.
                    Previous::None if index == 0 => None,
                    Previous::None => Some(Vec::new()),
                    Previous::Buffer(buffer) => Some(buffer),
                    Previous::Pipe(mut pipe) => match read_stream(&mut pipe) {
                        Ok(bytes) => Some(bytes),
                        Err(error) => {
                            cleanup(&mut children);
                            return PipelineOutcome::failure(format!(
                                "{}: cannot read pipeline input: {error}",
                                spec.name
                            ));
                        }
                    },
                },
            };

            let stdin_text = stdin_data
                .as_ref()
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned());

            // Built-ins in a real pipeline run in a child-like role: filesystem
            // side effects persist, but session state such as the working
            // directory does not leak back to the parent shell.
            let result = if stage_count == 1 {
                let mut context = CommandContext {
                    session,
                    registry,
                    history,
                    stdin: stdin_text.as_deref(),
                };

                commands::run(&mut context, &spec.name, &spec.arguments)
            } else {
                let mut isolated = session.clone();
                let mut context = CommandContext {
                    session: &mut isolated,
                    registry,
                    history,
                    stdin: stdin_text.as_deref(),
                };

                commands::run(&mut context, &spec.name, &spec.arguments)
            };

            let mut stage_stderr = result.stderr().to_owned();

            if let Some(target) = &spec.error {
                match write_output(target, stage_stderr.as_bytes()) {
                    Ok(()) => stage_stderr.clear(),
                    Err(error) => {
                        cleanup(&mut children);
                        return PipelineOutcome::failure(format!(
                            "{}: cannot write '{}': {error}",
                            spec.name,
                            target.path.display()
                        ));
                    }
                }
            }

            append_stream(&mut stderr, &stage_stderr);

            let stage_stdout = result.stdout();

            if let Some(target) = &spec.output {
                if let Err(error) = write_output(target, stage_stdout.as_bytes()) {
                    cleanup(&mut children);
                    return PipelineOutcome::failure(format!(
                        "{}: cannot write '{}': {error}",
                        spec.name,
                        target.path.display()
                    ));
                }
            } else if is_last {
                stdout.push_str(stage_stdout);
            } else {
                previous = Previous::Buffer(stage_stdout.as_bytes().to_vec());
            }

            if is_last {
                success = result.success();
                exit_code = None;

                if propagate_actions {
                    action = result.action();
                    stopped = action == CommandAction::Exit;
                }
            }

            continue;
        }

        let (stdin_stdio, feed) = match &spec.input {
            Some(path) => {
                // An explicit `<` overrides the previous stage's output.
                drop(previous.take());

                match File::open(path) {
                    Ok(file) => (Stdio::from(file), None),
                    Err(error) => {
                        cleanup(&mut children);
                        return PipelineOutcome::failure(format!(
                            "{}: cannot open '{}': {error}",
                            spec.name,
                            path.display()
                        ));
                    }
                }
            }
            None => match previous.take() {
                Previous::Pipe(pipe) => (Stdio::from(pipe), None),
                Previous::Buffer(buffer) => (Stdio::piped(), Some(buffer)),
                Previous::None => {
                    if options.stdio() == StdioMode::Capture {
                        (Stdio::null(), None)
                    } else {
                        (Stdio::inherit(), None)
                    }
                }
            },
        };

        let (stdout_stdio, capture_stdout) = match &spec.output {
            Some(target) => match open_output(target) {
                Ok(file) => (Stdio::from(file), false),
                Err(error) => {
                    cleanup(&mut children);
                    return PipelineOutcome::failure(format!(
                        "{}: cannot write '{}': {error}",
                        spec.name,
                        target.path.display()
                    ));
                }
            },
            None if !is_last => (Stdio::piped(), false),
            None if options.stdio() == StdioMode::Capture => (Stdio::piped(), true),
            None => (Stdio::inherit(), false),
        };

        let (stderr_stdio, capture_stderr) = match &spec.error {
            Some(target) => match open_output(target) {
                Ok(file) => (Stdio::from(file), false),
                Err(error) => {
                    cleanup(&mut children);
                    return PipelineOutcome::failure(format!(
                        "{}: cannot write '{}': {error}",
                        spec.name,
                        target.path.display()
                    ));
                }
            },
            None if is_last && options.stdio() == StdioMode::Capture => (Stdio::piped(), true),
            None => (Stdio::inherit(), false),
        };

        let external = ExternalCommand::new(&spec.name)
            .args(spec.arguments.iter().cloned())
            .current_dir(session.working_directory().to_path_buf())
            .envs(options.environment().iter().cloned());

        let mut child = match external.spawn_with(stdin_stdio, stdout_stdio, stderr_stdio) {
            Ok(child) => child,
            Err(error) => {
                cleanup(&mut children);
                return PipelineOutcome::failure(external_error_message(&spec.name, error));
            }
        };

        if let Some(bytes) = feed
            && let Some(mut handle) = child.stdin.take()
        {
            // Feed buffered built-in output without blocking pipeline
            // construction. The thread closes the pipe when it finishes, which
            // signals end-of-input to the child.
            writers.push(thread::spawn(move || {
                let _ = handle.write_all(&bytes);
            }));
        }

        if !is_last {
            previous = match child.stdout.take() {
                Some(pipe) => Previous::Pipe(pipe),
                // A redirection consumed stdout, so there is nothing to pass on.
                None => Previous::None,
            };
        }

        let child_index = children.len();
        children.push(Spawned {
            name: spec.name,
            child,
            capture_stdout,
            capture_stderr,
        });

        if is_last {
            final_child_index = Some(child_index);
        }
    }

    if let Some(index) = final_child_index {
        let mut entry = children.remove(index);

        if entry.capture_stdout || entry.capture_stderr {
            match entry.child.wait_with_output() {
                Ok(output) => {
                    if entry.capture_stdout {
                        stdout.push_str(&String::from_utf8_lossy(&output.stdout));
                    }

                    if entry.capture_stderr {
                        append_stream(&mut stderr, &String::from_utf8_lossy(&output.stderr));
                    }

                    success = output.status.success();
                    exit_code = output.status.code();
                    append_signal(&mut stderr, &entry.name, output.status);
                }
                Err(error) => {
                    cleanup(&mut children);
                    return PipelineOutcome::failure(format!(
                        "{}: failed while waiting: {error}",
                        entry.name
                    ));
                }
            }
        } else {
            match entry.child.wait() {
                Ok(status) => {
                    success = status.success();
                    exit_code = status.code();
                    append_signal(&mut stderr, &entry.name, status);
                }
                Err(error) => {
                    cleanup(&mut children);
                    return PipelineOutcome::failure(format!(
                        "{}: failed while waiting: {error}",
                        entry.name
                    ));
                }
            }
        }
    }

    // Intermediate stages are waited for, but their signal status is not
    // reported: a producer receiving SIGPIPE after a short consumer exits is
    // normal pipeline behaviour. The final stage's status is authoritative.
    for mut entry in children {
        if let Err(error) = entry.child.wait() {
            append_stream(
                &mut stderr,
                &format!("{}: failed while waiting: {error}", entry.name),
            );
        }
    }

    for writer in writers {
        let _ = writer.join();
    }

    PipelineOutcome {
        stdout,
        stderr,
        success,
        exit_code,
        action,
        stopped,
    }
}

/// Prepared form of a simple command: decoded words plus resolved redirections.
struct StageSpec {
    name: String,
    arguments: Vec<String>,
    is_builtin: bool,
    input: Option<PathBuf>,
    output: Option<RedirectionTarget>,
    error: Option<RedirectionTarget>,
}

struct RedirectionTarget {
    path: PathBuf,
    append: bool,
}

fn prepare_stage(
    command: &Command,
    session: &Session,
    registry: &BuiltinRegistry,
) -> Result<StageSpec, String> {
    let name = expand_word(&command.name, session);

    let mut arguments = Vec::with_capacity(command.arguments.len());

    for argument in &command.arguments {
        arguments.push(expand_word(argument, session));
    }

    let mut input = None;
    let mut output = None;
    let mut error = None;

    for redirection in &command.redirections {
        let target = expand_redirection_target(redirection, session);
        let path = commands::resolve_path(session, &target);

        match redirection.kind {
            RedirectionKind::Input => input = Some(path),
            RedirectionKind::Output => {
                output = Some(RedirectionTarget {
                    path,
                    append: false,
                });
            }
            RedirectionKind::Append => {
                output = Some(RedirectionTarget { path, append: true });
            }
            RedirectionKind::Error => {
                error = Some(RedirectionTarget {
                    path,
                    append: false,
                });
            }
            RedirectionKind::ErrorAppend => {
                error = Some(RedirectionTarget { path, append: true });
            }
        }
    }

    Ok(StageSpec {
        is_builtin: registry.get(&name).is_some(),
        name,
        arguments,
        input,
        output,
        error,
    })
}

/// Output of a stage that has already completed.
enum Previous {
    /// No producer: the next stage inherits the terminal or reads a file.
    None,
    /// A live pipe from an external process.
    Pipe(ChildStdout),
    /// Captured output from a built-in.
    Buffer(Vec<u8>),
}

impl Previous {
    fn take(&mut self) -> Self {
        std::mem::replace(self, Self::None)
    }
}

struct Spawned {
    name: String,
    child: Child,
    capture_stdout: bool,
    capture_stderr: bool,
}

fn read_stream(pipe: &mut ChildStdout) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    pipe.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn open_output(target: &RedirectionTarget) -> io::Result<File> {
    if target.append {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&target.path)
    } else {
        File::create(&target.path)
    }
}

fn write_output(target: &RedirectionTarget, bytes: &[u8]) -> io::Result<()> {
    let mut file = open_output(target)?;
    file.write_all(bytes)
}

fn cleanup(children: &mut Vec<Spawned>) {
    for mut entry in children.drain(..) {
        let _ = entry.child.kill();
        let _ = entry.child.wait();
    }
}

fn external_error_message(name: &str, error: ProcessError) -> String {
    match error {
        // Keep the established Phase 5 wording for an unknown command.
        ProcessError::NotFound { .. } => format!("Command not found: {name}"),
        other => other.to_string(),
    }
}

fn append_signal(stderr: &mut String, name: &str, status: ExitStatus) {
    if let Some(signal) = ProcessStatus::from_exit_status(status).signal() {
        append_stream(stderr, &format!("{name}: terminated by signal {signal}"));
    }
}

fn append_stream(target: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }

    if !target.is_empty() && !target.ends_with('\n') {
        target.push('\n');
    }

    target.push_str(text);
}
