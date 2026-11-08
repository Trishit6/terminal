use crate::outcome::{CommandAction, CommandOutcome};
use crate::process::{ExternalCommand, ProcessOptions};
use crate::registry::BuiltinRegistry;
use chrono::{DateTime, Local};
use shell_core::{PRODUCT_NAME, lex_spanned, parse};
use shell_runtime::{JobStatus, Session};
use std::env;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub(crate) struct CommandResult {
    stdout: String,
    stderr: String,
    success: bool,
    action: CommandAction,
    exit_code: Option<i32>,
}

impl CommandResult {
    fn ok(output: impl Into<String>) -> Self {
        Self {
            stdout: output.into(),
            stderr: String::new(),
            success: true,
            action: CommandAction::Continue,
            exit_code: None,
        }
    }

    fn silent() -> Self {
        Self::ok("")
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            stdout: String::new(),
            stderr: message.into(),
            success: false,
            action: CommandAction::Continue,
            exit_code: None,
        }
    }

    fn from_parts(output: String, errors: &[String]) -> Self {
        Self::from_parts_status(output, errors, errors.is_empty())
    }

    fn from_parts_status(output: String, errors: &[String], success: bool) -> Self {
        Self {
            stdout: output,
            stderr: errors.join("\n"),
            success: success && errors.is_empty(),
            action: CommandAction::Continue,
            exit_code: None,
        }
    }

    fn status(success: bool) -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            success,
            action: CommandAction::Continue,
            exit_code: None,
        }
    }

    fn from_outcome(outcome: CommandOutcome) -> Self {
        Self {
            stdout: outcome.output().unwrap_or_default().to_owned(),
            stderr: String::new(),
            success: outcome.success(),
            action: outcome.action(),
            exit_code: outcome.exit_code(),
        }
    }

    fn clear_history(output: impl Into<String>) -> Self {
        Self {
            stdout: output.into(),
            stderr: String::new(),
            success: true,
            action: CommandAction::ClearHistory,
            exit_code: None,
        }
    }

    fn exit(exit_code: Option<i32>) -> Self {
        Self {
            stdout: "Goodbye.".to_owned(),
            stderr: String::new(),
            success: true,
            action: CommandAction::Exit,
            exit_code,
        }
    }

    fn clear() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            success: true,
            action: CommandAction::ClearScreen,
            exit_code: None,
        }
    }

    pub(crate) fn stdout(&self) -> &str {
        &self.stdout
    }

    pub(crate) fn stderr(&self) -> &str {
        &self.stderr
    }

    pub(crate) const fn success(&self) -> bool {
        self.success
    }

    pub(crate) const fn action(&self) -> CommandAction {
        self.action
    }

    pub(crate) const fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }
}

/// State shared by command implementations during one evaluation.
pub(crate) struct CommandContext<'a> {
    pub(crate) session: &'a mut Session,
    pub(crate) registry: &'a BuiltinRegistry,
    pub(crate) history: &'a [String],
    pub(crate) options: &'a ProcessOptions,
    /// Data supplied by a pipeline or an input redirection, if any.
    ///
    /// `None` means the command inherited the terminal; built-ins never read
    /// the terminal directly, so `cat` without operands reports a missing
    /// operand instead of blocking.
    pub(crate) stdin: Option<&'a str>,
}

pub(crate) fn run(context: &mut CommandContext<'_>, name: &str, args: &[String]) -> CommandResult {
    match name {
        "help" => CommandResult::ok(context.registry.help_text()),
        "pwd" => pwd(context.session),
        "cd" => cd(context.session, args),
        "ls" => ls(context.session, args),
        "tree" => tree(context.session, args),
        "find" => find(context.session, args),
        "stat" => stat(context.session, args),
        "mkdir" => mkdir(context.session, args),
        "touch" => touch(context.session, args),
        "cat" => cat(context.session, context.stdin, args),
        "head" => head(context.session, context.stdin, args),
        "tail" => tail(context.session, context.stdin, args),
        "wc" => wc(context.session, context.stdin, args),
        "grep" => grep(context.session, context.stdin, args),
        "sort" => sort(context.session, context.stdin, args),
        "uniq" => uniq(context.session, context.stdin, args),
        "echo" => echo(args),
        "print" => print_builtin(args),
        "input" => {
            let stdin = context.stdin;
            input_builtin(context.session, stdin, args)
        }
        "len" => len_builtin(args),
        "type" => type_builtin(args),
        "str" => str_builtin(args),
        "int" => int_builtin(args),
        "float" => float_builtin(args),
        "bool" => bool_builtin(args),
        "cp" => cp(context.session, args),
        "mv" => mv(context.session, args),
        "rm" => rm(context.session, args),
        "which" => which(context.session, context.registry, args),
        "basename" => basename(args),
        "dirname" => dirname(args),
        "realpath" => realpath(context.session, args),
        "clear" => CommandResult::clear(),
        "history" => history(context.session, context.history, args),
        "jobs" => jobs(context.session, args),
        "wait" => wait(context.session, args),
        "kill" => kill(context.session, args),
        "fg" => fg(context.session, args),
        "bg" => bg(context.session, args),
        "run" => run_script(
            context.session,
            context.registry,
            context.history,
            context.options,
            args,
        ),
        "test" | "[" => test_builtin(context.session, name, args),
        "expr" => expr_builtin(args),
        "sleep" => sleep(args),
        "true" => CommandResult::status(true),
        "false" => CommandResult::status(false),
        "whoami" => whoami(),
        "date" => date(),
        "version" => CommandResult::ok(format!("{PRODUCT_NAME} v{}", context.session.version())),
        "exit" => exit_builtin(context.session, args),
        "export" => export_builtin(context.session, args),
        "unset" => unset_builtin(context.session, args),
        "env" => env_builtin(context.session, args),
        "set" => set_builtin(context.session, args),
        _ => CommandResult::error(format!("Command not found: {name}")),
    }
}

fn pwd(session: &Session) -> CommandResult {
    CommandResult::ok(session.working_directory().display().to_string())
}

fn cd(session: &mut Session, args: &[String]) -> CommandResult {
    if args.len() > 1 {
        return CommandResult::error("cd: too many arguments");
    }

    let target = match args.first() {
        Some(argument) => resolve_path(session, argument),
        None => match session_home(session) {
            Some(home) => home,
            None => return CommandResult::error("cd: cannot determine the home directory"),
        },
    };

    if let Err(message) = validate_directory(&target, args.first().map_or("~", String::as_str)) {
        return CommandResult::error(message);
    }

    let canonical = match fs::canonicalize(&target) {
        Ok(path) => simplify_path(path),
        Err(error) => {
            return CommandResult::error(format!(
                "cd: {}: {}",
                args.first().map_or("~", String::as_str),
                io_message(&error)
            ));
        }
    };

    session.set_working_directory(canonical);
    CommandResult::silent()
}

fn exit_builtin(session: &mut Session, args: &[String]) -> CommandResult {
    if args.len() > 1 {
        return CommandResult::error("exit: too many arguments");
    }

    let status = match args.first() {
        None => 0,
        Some(argument) => match argument.trim().parse::<i32>() {
            Ok(status) => status,
            Err(_) => {
                return CommandResult::error(format!(
                    "exit: {argument}: numeric argument required"
                ));
            }
        },
    };

    session.set_last_status(status);
    CommandResult::exit(Some(status))
}

fn validate_directory(path: &Path, display: &str) -> Result<(), String> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(format!("cd: {display}: not a directory")),
        Err(error) => Err(format!("cd: {display}: {}", io_message(&error))),
    }
}

fn ls(session: &Session, args: &[String]) -> CommandResult {
    let targets = if args.is_empty() {
        vec![".".to_owned()]
    } else {
        args.to_vec()
    };
    let multiple = targets.len() > 1;
    let mut output = String::new();
    let mut errors = Vec::new();

    for target in &targets {
        if multiple {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(target);
            output.push_str(":\n");
        }

        match list_target(session, target) {
            Ok(entries) => {
                for entry in entries {
                    output.push_str(&entry);
                    output.push('\n');
                }
            }
            Err(message) => errors.push(message),
        }
    }

    if output.ends_with('\n') {
        output.pop();
    }

    CommandResult::from_parts(output, &errors)
}

fn list_target(session: &Session, target: &str) -> Result<Vec<String>, String> {
    let path = resolve_path(session, target);
    let metadata =
        fs::metadata(&path).map_err(|error| format!("ls: cannot access '{target}': {error}"))?;

    if !metadata.is_dir() {
        return Ok(vec![target.to_owned()]);
    }

    let mut entries = Vec::new();
    let directory =
        fs::read_dir(&path).map_err(|error| format!("ls: cannot read '{target}': {error}"))?;

    for entry in directory {
        let entry = entry.map_err(|error| format!("ls: cannot read '{target}': {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("ls: cannot inspect '{}': {error}", entry.path().display()))?;
        let mut name = entry.file_name().to_string_lossy().into_owned();

        if file_type.is_dir() {
            name.push(std::path::MAIN_SEPARATOR);
        }

        entries.push(name);
    }

    entries.sort();
    Ok(entries)
}

fn tree(session: &Session, args: &[String]) -> CommandResult {
    let mut show_hidden = false;
    let mut dirs_only = false;
    let mut max_depth = usize::MAX;
    let mut operands = Vec::new();
    let mut options = true;
    let mut index = 0;

    while index < args.len() {
        let argument = &args[index];
        if options && argument == "--" {
            options = false;
        } else if options && (argument == "-a" || argument == "--all") {
            show_hidden = true;
        } else if options && (argument == "-d" || argument == "--dirs-only") {
            dirs_only = true;
        } else if options && (argument == "-L" || argument == "--level") {
            index += 1;
            let Some(value) = args.get(index) else {
                return CommandResult::error("tree: -L requires a depth");
            };
            match value.parse::<usize>() {
                Ok(depth) => max_depth = depth,
                Err(_) => {
                    return CommandResult::error(format!("tree: invalid depth: {value}"));
                }
            }
        } else if options && argument.starts_with('-') && argument.len() > 1 {
            return CommandResult::error(format!("tree: unknown option: {argument}"));
        } else {
            operands.push(argument.clone());
        }
        index += 1;
    }

    if operands.len() > 1 {
        return CommandResult::error("tree: too many operands");
    }

    let target = operands.first().map_or(".", String::as_str);
    let path = resolve_path(session, target);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return CommandResult::error(format!(
                "tree: cannot access '{target}': {}",
                io_message(&error)
            ));
        }
    };

    let mut output = format!("{target}\n");
    let mut directories = 0usize;
    let mut files = 0usize;
    let mut errors = Vec::new();

    if metadata.is_dir() {
        tree_children(
            &path,
            "",
            1,
            max_depth,
            show_hidden,
            dirs_only,
            &mut output,
            &mut directories,
            &mut files,
            &mut errors,
        );
    } else {
        files = 1;
    }

    if !dirs_only || directories > 0 || files > 0 {
        output.push_str(&format!(
            "\n{directories} director{}, {files} file{}",
            if directories == 1 { "y" } else { "ies" },
            if files == 1 { "" } else { "s" }
        ));
    }

    CommandResult::from_parts(output, &errors)
}

#[allow(clippy::too_many_arguments)]
fn tree_children(
    directory: &Path,
    prefix: &str,
    depth: usize,
    max_depth: usize,
    show_hidden: bool,
    dirs_only: bool,
    output: &mut String,
    directories: &mut usize,
    files: &mut usize,
    errors: &mut Vec<String>,
) {
    if depth > max_depth {
        return;
    }

    let mut entries = match fs::read_dir(directory) {
        Ok(entries) => entries.flatten().collect::<Vec<_>>(),
        Err(error) => {
            errors.push(format!(
                "tree: cannot read '{}': {}",
                directory.display(),
                io_message(&error)
            ));
            return;
        }
    };

    entries.sort_by_key(|entry| entry.file_name());

    let visible: Vec<_> = entries
        .into_iter()
        .filter(|entry| show_hidden || !entry.file_name().to_string_lossy().starts_with('.'))
        .collect();

    for (index, entry) in visible.iter().enumerate() {
        let last = index + 1 == visible.len();
        let connector = if last { "└── " } else { "├── " };
        let name = entry.file_name().to_string_lossy().into_owned();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                errors.push(format!(
                    "tree: cannot inspect '{}': {}",
                    entry.path().display(),
                    io_message(&error)
                ));
                continue;
            }
        };

        if file_type.is_dir() {
            *directories += 1;
            output.push_str(&format!("{prefix}{connector}{name}/\n"));
            let child_prefix = format!("{prefix}{}", if last { "    " } else { "│   " });
            tree_children(
                &entry.path(),
                &child_prefix,
                depth + 1,
                max_depth,
                show_hidden,
                dirs_only,
                output,
                directories,
                files,
                errors,
            );
        } else if !dirs_only {
            *files += 1;
            let suffix = if file_type.is_symlink() { "@" } else { "" };
            output.push_str(&format!("{prefix}{connector}{name}{suffix}\n"));
        }
    }
}

fn find(session: &Session, args: &[String]) -> CommandResult {
    let mut paths = Vec::new();
    let mut name = None;
    let mut iname = None;
    let mut path_pattern = None;
    let mut ipath_pattern = None;
    let mut type_filter = None;
    let mut max_depth = usize::MAX;
    let mut min_depth = 0usize;
    let mut index = 0;

    while index < args.len() {
        let argument = &args[index];
        match argument.as_str() {
            "--" => {
                paths.extend(args[index + 1..].iter().cloned());
                break;
            }
            "-name" | "-iname" | "-path" | "-ipath" | "-type" | "-maxdepth" | "-mindepth" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return CommandResult::error(format!("find: {argument} requires a value"));
                };
                match argument.as_str() {
                    "-name" => name = Some(value.clone()),
                    "-iname" => iname = Some(value.clone()),
                    "-path" => path_pattern = Some(value.clone()),
                    "-ipath" => ipath_pattern = Some(value.clone()),
                    "-type" => match value.as_str() {
                        "f" | "d" | "l" => type_filter = Some(value.clone()),
                        _ => {
                            return CommandResult::error(format!(
                                "find: unsupported type '{value}' (use f, d, or l)"
                            ));
                        }
                    },
                    "-maxdepth" => match value.parse::<usize>() {
                        Ok(depth) => max_depth = depth,
                        Err(_) => {
                            return CommandResult::error(format!(
                                "find: invalid maxdepth: {value}"
                            ));
                        }
                    },
                    "-mindepth" => match value.parse::<usize>() {
                        Ok(depth) => min_depth = depth,
                        Err(_) => {
                            return CommandResult::error(format!(
                                "find: invalid mindepth: {value}"
                            ));
                        }
                    },
                    _ => unreachable!(),
                }
            }
            option if option.starts_with('-') && option.len() > 1 => {
                return CommandResult::error(format!("find: unknown option: {option}"));
            }
            _ => paths.push(argument.clone()),
        }
        index += 1;
    }

    if paths.is_empty() {
        paths.push(".".to_owned());
    }

    let mut output = Vec::new();
    let mut errors = Vec::new();

    for target in paths {
        let path = resolve_path(session, &target);
        find_walk(
            &path,
            &target,
            0,
            max_depth,
            min_depth,
            name.as_deref(),
            iname.as_deref(),
            path_pattern.as_deref(),
            ipath_pattern.as_deref(),
            type_filter.as_deref(),
            &mut output,
            &mut errors,
        );
    }

    CommandResult::from_parts(output.join("\n"), &errors)
}

#[allow(clippy::too_many_arguments)]
fn find_walk(
    path: &Path,
    display: &str,
    depth: usize,
    max_depth: usize,
    min_depth: usize,
    name: Option<&str>,
    iname: Option<&str>,
    path_pattern: Option<&str>,
    ipath_pattern: Option<&str>,
    type_filter: Option<&str>,
    output: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            errors.push(format!(
                "find: cannot access '{display}': {}",
                io_message(&error)
            ));
            return;
        }
    };

    if depth >= min_depth {
        let file_name = path.file_name().map_or_else(
            || display.to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        let matches = name.is_none_or(|pattern| glob_match(pattern, &file_name, false))
            && iname.is_none_or(|pattern| glob_match(pattern, &file_name, true))
            && path_pattern.is_none_or(|pattern| glob_match(pattern, display, false))
            && ipath_pattern.is_none_or(|pattern| glob_match(pattern, display, true))
            && type_filter.is_none_or(|kind| match kind {
                "f" => metadata.is_file(),
                "d" => metadata.is_dir(),
                "l" => metadata.file_type().is_symlink(),
                _ => false,
            });

        if matches {
            output.push(display.to_owned());
        }
    }

    if depth >= max_depth || !metadata.is_dir() || metadata.file_type().is_symlink() {
        return;
    }

    let mut entries = match fs::read_dir(path) {
        Ok(entries) => entries.flatten().collect::<Vec<_>>(),
        Err(error) => {
            errors.push(format!(
                "find: cannot read '{display}': {}",
                io_message(&error)
            ));
            return;
        }
    };
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let child_display = Path::new(display)
            .join(entry.file_name())
            .to_string_lossy()
            .into_owned();
        find_walk(
            &entry.path(),
            &child_display,
            depth + 1,
            max_depth,
            min_depth,
            name,
            iname,
            path_pattern,
            ipath_pattern,
            type_filter,
            output,
            errors,
        );
    }
}

fn glob_match(pattern: &str, text: &str, case_insensitive: bool) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    glob_match_chars(&pattern, &text, case_insensitive)
}

fn glob_match_chars(pattern: &[char], text: &[char], case_insensitive: bool) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }

    match pattern[0] {
        '*' => {
            glob_match_chars(&pattern[1..], text, case_insensitive)
                || (!text.is_empty() && glob_match_chars(pattern, &text[1..], case_insensitive))
        }
        '?' => !text.is_empty() && glob_match_chars(&pattern[1..], &text[1..], case_insensitive),
        '[' => {
            let Some(close) = pattern.iter().position(|character| *character == ']') else {
                return !text.is_empty()
                    && equal_character(pattern[0], text[0], case_insensitive)
                    && glob_match_chars(&pattern[1..], &text[1..], case_insensitive);
            };
            if text.is_empty() {
                return false;
            }
            let mut class = &pattern[1..close];
            let negated = class
                .first()
                .is_some_and(|character| *character == '!' || *character == '^');
            if negated {
                class = &class[1..];
            }
            let mut matched = false;
            let mut index = 0;
            while index < class.len() {
                if index + 2 < class.len() && class[index + 1] == '-' {
                    let start = class[index];
                    let end = class[index + 2];
                    let candidate = text[0];
                    let candidate = if case_insensitive {
                        candidate.to_ascii_lowercase()
                    } else {
                        candidate
                    };
                    let start = if case_insensitive {
                        start.to_ascii_lowercase()
                    } else {
                        start
                    };
                    let end = if case_insensitive {
                        end.to_ascii_lowercase()
                    } else {
                        end
                    };
                    if start <= candidate && candidate <= end {
                        matched = true;
                    }
                    index += 3;
                } else {
                    if equal_character(class[index], text[0], case_insensitive) {
                        matched = true;
                    }
                    index += 1;
                }
            }
            (matched != negated)
                && glob_match_chars(&pattern[close + 1..], &text[1..], case_insensitive)
        }
        character => {
            !text.is_empty()
                && equal_character(character, text[0], case_insensitive)
                && glob_match_chars(&pattern[1..], &text[1..], case_insensitive)
        }
    }
}

fn equal_character(left: char, right: char, case_insensitive: bool) -> bool {
    if case_insensitive {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

fn stat(session: &Session, args: &[String]) -> CommandResult {
    let mut dereference = false;
    let mut operands = Vec::new();
    let mut options = true;

    for argument in args {
        if options && argument == "--" {
            options = false;
        } else if options && (argument == "-L" || argument == "--dereference") {
            dereference = true;
        } else if options && argument.starts_with('-') && argument.len() > 1 {
            return CommandResult::error(format!("stat: unknown option: {argument}"));
        } else {
            operands.push(argument.clone());
        }
    }

    if operands.is_empty() {
        return CommandResult::error("stat: missing operand");
    }

    let mut output = Vec::new();
    let mut errors = Vec::new();

    for operand in operands {
        let path = resolve_path(session, &operand);
        let metadata = if dereference {
            fs::metadata(&path)
        } else {
            fs::symlink_metadata(&path)
        };

        match metadata {
            Ok(metadata) => {
                let file_type = if metadata.file_type().is_symlink() {
                    "symbolic link"
                } else if metadata.is_dir() {
                    "directory"
                } else if metadata.is_file() {
                    "regular file"
                } else {
                    "special file"
                };

                let mut block = format!("  File: {operand}\n");
                block.push_str(&format!("  Type: {file_type}\n"));
                block.push_str(&format!("  Size: {} bytes\n", metadata.len()));
                block.push_str(&format!(
                    "  Permissions: {}\n",
                    format_permissions(&metadata)
                ));
                block.push_str(&format!(
                    "  Modified: {}\n",
                    format_time(metadata.modified().ok())
                ));
                block.push_str(&format!(
                    "  Accessed: {}",
                    format_time(metadata.accessed().ok())
                ));
                if let Ok(created) = metadata.created() {
                    block.push_str(&format!("\n  Created: {}", format_time(Some(created))));
                }
                output.push(block);
            }
            Err(error) => errors.push(format!(
                "stat: cannot stat '{operand}': {}",
                io_message(&error)
            )),
        }
    }

    CommandResult::from_parts(output.join("\n\n"), &errors)
}

fn format_time(time: Option<SystemTime>) -> String {
    time.map_or_else(
        || "unknown".to_owned(),
        |time| {
            let datetime: DateTime<Local> = time.into();
            datetime.format("%Y-%m-%d %H:%M:%S %Z").to_string()
        },
    )
}

#[cfg(unix)]
fn format_permissions(metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::PermissionsExt;

    let mode = metadata.permissions().mode();
    let mut text = String::with_capacity(10);
    text.push(if metadata.is_dir() { 'd' } else { '-' });
    for shift in [6, 3, 0] {
        text.push(if mode & (0o4 << shift) != 0 { 'r' } else { '-' });
        text.push(if mode & (0o2 << shift) != 0 { 'w' } else { '-' });
        text.push(if mode & (0o1 << shift) != 0 { 'x' } else { '-' });
    }
    format!("{text} ({:04o})", mode & 0o7777)
}

#[cfg(not(unix))]
fn format_permissions(metadata: &fs::Metadata) -> String {
    if metadata.permissions().readonly() {
        "readonly".to_owned()
    } else {
        "read-write".to_owned()
    }
}

fn which(session: &Session, registry: &BuiltinRegistry, args: &[String]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::error("which: missing command name");
    }

    let mut output = Vec::new();
    let mut errors = Vec::new();

    for name in args {
        if registry.get(name).is_some() {
            output.push(format!("{name}: shell builtin"));
            continue;
        }

        let external = ExternalCommand::new(name)
            .current_dir(session.working_directory().to_path_buf())
            .clear_env()
            .envs(session.child_environment(&[]));

        match external.resolve() {
            Ok(path) => output.push(path.display().to_string()),
            Err(_) => errors.push(format!("which: no {name} in PATH")),
        }
    }

    CommandResult::from_parts(output.join("\n"), &errors)
}

fn basename(args: &[String]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::error("basename: missing operand");
    }
    if args.len() > 2 {
        return CommandResult::error("basename: too many operands");
    }

    let path = Path::new(&args[0]);
    let mut name = path.file_name().map_or_else(
        || args[0].clone(),
        |name| name.to_string_lossy().into_owned(),
    );

    if let Some(suffix) = args.get(1)
        && let Some(stripped) = name.strip_suffix(suffix)
        && !stripped.is_empty()
    {
        name = stripped.to_owned();
    }

    CommandResult::ok(name)
}

fn dirname(args: &[String]) -> CommandResult {
    if args.len() != 1 {
        return CommandResult::error("dirname: expected exactly one operand");
    }

    let path = Path::new(&args[0]);
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    CommandResult::ok(parent.map_or_else(|| ".".to_owned(), |parent| parent.display().to_string()))
}

fn realpath(session: &Session, args: &[String]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::error("realpath: missing operand");
    }

    let mut output = Vec::new();
    let mut errors = Vec::new();

    for operand in args {
        let path = resolve_path(session, operand);
        match fs::canonicalize(&path) {
            Ok(path) => output.push(simplify_path(path).display().to_string()),
            Err(error) => errors.push(format!(
                "realpath: cannot resolve '{operand}': {}",
                io_message(&error)
            )),
        }
    }

    CommandResult::from_parts(output.join("\n"), &errors)
}

fn mkdir(session: &Session, args: &[String]) -> CommandResult {
    let (parents, operands) = match parse_mkdir_options(args) {
        Ok(parsed) => parsed,
        Err(message) => return CommandResult::error(message),
    };

    if operands.is_empty() {
        return CommandResult::error("mkdir: missing operand");
    }

    let mut errors = Vec::new();

    for operand in operands {
        let path = resolve_path(session, &operand);
        let result = if parents {
            fs::create_dir_all(&path)
        } else {
            fs::create_dir(&path)
        };

        if let Err(error) = result {
            errors.push(format!(
                "mkdir: cannot create directory '{operand}': {}",
                io_message(&error)
            ));
        }
    }

    CommandResult::from_parts(String::new(), &errors)
}

fn parse_mkdir_options(args: &[String]) -> Result<(bool, Vec<String>), String> {
    let mut parents = false;
    let mut operands = Vec::new();
    let mut options = true;

    for argument in args {
        if options && argument == "--" {
            options = false;
        } else if options && (argument == "-p" || argument == "--parents") {
            parents = true;
        } else if options && argument.starts_with('-') && argument.len() > 1 {
            return Err(format!("mkdir: unknown option: {argument}"));
        } else {
            operands.push(argument.clone());
        }
    }

    Ok((parents, operands))
}

fn touch(session: &Session, args: &[String]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::error("touch: missing file operand");
    }

    let now = SystemTime::now();
    let mut errors = Vec::new();

    for operand in args {
        let path = resolve_path(session, operand);
        let opened = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path);

        match opened {
            Ok(file) => {
                if let Err(error) = file.set_modified(now) {
                    errors.push(format!(
                        "touch: cannot update '{operand}': {}",
                        io_message(&error)
                    ));
                }
            }
            Err(error) => errors.push(format!(
                "touch: cannot touch '{operand}': {}",
                io_message(&error)
            )),
        }
    }

    CommandResult::from_parts(String::new(), &errors)
}

fn cat(session: &Session, stdin: Option<&str>, args: &[String]) -> CommandResult {
    if args.is_empty() {
        return match stdin {
            Some(input) => CommandResult::ok(input),
            None => CommandResult::error("cat: missing file operand"),
        };
    }

    let mut output = String::new();
    let mut errors = Vec::new();

    for operand in args {
        let path = resolve_path(session, operand);

        match fs::read(&path) {
            Ok(bytes) => output.push_str(&String::from_utf8_lossy(&bytes)),
            Err(error) => errors.push(format!(
                "cat: cannot read '{operand}': {}",
                io_message(&error)
            )),
        }
    }

    CommandResult::from_parts(output, &errors)
}

struct TextInput {
    label: Option<String>,
    text: String,
}

fn read_text_inputs(
    session: &Session,
    stdin: Option<&str>,
    files: &[String],
    command: &str,
) -> (Vec<TextInput>, Vec<String>) {
    let mut inputs = Vec::new();
    let mut errors = Vec::new();

    if files.is_empty() {
        if let Some(input) = stdin {
            inputs.push(TextInput {
                label: None,
                text: input.to_owned(),
            });
        } else {
            errors.push(format!("{command}: missing file operand"));
        }
        return (inputs, errors);
    }

    for file in files {
        let path = resolve_path(session, file);
        match fs::read(&path) {
            Ok(bytes) => inputs.push(TextInput {
                label: Some(file.clone()),
                text: String::from_utf8_lossy(&bytes).into_owned(),
            }),
            Err(error) => errors.push(format!(
                "{command}: cannot read '{file}': {}",
                io_message(&error)
            )),
        }
    }

    (inputs, errors)
}

fn parse_line_options(
    command: &str,
    args: &[String],
) -> Result<(usize, bool, Vec<String>), String> {
    let mut count = 10usize;
    let mut quiet = false;
    let mut operands = Vec::new();
    let mut options = true;
    let mut index = 0;

    while index < args.len() {
        let argument = &args[index];
        if options && argument == "--" {
            options = false;
        } else if options && (argument == "-q" || argument == "--quiet") {
            quiet = true;
        } else if options && (argument == "-n" || argument == "--lines") {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(format!("{command}: {argument} requires a number"));
            };
            count = value
                .parse::<usize>()
                .map_err(|_| format!("{command}: invalid line count: {value}"))?;
        } else if options && argument.starts_with("-n") && argument.len() > 2 {
            count = argument[2..]
                .parse::<usize>()
                .map_err(|_| format!("{command}: invalid line count: {}", &argument[2..]))?;
        } else if options && argument.starts_with("--lines=") {
            let value = &argument["--lines=".len()..];
            count = value
                .parse::<usize>()
                .map_err(|_| format!("{command}: invalid line count: {value}"))?;
        } else if options && argument.starts_with('-') && argument.len() > 1 {
            return Err(format!("{command}: unknown option: {argument}"));
        } else {
            operands.push(argument.clone());
        }
        index += 1;
    }

    Ok((count, quiet, operands))
}

fn head(session: &Session, stdin: Option<&str>, args: &[String]) -> CommandResult {
    let (count, quiet, operands) = match parse_line_options("head", args) {
        Ok(parsed) => parsed,
        Err(message) => return CommandResult::error(message),
    };
    let (inputs, errors) = read_text_inputs(session, stdin, &operands, "head");
    let multiple = inputs.len() > 1;
    let mut output = String::new();

    for input in inputs {
        if multiple && !quiet {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(&format!(
                "==> {} <==\n",
                input.label.as_deref().unwrap_or("standard input")
            ));
        }

        output.push_str(
            &input
                .text
                .split_inclusive('\n')
                .take(count)
                .collect::<String>(),
        );
        if !output.ends_with('\n') && !output.is_empty() {
            output.push('\n');
        }
    }

    CommandResult::from_parts(output, &errors)
}

fn tail(session: &Session, stdin: Option<&str>, args: &[String]) -> CommandResult {
    let (count, quiet, operands) = match parse_line_options("tail", args) {
        Ok(parsed) => parsed,
        Err(message) => return CommandResult::error(message),
    };
    let (inputs, errors) = read_text_inputs(session, stdin, &operands, "tail");
    let multiple = inputs.len() > 1;
    let mut output = String::new();

    for input in inputs {
        if multiple && !quiet {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(&format!(
                "==> {} <==\n",
                input.label.as_deref().unwrap_or("standard input")
            ));
        }

        let lines: Vec<&str> = input.text.split_inclusive('\n').collect();
        let start = lines.len().saturating_sub(count);
        output.push_str(&lines[start..].concat());
        if !output.ends_with('\n') && !output.is_empty() {
            output.push('\n');
        }
    }

    CommandResult::from_parts(output, &errors)
}

fn wc(session: &Session, stdin: Option<&str>, args: &[String]) -> CommandResult {
    let mut show_lines = false;
    let mut show_words = false;
    let mut show_chars = false;
    let mut show_bytes = false;
    let mut operands = Vec::new();
    let mut options = true;

    for argument in args {
        if options && argument == "--" {
            options = false;
        } else if options && argument.starts_with('-') && argument.len() > 1 {
            for flag in argument[1..].chars() {
                match flag {
                    'l' => show_lines = true,
                    'w' => show_words = true,
                    'm' => show_chars = true,
                    'c' => show_bytes = true,
                    _ => return CommandResult::error(format!("wc: unknown option: -{flag}")),
                }
            }
        } else {
            operands.push(argument.clone());
        }
    }

    if !show_lines && !show_words && !show_chars && !show_bytes {
        show_lines = true;
        show_words = true;
        show_bytes = true;
    }

    let (inputs, errors) = read_text_inputs(session, stdin, &operands, "wc");
    let multiple = inputs.len() > 1;
    let mut output = Vec::new();
    let mut totals = [0usize; 4];

    for input in inputs {
        let lines = input.text.bytes().filter(|byte| *byte == b'\n').count();
        let words = input.text.split_whitespace().count();
        let chars = input.text.chars().count();
        let bytes = input.text.len();
        let values = [lines, words, chars, bytes];

        for (index, value) in values.iter().enumerate() {
            totals[index] += value;
        }

        let mut fields = Vec::new();
        if show_lines {
            fields.push(lines.to_string());
        }
        if show_words {
            fields.push(words.to_string());
        }
        if show_chars {
            fields.push(chars.to_string());
        }
        if show_bytes {
            fields.push(bytes.to_string());
        }
        if let Some(label) = input.label {
            fields.push(label);
        }
        output.push(fields.join(" "));
    }

    if multiple {
        let mut fields = Vec::new();
        if show_lines {
            fields.push(totals[0].to_string());
        }
        if show_words {
            fields.push(totals[1].to_string());
        }
        if show_chars {
            fields.push(totals[2].to_string());
        }
        if show_bytes {
            fields.push(totals[3].to_string());
        }
        fields.push("total".to_owned());
        output.push(fields.join(" "));
    }

    CommandResult::from_parts(output.join("\n"), &errors)
}

fn grep(session: &Session, stdin: Option<&str>, args: &[String]) -> CommandResult {
    let mut ignore_case = false;
    let mut invert = false;
    let mut line_numbers = false;
    let mut count_only = false;
    let mut quiet = false;
    let mut pattern = None;
    let mut operands = Vec::new();
    let mut options = true;
    let mut index = 0;

    while index < args.len() {
        let argument = &args[index];
        if options && argument == "--" {
            options = false;
        } else if options && argument == "-e" {
            index += 1;
            let Some(value) = args.get(index) else {
                return CommandResult::error("grep: -e requires a pattern");
            };
            pattern = Some(value.clone());
        } else if options && argument.starts_with('-') && argument.len() > 1 {
            for flag in argument[1..].chars() {
                match flag {
                    'i' => ignore_case = true,
                    'v' => invert = true,
                    'n' => line_numbers = true,
                    'c' => count_only = true,
                    'q' => quiet = true,
                    'F' => {}
                    _ => return CommandResult::error(format!("grep: unknown option: -{flag}")),
                }
            }
        } else if pattern.is_none() {
            pattern = Some(argument.clone());
        } else {
            operands.push(argument.clone());
        }
        index += 1;
    }

    let Some(pattern) = pattern else {
        return CommandResult::error("grep: missing pattern");
    };

    let (inputs, errors) = read_text_inputs(session, stdin, &operands, "grep");
    let multiple = inputs.len() > 1;
    let mut output = Vec::new();
    let mut matched_any = false;

    for input in inputs {
        let mut selected = Vec::new();
        for (line_index, line) in input.text.lines().enumerate() {
            let matches = if ignore_case {
                line.to_lowercase().contains(&pattern.to_lowercase())
            } else {
                line.contains(&pattern)
            };
            if matches != invert {
                selected.push((line_index + 1, line));
            }
        }

        if !selected.is_empty() {
            matched_any = true;
        }

        if quiet {
            continue;
        }

        if count_only {
            let label = input
                .label
                .as_deref()
                .map_or_else(String::new, |label| format!("{label}:"));
            output.push(format!("{label}{}", selected.len()));
            continue;
        }

        for (line_number, line) in selected {
            let mut prefix = String::new();
            if multiple {
                if let Some(label) = &input.label {
                    prefix.push_str(label);
                    prefix.push(':');
                }
            }
            if line_numbers {
                prefix.push_str(&format!("{line_number}:"));
            }
            output.push(format!("{prefix}{line}"));
        }
    }

    let mut text = output.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }

    CommandResult::from_parts_status(text, &errors, matched_any)
}

fn sort(session: &Session, stdin: Option<&str>, args: &[String]) -> CommandResult {
    let mut reverse = false;
    let mut numeric = false;
    let mut unique = false;
    let mut fold_case = false;
    let mut operands = Vec::new();
    let mut options = true;

    for argument in args {
        if options && argument == "--" {
            options = false;
        } else if options && argument.starts_with('-') && argument.len() > 1 {
            for flag in argument[1..].chars() {
                match flag {
                    'r' => reverse = true,
                    'n' => numeric = true,
                    'u' => unique = true,
                    'f' => fold_case = true,
                    _ => return CommandResult::error(format!("sort: unknown option: -{flag}")),
                }
            }
        } else {
            operands.push(argument.clone());
        }
    }

    let (inputs, errors) = read_text_inputs(session, stdin, &operands, "sort");
    let mut lines: Vec<String> = inputs
        .into_iter()
        .flat_map(|input| input.text.lines().map(str::to_owned).collect::<Vec<_>>())
        .collect();

    lines.sort_by(|left, right| {
        let ordering = if numeric {
            let left = left.trim().parse::<f64>().unwrap_or(f64::NAN);
            let right = right.trim().parse::<f64>().unwrap_or(f64::NAN);
            left.partial_cmp(&right)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else if fold_case {
            left.to_lowercase().cmp(&right.to_lowercase())
        } else {
            left.cmp(right)
        };
        if reverse {
            ordering.reverse()
        } else {
            ordering
        }
    });

    if unique {
        lines.dedup();
    }

    let mut output = lines.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }

    CommandResult::from_parts(output, &errors)
}

fn uniq(session: &Session, stdin: Option<&str>, args: &[String]) -> CommandResult {
    let mut count = false;
    let mut only_duplicates = false;
    let mut only_unique = false;
    let mut operands = Vec::new();
    let mut options = true;

    for argument in args {
        if options && argument == "--" {
            options = false;
        } else if options && argument.starts_with('-') && argument.len() > 1 {
            for flag in argument[1..].chars() {
                match flag {
                    'c' => count = true,
                    'd' => only_duplicates = true,
                    'u' => only_unique = true,
                    _ => return CommandResult::error(format!("uniq: unknown option: -{flag}")),
                }
            }
        } else {
            operands.push(argument.clone());
        }
    }

    let (inputs, errors) = read_text_inputs(session, stdin, &operands, "uniq");
    let lines: Vec<&str> = inputs.iter().flat_map(|input| input.text.lines()).collect();

    let mut output = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let mut end = index + 1;
        while end < lines.len() && lines[end] == lines[index] {
            end += 1;
        }
        let repetitions = end - index;
        let include = (!only_duplicates || repetitions > 1) && (!only_unique || repetitions == 1);
        if include {
            if count {
                output.push(format!("{repetitions} {}", lines[index]));
            } else {
                output.push(lines[index].to_owned());
            }
        }
        index = end;
    }

    let mut text = output.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    CommandResult::from_parts(text, &errors)
}

fn echo(args: &[String]) -> CommandResult {
    CommandResult::ok(format!("{}\n", args.join(" ")))
}

/// `print` is the scripting form of `echo`: it joins its arguments with a
/// single space and terminates the line.
fn print_builtin(args: &[String]) -> CommandResult {
    CommandResult::ok(format!("{}\n", args.join(" ")))
}

/// `input <variable>` reads one line of standard input into a shell variable.
///
/// Standard input comes from a pipeline or a `<` redirection. Built-ins never
/// read the terminal directly, so interactive use without redirection reports
/// a clear error instead of competing with the line editor for keystrokes.
fn input_builtin(session: &mut Session, stdin: Option<&str>, args: &[String]) -> CommandResult {
    let [name] = args else {
        return CommandResult::error("input: usage: input <variable>");
    };

    if !shell_core::is_valid_variable_name(name) {
        return CommandResult::error(format!("input: '{name}' is not a valid variable name"));
    }

    let Some(stdin) = stdin else {
        return CommandResult::error(
            "input: standard input is not available; pipe or redirect input",
        );
    };

    let line = stdin
        .split_inclusive('\n')
        .next()
        .unwrap_or_default()
        .trim_end_matches(['\n', '\r']);

    session.set_variable(name, line.to_owned(), false);
    CommandResult::silent()
}

/// `len <value>` prints the number of characters (Unicode scalar values).
fn len_builtin(args: &[String]) -> CommandResult {
    let Some(value) = single_argument(args) else {
        return CommandResult::error("len: usage: len <value>");
    };

    CommandResult::ok(value.chars().count().to_string())
}

/// `type <value>` classifies a value as `bool`, `int`, `float`, or `string`.
fn type_builtin(args: &[String]) -> CommandResult {
    let Some(value) = single_argument(args) else {
        return CommandResult::error("type: usage: type <value>");
    };

    CommandResult::ok(value_kind(value))
}

/// `str <value>` converts any value to its string form, which is already how
/// arguments arrive, so it is an explicit identity conversion.
fn str_builtin(args: &[String]) -> CommandResult {
    let Some(value) = single_argument(args) else {
        return CommandResult::error("str: usage: str <value>");
    };

    CommandResult::ok(value)
}

/// `int <value>` parses a decimal integer and prints its canonical form.
fn int_builtin(args: &[String]) -> CommandResult {
    let Some(value) = single_argument(args) else {
        return CommandResult::error("int: usage: int <value>");
    };

    match value.trim().parse::<i64>() {
        Ok(number) => CommandResult::ok(number.to_string()),
        Err(_) => CommandResult::error(format!("int: cannot convert '{value}' to an integer")),
    }
}

/// `float <value>` parses a floating-point number and prints its canonical form.
fn float_builtin(args: &[String]) -> CommandResult {
    let Some(value) = single_argument(args) else {
        return CommandResult::error("float: usage: float <value>");
    };

    match value.trim().parse::<f64>() {
        Ok(number) if number.is_finite() => CommandResult::ok(number.to_string()),
        Ok(_) => CommandResult::error(format!("float: '{value}' is not a finite number")),
        Err(_) => CommandResult::error(format!("float: cannot convert '{value}' to a number")),
    }
}

/// `bool <value>` parses a boolean and prints `true` or `false`.
fn bool_builtin(args: &[String]) -> CommandResult {
    let Some(value) = single_argument(args) else {
        return CommandResult::error("bool: usage: bool <value>");
    };

    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => CommandResult::ok("true"),
        "false" | "no" | "off" | "0" => CommandResult::ok("false"),
        _ => CommandResult::error(format!("bool: cannot convert '{value}' to a boolean")),
    }
}

/// Returns the single operand of a value command, or `None` when the argument
/// count is wrong.
fn single_argument(args: &[String]) -> Option<&str> {
    match args {
        [value] => Some(value.as_str()),
        _ => None,
    }
}

/// Classifies one value using the same rules the conversion commands follow.
fn value_kind(value: &str) -> &'static str {
    let trimmed = value.trim();

    if matches!(
        trimmed.to_ascii_lowercase().as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off"
    ) {
        return "bool";
    }

    if trimmed.parse::<i64>().is_ok() {
        return "int";
    }

    if trimmed.parse::<f64>().is_ok_and(f64::is_finite) {
        return "float";
    }

    "string"
}

fn cp(session: &Session, args: &[String]) -> CommandResult {
    let (recursive, operands) = match parse_copy_options(args) {
        Ok(parsed) => parsed,
        Err(message) => return CommandResult::error(message),
    };

    if operands.len() < 2 {
        return CommandResult::error("cp: missing destination operand");
    }

    let destination = resolve_path(session, operands.last().expect("checked above"));
    let sources = &operands[..operands.len() - 1];

    if sources.len() > 1 && !destination.is_dir() {
        return CommandResult::error(format!(
            "cp: target '{}' is not a directory",
            operands.last().expect("checked above")
        ));
    }

    let mut errors = Vec::new();

    for source in sources {
        let source_path = resolve_path(session, source);

        if let Err(error) = copy_entry(&source_path, &destination, recursive) {
            errors.push(format!("cp: cannot copy '{source}': {error}"));
        }
    }

    CommandResult::from_parts(String::new(), &errors)
}

fn parse_copy_options(args: &[String]) -> Result<(bool, Vec<String>), String> {
    let mut recursive = false;
    let mut operands = Vec::new();
    let mut options = true;

    for argument in args {
        if options && argument == "--" {
            options = false;
        } else if options && (argument == "-r" || argument == "-R" || argument == "--recursive") {
            recursive = true;
        } else if options && argument.starts_with('-') && argument.len() > 1 {
            return Err(format!("cp: unknown option: {argument}"));
        } else {
            operands.push(argument.clone());
        }
    }

    Ok((recursive, operands))
}

fn copy_entry(source: &Path, destination: &Path, recursive: bool) -> Result<(), String> {
    let metadata = fs::metadata(source).map_err(|error| io_message(&error))?;
    let destination = if destination.is_dir() {
        let name = source
            .file_name()
            .ok_or_else(|| "source has no file name".to_owned())?;
        destination.join(name)
    } else {
        destination.to_path_buf()
    };

    if source == destination.as_path() {
        return Err("source and destination are the same file".to_owned());
    }

    if metadata.is_dir() {
        if !recursive {
            return Err("is a directory (use -r to copy directories)".to_owned());
        }

        if destination.starts_with(source) {
            return Err("cannot copy a directory into itself".to_owned());
        }

        copy_directory(source, &destination).map_err(|error| io_message(&error))
    } else {
        fs::copy(source, &destination)
            .map(|_| ())
            .map_err(|error| io_message(&error))
    }
}

fn copy_directory(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = destination.join(entry.file_name());

        if file_type.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }

    Ok(())
}

fn mv(session: &Session, args: &[String]) -> CommandResult {
    let operands = match parse_plain_operands("mv", args) {
        Ok(operands) => operands,
        Err(message) => return CommandResult::error(message),
    };

    if operands.len() < 2 {
        return CommandResult::error("mv: missing destination operand");
    }

    let destination = resolve_path(session, operands.last().expect("checked above"));
    let sources = &operands[..operands.len() - 1];

    if sources.len() > 1 && !destination.is_dir() {
        return CommandResult::error(format!(
            "mv: target '{}' is not a directory",
            operands.last().expect("checked above")
        ));
    }

    let mut errors = Vec::new();

    for source in sources {
        let source_path = resolve_path(session, source);

        if let Err(error) = move_entry(&source_path, &destination) {
            errors.push(format!("mv: cannot move '{source}': {error}"));
        }
    }

    CommandResult::from_parts(String::new(), &errors)
}

fn move_entry(source: &Path, destination: &Path) -> Result<(), String> {
    let destination = if destination.is_dir() {
        let name = source
            .file_name()
            .ok_or_else(|| "source has no file name".to_owned())?;
        destination.join(name)
    } else {
        destination.to_path_buf()
    };

    if source == destination.as_path() {
        return Err("source and destination are the same file".to_owned());
    }

    if destination.starts_with(source) {
        return Err("cannot move a directory into itself".to_owned());
    }

    match fs::rename(source, &destination) {
        Ok(()) => Ok(()),
        Err(first_error) => {
            if destination.is_file() {
                fs::remove_file(&destination).map_err(|error| io_message(&error))?;
                if fs::rename(source, &destination).is_ok() {
                    return Ok(());
                }
            }

            move_across_filesystems(source, &destination).map_err(|fallback| {
                format!(
                    "{} (rename failed: {})",
                    io_message(&fallback),
                    io_message(&first_error)
                )
            })
        }
    }
}

fn move_across_filesystems(source: &Path, destination: &Path) -> io::Result<()> {
    let metadata = fs::metadata(source)?;

    if metadata.is_dir() {
        copy_directory(source, destination)?;
        fs::remove_dir_all(source)
    } else {
        fs::copy(source, destination)?;
        fs::remove_file(source)
    }
}

fn rm(session: &Session, args: &[String]) -> CommandResult {
    let (recursive, force, operands) = match parse_rm_options(args) {
        Ok(parsed) => parsed,
        Err(message) => return CommandResult::error(message),
    };

    if operands.is_empty() {
        return if force {
            CommandResult::silent()
        } else {
            CommandResult::error("rm: missing operand")
        };
    }

    let mut errors = Vec::new();

    for operand in operands {
        let path = resolve_path(session, &operand);

        if path.parent().is_none() {
            errors.push(format!("rm: refusing to remove '{operand}'"));
            continue;
        }

        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if force && error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                errors.push(format!(
                    "rm: cannot remove '{operand}': {}",
                    io_message(&error)
                ));
                continue;
            }
        };

        let result = if metadata.is_dir() {
            if recursive {
                fs::remove_dir_all(&path)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::IsADirectory,
                    "is a directory (use -r to remove directories)",
                ))
            }
        } else {
            fs::remove_file(&path)
        };

        if let Err(error) = result {
            errors.push(format!(
                "rm: cannot remove '{operand}': {}",
                io_message(&error)
            ));
        }
    }

    CommandResult::from_parts(String::new(), &errors)
}

fn parse_rm_options(args: &[String]) -> Result<(bool, bool, Vec<String>), String> {
    let mut recursive = false;
    let mut force = false;
    let mut operands = Vec::new();
    let mut options = true;

    for argument in args {
        if options && argument == "--" {
            options = false;
        } else if options && argument.starts_with("--") {
            match argument.as_str() {
                "--recursive" => recursive = true,
                "--force" => force = true,
                _ => return Err(format!("rm: unknown option: {argument}")),
            }
        } else if options && argument.starts_with('-') && argument.len() > 1 {
            for flag in argument[1..].chars() {
                match flag {
                    'r' | 'R' => recursive = true,
                    'f' => force = true,
                    _ => return Err(format!("rm: unknown option: -{flag}")),
                }
            }
        } else {
            operands.push(argument.clone());
        }
    }

    Ok((recursive, force, operands))
}

fn parse_plain_operands(name: &str, args: &[String]) -> Result<Vec<String>, String> {
    let mut operands = Vec::new();
    let mut options = true;

    for argument in args {
        if options && argument == "--" {
            options = false;
        } else if options && argument.starts_with('-') && argument.len() > 1 {
            return Err(format!("{name}: unknown option: {argument}"));
        } else {
            operands.push(argument.clone());
        }
    }

    Ok(operands)
}

fn jobs(session: &Session, args: &[String]) -> CommandResult {
    if !args.is_empty() {
        return CommandResult::error("jobs: no options are supported");
    }

    let table = session.jobs();
    let mut table = table
        .lock()
        .expect("job table mutex should not be poisoned");
    let snapshot = table.snapshot();

    if snapshot.is_empty() {
        return CommandResult::ok("No background jobs.");
    }

    let output = snapshot
        .into_iter()
        .map(|job| {
            let status = match job.status {
                JobStatus::Running => "Running".to_owned(),
                JobStatus::Done => format!("Done({})", job.exit_code.unwrap_or(0)),
                JobStatus::Failed => format!("Failed({})", job.exit_code.unwrap_or(1)),
                JobStatus::Killed => "Killed".to_owned(),
            };
            format!("[{:<3}] {:<10} {}", job.id, status, job.command)
        })
        .collect::<Vec<_>>()
        .join("\n");

    CommandResult::ok(output)
}

fn wait(session: &Session, args: &[String]) -> CommandResult {
    let requested = if args.is_empty() {
        None
    } else {
        let mut ids = Vec::with_capacity(args.len());
        for argument in args {
            match argument.parse::<u32>() {
                Ok(id) => ids.push(id),
                Err(_) => {
                    return CommandResult::error(format!("wait: invalid job id: {argument}"));
                }
            }
        }
        Some(ids)
    };

    let table = session.jobs();

    if let Some(ids) = &requested {
        let mut table_guard = table
            .lock()
            .expect("job table mutex should not be poisoned");
        for id in ids {
            if table_guard.get(*id).is_none() {
                return CommandResult::error(format!("wait: job {id} does not exist"));
            }
        }
    }

    loop {
        let finished = {
            let mut table_guard = table
                .lock()
                .expect("job table mutex should not be poisoned");
            match &requested {
                Some(ids) => ids.iter().all(|id| {
                    table_guard
                        .get(*id)
                        .is_some_and(|job| !job.status.is_running())
                }),
                None => table_guard.all_finished(),
            }
        };

        if finished {
            break;
        }

        std::thread::sleep(Duration::from_millis(20));
    }

    let mut table_guard = table
        .lock()
        .expect("job table mutex should not be poisoned");
    let infos = match &requested {
        Some(ids) => ids.iter().filter_map(|id| table_guard.get(*id)).collect(),
        None => table_guard.snapshot(),
    };
    let success = infos.iter().all(|job| job.status == JobStatus::Done);

    for job in &infos {
        table_guard.remove(job.id);
    }

    CommandResult::status(success)
}

fn kill(session: &Session, args: &[String]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::error("kill: missing job id");
    }

    let table = session.jobs();
    let mut table = table
        .lock()
        .expect("job table mutex should not be poisoned");
    let mut output = Vec::new();
    let mut errors = Vec::new();

    for argument in args {
        let Ok(id) = argument.parse::<u32>() else {
            errors.push(format!("kill: invalid job id: {argument}"));
            continue;
        };

        match table.kill(id) {
            Ok(job) => output.push(format!("Terminated job {id}: {}", job.command)),
            Err(message) => errors.push(format!("kill: {message}")),
        }
    }

    CommandResult::from_parts(output.join("\n"), &errors)
}

fn fg(session: &Session, args: &[String]) -> CommandResult {
    if args.len() > 1 {
        return CommandResult::error("fg: too many operands");
    }

    let table = session.jobs();
    let id = {
        let mut table_guard = table
            .lock()
            .expect("job table mutex should not be poisoned");
        match args.first() {
            Some(argument) => match argument.parse::<u32>() {
                Ok(id) => id,
                Err(_) => return CommandResult::error(format!("fg: invalid job id: {argument}")),
            },
            None => match table_guard.last_id() {
                Some(id) => id,
                None => return CommandResult::error("fg: no current job"),
            },
        }
    };

    loop {
        let finished = {
            let mut table_guard = table
                .lock()
                .expect("job table mutex should not be poisoned");
            match table_guard.get(id) {
                Some(job) => !job.status.is_running(),
                None => return CommandResult::error(format!("fg: job {id} does not exist")),
            }
        };

        if finished {
            break;
        }

        std::thread::sleep(Duration::from_millis(20));
    }

    let mut table_guard = table
        .lock()
        .expect("job table mutex should not be poisoned");
    let Some(job) = table_guard.remove(id) else {
        return CommandResult::error(format!("fg: job {id} does not exist"));
    };

    let result = CommandResult::status(job.status == JobStatus::Done);
    if job.status == JobStatus::Done {
        CommandResult::ok(job.command)
    } else {
        CommandResult {
            stdout: job.command,
            stderr: String::new(),
            success: result.success,
            action: CommandAction::Continue,
            exit_code: None,
        }
    }
}

fn bg(session: &Session, args: &[String]) -> CommandResult {
    if args.len() > 1 {
        return CommandResult::error("bg: too many operands");
    }

    let table = session.jobs();
    let mut table = table
        .lock()
        .expect("job table mutex should not be poisoned");
    let id = match args.first() {
        Some(argument) => match argument.parse::<u32>() {
            Ok(id) => id,
            Err(_) => return CommandResult::error(format!("bg: invalid job id: {argument}")),
        },
        None => match table.last_id() {
            Some(id) => id,
            None => return CommandResult::error("bg: no current job"),
        },
    };

    match table.get(id) {
        Some(job) if job.status == JobStatus::Running => {
            CommandResult::ok(format!("Job {id} is already running: {}", job.command))
        }
        Some(job) => CommandResult::error(format!(
            "bg: job {id} is not running (status: {:?})",
            job.status
        )),
        None => CommandResult::error(format!("bg: job {id} does not exist")),
    }
}

const MAX_SCRIPT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SCRIPT_DEPTH: usize = 32;

fn run_script(
    session: &mut Session,
    registry: &BuiltinRegistry,
    history: &[String],
    options: &ProcessOptions,
    args: &[String],
) -> CommandResult {
    if args.is_empty() {
        return CommandResult::error("run: missing script file");
    }

    if !session.enter_script(MAX_SCRIPT_DEPTH) {
        return CommandResult::error(format!(
            "run: maximum script nesting depth of {MAX_SCRIPT_DEPTH} exceeded"
        ));
    }

    let result = run_script_inner(session, registry, history, options, args);
    session.leave_script();
    result
}

fn run_script_inner(
    session: &mut Session,
    registry: &BuiltinRegistry,
    history: &[String],
    options: &ProcessOptions,
    args: &[String],
) -> CommandResult {
    let script = &args[0];
    let path = resolve_path(session, script);

    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return CommandResult::error(format!(
                "run: cannot access '{script}': {}",
                io_message(&error)
            ));
        }
    };

    if metadata.len() > MAX_SCRIPT_BYTES {
        return CommandResult::error(format!(
            "run: '{script}' exceeds the {MAX_SCRIPT_BYTES}-byte script limit"
        ));
    }

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            return CommandResult::error(format!(
                "run: cannot read '{script}': {}",
                io_message(&error)
            ));
        }
    };

    let tokens = match lex_spanned(&source) {
        Ok(tokens) => tokens,
        Err(error) => {
            return CommandResult::error(format!("run: {script}: {error}"));
        }
    };

    let ast = match parse(&tokens) {
        Ok(ast) => ast,
        Err(error) => {
            return CommandResult::error(format!("run: {script}: {error}"));
        }
    };

    if ast.is_empty() {
        return CommandResult::silent();
    }

    let outcome = crate::eval::evaluate(registry, history, session, &ast, options);
    CommandResult::from_outcome(outcome)
}

fn history(session: &mut Session, entries: &[String], args: &[String]) -> CommandResult {
    if args.is_empty() {
        let width = entries.len().to_string().len();
        let output = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| format!("{:>width$}  {entry}", index + 1))
            .collect::<Vec<_>>()
            .join("\n");

        return CommandResult::ok(output);
    }

    match args[0].as_str() {
        "clear" => {
            // Clear in-memory history (dispatcher handles this)
            // Also clear the history file if it exists
            if let Some(history_file) = session.config().history_path() {
                let _ = std::fs::remove_file(history_file);
            }
            CommandResult::clear_history("History cleared.".to_string())
        }
        "search" => {
            if args.len() < 2 {
                return CommandResult::error("history search: missing keyword");
            }
            let keyword = &args[1];
            let width = entries.len().to_string().len();
            let output = entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.contains(keyword))
                .map(|(index, entry)| format!("{:>width$}  {entry}", index + 1))
                .collect::<Vec<_>>()
                .join("\n");

            if output.is_empty() {
                CommandResult::ok(format!("No history entries matching '{keyword}'."))
            } else {
                CommandResult::ok(output)
            }
        }
        "limit" => {
            if args.len() < 2 {
                return CommandResult::error("history limit: missing number");
            }
            let limit_str = &args[1];
            let limit = match limit_str.parse::<usize>() {
                Ok(l) => l,
                Err(_) => {
                    return CommandResult::error(format!(
                        "history limit: invalid number: {limit_str}"
                    ));
                }
            };

            if limit == 0 {
                return CommandResult::error("history limit: must be greater than zero");
            }

            // This only affects the current session. To persist, update the
            // configuration file.
            session.set_history_limit(limit);
            CommandResult::ok(format!("History limit set to {limit} for this session."))
        }
        _ => CommandResult::error(format!(
            "history: unknown subcommand '{}'. Use: clear, search, limit",
            args[0]
        )),
    }
}

fn test_builtin(session: &Session, name: &str, args: &[String]) -> CommandResult {
    let mut tokens = args.to_vec();

    if name == "[" {
        if tokens.last().is_none_or(|token| token != "]") {
            return CommandResult::error("[ : missing closing ]");
        }
        tokens.pop();
    }

    match eval_test(&tokens, session) {
        Ok(result) => CommandResult::status(result),
        Err(message) => CommandResult::error(format!("{name}: {message}")),
    }
}

fn eval_test(tokens: &[String], session: &Session) -> Result<bool, String> {
    if tokens.is_empty() {
        return Ok(false);
    }

    if tokens[0] == "!" {
        return Ok(!eval_test(&tokens[1..], session)?);
    }

    if tokens.len() == 1 {
        return Ok(!tokens[0].is_empty());
    }

    if tokens.len() == 2 {
        return eval_test_unary(&tokens[0], &tokens[1], session);
    }

    if let Some(position) = find_test_operator(tokens, "-o") {
        return Ok(eval_test(&tokens[..position], session)?
            || eval_test(&tokens[position + 1..], session)?);
    }

    if let Some(position) = find_test_operator(tokens, "-a") {
        return Ok(eval_test(&tokens[..position], session)?
            && eval_test(&tokens[position + 1..], session)?);
    }

    if tokens.len() == 3 {
        return eval_test_binary(&tokens[0], &tokens[1], &tokens[2]);
    }

    Err("malformed conditional expression".to_owned())
}

fn find_test_operator(tokens: &[String], operator: &str) -> Option<usize> {
    tokens
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, token)| token.as_str() == operator)
        .map(|(index, _)| index)
}

fn eval_test_unary(operator: &str, operand: &str, session: &Session) -> Result<bool, String> {
    let path = resolve_path(session, operand);
    let metadata = || fs::symlink_metadata(&path);

    match operator {
        "-e" | "-a" => Ok(metadata().is_ok()),
        "-f" => Ok(metadata().is_ok_and(|metadata| metadata.is_file())),
        "-d" => Ok(metadata().is_ok_and(|metadata| metadata.is_dir())),
        "-L" | "-h" => Ok(metadata().is_ok_and(|metadata| metadata.file_type().is_symlink())),
        "-s" => Ok(metadata().is_ok_and(|metadata| metadata.len() > 0)),
        "-r" => Ok(metadata().is_ok()),
        "-w" => Ok(metadata().is_ok_and(|metadata| !metadata.permissions().readonly())),
        "-x" => Ok(metadata().is_ok_and(|metadata| is_executable(&metadata))),
        "-z" => Ok(operand.is_empty()),
        "-n" => Ok(!operand.is_empty()),
        _ => Err(format!("unknown unary operator: {operator}")),
    }
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    true
}

fn eval_test_binary(left: &str, operator: &str, right: &str) -> Result<bool, String> {
    match operator {
        "=" | "==" => Ok(left == right),
        "!=" => Ok(left != right),
        "-eq" | "-ne" | "-lt" | "-le" | "-gt" | "-ge" => {
            let left = left
                .parse::<i64>()
                .map_err(|_| format!("integer expression expected: {left}"))?;
            let right = right
                .parse::<i64>()
                .map_err(|_| format!("integer expression expected: {right}"))?;
            Ok(match operator {
                "-eq" => left == right,
                "-ne" => left != right,
                "-lt" => left < right,
                "-le" => left <= right,
                "-gt" => left > right,
                "-ge" => left >= right,
                _ => unreachable!(),
            })
        }
        _ => Err(format!("unknown binary operator: {operator}")),
    }
}

fn expr_builtin(args: &[String]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::error("expr: missing expression");
    }

    let mut parser = ExprParser {
        tokens: args,
        position: 0,
    };

    match parser.parse_comparison() {
        Ok(value) => {
            if parser.position != args.len() {
                return CommandResult::error(format!(
                    "expr: unexpected token: {}",
                    args[parser.position]
                ));
            }
            CommandResult::ok(value.render())
        }
        Err(message) => CommandResult::error(format!("expr: {message}")),
    }
}

struct ExprParser<'a> {
    tokens: &'a [String],
    position: usize,
}

#[derive(Clone)]
enum ExprValue {
    Number(f64),
    Text(String),
}

impl ExprValue {
    fn number(&self) -> Result<f64, String> {
        match self {
            Self::Number(value) => Ok(*value),
            Self::Text(value) => value
                .parse::<f64>()
                .map_err(|_| format!("not a number: {value}")),
        }
    }

    fn render(self) -> String {
        match self {
            Self::Number(value) => {
                if value.fract() == 0.0 && value.abs() <= i64::MAX as f64 {
                    format!("{}", value as i64)
                } else {
                    format!("{value}")
                }
            }
            Self::Text(value) => value,
        }
    }
}

impl<'a> ExprParser<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.tokens.get(self.position).map(String::as_str)
    }

    fn advance(&mut self) -> Option<&'a str> {
        let token = self.peek()?;
        self.position += 1;
        Some(token)
    }

    fn parse_comparison(&mut self) -> Result<ExprValue, String> {
        let mut left = self.parse_additive()?;

        while let Some(operator) = self.peek() {
            if !matches!(operator, "=" | "==" | "!=" | "<" | "<=" | ">" | ">=") {
                break;
            }
            self.advance();
            let right = self.parse_additive()?;
            let result = match operator {
                "=" | "==" => compare_values(left, right, |ordering| ordering.is_eq()),
                "!=" => compare_values(left, right, |ordering| !ordering.is_eq()),
                "<" => compare_values(left, right, |ordering| ordering.is_lt()),
                "<=" => compare_values(left, right, |ordering| ordering.is_le()),
                ">" => compare_values(left, right, |ordering| ordering.is_gt()),
                ">=" => compare_values(left, right, |ordering| ordering.is_ge()),
                _ => unreachable!(),
            }?;
            left = ExprValue::Number(if result { 1.0 } else { 0.0 });
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<ExprValue, String> {
        let mut left = self.parse_multiplicative()?;

        while let Some(operator) = self.peek() {
            if operator != "+" && operator != "-" {
                break;
            }
            self.advance();
            let right = self.parse_multiplicative()?;
            let left_number = left.number()?;
            let right_number = right.number()?;
            left = ExprValue::Number(if operator == "+" {
                left_number + right_number
            } else {
                left_number - right_number
            });
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<ExprValue, String> {
        let mut left = self.parse_primary()?;

        while let Some(operator) = self.peek() {
            if !matches!(operator, "*" | "/" | "%") {
                break;
            }
            self.advance();
            let right = self.parse_primary()?;
            let left_number = left.number()?;
            let right_number = right.number()?;
            if right_number == 0.0 && (operator == "/" || operator == "%") {
                return Err("division by zero".to_owned());
            }
            left = ExprValue::Number(match operator {
                "*" => left_number * right_number,
                "/" => left_number / right_number,
                "%" => left_number % right_number,
                _ => unreachable!(),
            });
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<ExprValue, String> {
        let token = self
            .advance()
            .ok_or_else(|| "unexpected end of expression".to_owned())?;

        if token == "-" {
            let value = self.parse_primary()?;
            return Ok(ExprValue::Number(-value.number()?));
        }

        if let Ok(value) = token.parse::<f64>() {
            Ok(ExprValue::Number(value))
        } else {
            Ok(ExprValue::Text(token.to_owned()))
        }
    }
}

fn compare_values(
    left: ExprValue,
    right: ExprValue,
    predicate: impl Fn(std::cmp::Ordering) -> bool,
) -> Result<bool, String> {
    if let (Ok(left), Ok(right)) = (left.number(), right.number()) {
        return left
            .partial_cmp(&right)
            .map(predicate)
            .ok_or_else(|| "cannot compare NaN".to_owned());
    }

    let ExprValue::Text(left) = left else {
        return Err("cannot compare a number with text".to_owned());
    };
    let ExprValue::Text(right) = right else {
        return Err("cannot compare text with a number".to_owned());
    };
    Ok(predicate(left.cmp(&right)))
}

fn sleep(args: &[String]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::error("sleep: missing operand");
    }

    let mut total = Duration::ZERO;

    for argument in args {
        let duration = match parse_duration(argument) {
            Ok(duration) => duration,
            Err(message) => return CommandResult::error(format!("sleep: {message}")),
        };
        total = total.saturating_add(duration);
    }

    std::thread::sleep(total);
    CommandResult::silent()
}

fn parse_duration(value: &str) -> Result<Duration, String> {
    let split = value
        .find(|character: char| !character.is_ascii_digit() && character != '.')
        .unwrap_or(value.len());
    let number = &value[..split];
    let unit = &value[split..];

    let number = number
        .parse::<f64>()
        .map_err(|_| format!("invalid duration: {value}"))?;
    if !number.is_finite() || number < 0.0 {
        return Err(format!("invalid duration: {value}"));
    }

    let seconds = match unit {
        "" | "s" => number,
        "ms" => number / 1_000.0,
        "m" => number * 60.0,
        "h" => number * 3_600.0,
        _ => return Err(format!("unknown duration unit: {unit}")),
    };

    Duration::try_from_secs_f64(seconds).map_err(|_| format!("duration is too large: {value}"))
}

fn whoami() -> CommandResult {
    let name = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .or_else(|_| env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_owned());

    CommandResult::ok(name)
}

fn date() -> CommandResult {
    CommandResult::ok(Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string())
}

pub(crate) fn resolve_path(session: &Session, input: &str) -> PathBuf {
    let path = expand_tilde(session, input);

    if path.is_absolute() {
        path
    } else {
        session.working_directory().join(path)
    }
}

fn expand_tilde(session: &Session, input: &str) -> PathBuf {
    if input == "~" {
        if let Some(home) = session_home(session) {
            return home;
        }
    }

    if let Some(rest) = input
        .strip_prefix("~/")
        .or_else(|| input.strip_prefix("~\\"))
    {
        if let Some(home) = session_home(session) {
            return home.join(rest);
        }
    }

    PathBuf::from(input)
}

/// The shell's current `HOME` value, falling back to the process environment.
///
/// Using the session value means `HOME=/somewhere; cd` and `cd ~` follow the
/// variable the user can actually inspect and change.
fn session_home(session: &Session) -> Option<PathBuf> {
    session
        .get_variable("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .or_else(home_directory)
}

fn home_directory() -> Option<PathBuf> {
    if cfg!(windows) {
        if let Some(profile) = env::var_os("USERPROFILE") {
            return Some(PathBuf::from(profile));
        }

        let drive = env::var_os("HOMEDRIVE")?;
        let path = env::var_os("HOMEPATH")?;
        let mut home = PathBuf::from(drive);
        home.push(path);
        Some(home)
    } else {
        env::var_os("HOME").map(PathBuf::from)
    }
}

#[cfg(windows)]
fn simplify_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();

    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }

    if let Some(rest) = text.strip_prefix(r"\\?\") {
        return PathBuf::from(rest);
    }

    path
}

#[cfg(not(windows))]
fn simplify_path(path: PathBuf) -> PathBuf {
    path
}

fn io_message(error: &io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound => "no such file or directory".to_owned(),
        io::ErrorKind::PermissionDenied => "permission denied".to_owned(),
        io::ErrorKind::IsADirectory => "is a directory".to_owned(),
        io::ErrorKind::NotADirectory => "not a directory".to_owned(),
        io::ErrorKind::AlreadyExists => "already exists".to_owned(),
        _ => error.to_string(),
    }
}

fn export_builtin(session: &mut Session, args: &[String]) -> CommandResult {
    if args.is_empty() {
        // List exported variables
        let mut output = String::new();
        let vars = session.local_variables();
        for (name, value, exported) in vars {
            if exported {
                output.push_str(&format!("{name}={value}\n"));
            }
        }
        // Also show exported inherited variables
        let inherited = session.inherited_variables();
        for (name, value) in inherited {
            // Only show if not overridden by a local
            if session.get_local(&name).is_none() {
                output.push_str(&format!("{name}={value}\n"));
            }
        }
        if !output.is_empty() {
            output.pop(); // Remove trailing newline
        }
        CommandResult::ok(output)
    } else {
        // Export variables
        for arg in args {
            if let Some(eq_pos) = arg.find('=') {
                let name = &arg[..eq_pos];
                let value = arg[eq_pos + 1..].to_owned();
                if !shell_core::is_valid_variable_name(name) {
                    return CommandResult::error(format!("export: invalid variable name: {name}"));
                }
                session.set_exported_variable(name, value);
            } else {
                if !shell_core::is_valid_variable_name(arg) {
                    return CommandResult::error(format!("export: invalid variable name: {arg}"));
                }
                if !session.export_variable(arg) {
                    // If not a local variable, check if it's in inherited
                    if session.get_variable(arg).is_some() {
                        // Re-export by copying to locals
                        if let Some(value) = session.get_variable(arg) {
                            session.set_exported_variable(arg, value);
                        }
                    } else {
                        return CommandResult::error(format!("export: variable not found: {arg}"));
                    }
                }
            }
        }
        CommandResult::silent()
    }
}

fn unset_builtin(session: &mut Session, args: &[String]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::error("unset: missing operand");
    }

    for arg in args {
        if !shell_core::is_valid_variable_name(arg) {
            return CommandResult::error(format!("unset: invalid variable name: {arg}"));
        }
        session.unset_variable(arg);
    }

    CommandResult::silent()
}

fn env_builtin(session: &Session, args: &[String]) -> CommandResult {
    if !args.is_empty() {
        return CommandResult::error("env: no options are supported");
    }

    let mut output = String::new();
    let env = session.child_environment(&[]);
    for (key, value) in env {
        output.push_str(&format!(
            "{}={}\n",
            key.to_string_lossy(),
            value.to_string_lossy()
        ));
    }
    if !output.is_empty() {
        output.pop(); // Remove trailing newline
    }
    CommandResult::ok(output)
}

fn set_builtin(session: &Session, args: &[String]) -> CommandResult {
    if !args.is_empty() {
        return CommandResult::error("set: no options are supported");
    }

    let mut output = String::new();

    // Local variables
    output.push_str("Local variables:\n");
    let vars = session.local_variables();
    if vars.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for (name, value, exported) in vars {
            let export_mark = if exported { " (exported)" } else { "" };
            output.push_str(&format!("  {name}={value}{export_mark}\n"));
        }
    }

    // Inherited environment variables
    output.push_str("\nInherited environment:\n");
    let inherited = session.inherited_variables();
    if inherited.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for (name, value) in inherited {
            // Don't show if overridden by local
            if session.get_local(&name).is_none() {
                output.push_str(&format!("  {name}={value}\n"));
            }
        }
    }

    if !output.is_empty() && output.ends_with('\n') {
        output.pop();
    }
    CommandResult::ok(output)
}
