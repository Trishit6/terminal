//! Implementations of the native Phase 5 commands.
//!
//! Every command resolves paths against the session working directory and uses
//! only the Rust standard library (plus `chrono` for local date formatting), so
//! the same implementation runs on Linux, macOS, and Windows.

use crate::outcome::CommandAction;
use crate::registry::BuiltinRegistry;
use chrono::Local;
use shell_runtime::Session;
use std::env;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Result of one native command.
///
/// Standard output and standard error are kept separate so redirection
/// (`>`, `>>`, `2>`, `2>>`) and pipelines can route each stream independently.
#[derive(Debug)]
pub(crate) struct CommandResult {
    stdout: String,
    stderr: String,
    success: bool,
    action: CommandAction,
}

impl CommandResult {
    fn ok(output: impl Into<String>) -> Self {
        Self {
            stdout: output.into(),
            stderr: String::new(),
            success: true,
            action: CommandAction::Continue,
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
        }
    }

    fn from_parts(output: String, errors: &[String]) -> Self {
        Self {
            stdout: output,
            stderr: errors.join("\n"),
            success: errors.is_empty(),
            action: CommandAction::Continue,
        }
    }

    fn exit() -> Self {
        Self {
            stdout: "Goodbye.".to_owned(),
            stderr: String::new(),
            success: true,
            action: CommandAction::Exit,
        }
    }

    fn clear() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            success: true,
            action: CommandAction::ClearScreen,
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
}

/// State shared by command implementations during one evaluation.
pub(crate) struct CommandContext<'a> {
    pub(crate) session: &'a mut Session,
    pub(crate) registry: &'a BuiltinRegistry,
    pub(crate) history: &'a [String],
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
        "mkdir" => mkdir(context.session, args),
        "touch" => touch(context.session, args),
        "cat" => cat(context.session, context.stdin, args),
        "echo" => echo(args),
        "cp" => cp(context.session, args),
        "mv" => mv(context.session, args),
        "rm" => rm(context.session, args),
        "clear" => CommandResult::clear(),
        "history" => history(context.history, args),
        "whoami" => whoami(),
        "date" => date(),
        "version" => CommandResult::ok(format!("Custom Shell v{}", context.session.version())),
        "exit" => CommandResult::exit(),
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
        None => match home_directory() {
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

fn echo(args: &[String]) -> CommandResult {
    CommandResult::ok(format!("{}\n", args.join(" ")))
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

fn history(entries: &[String], args: &[String]) -> CommandResult {
    if !args.is_empty() {
        return CommandResult::error("history: no options are supported");
    }

    let width = entries.len().to_string().len();
    let output = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| format!("{:>width$}  {entry}", index + 1))
        .collect::<Vec<_>>()
        .join("\n");

    CommandResult::ok(output)
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
    let path = expand_tilde(input);

    if path.is_absolute() {
        path
    } else {
        session.working_directory().join(path)
    }
}

fn expand_tilde(input: &str) -> PathBuf {
    if input == "~" {
        if let Some(home) = home_directory() {
            return home;
        }
    }

    if let Some(rest) = input
        .strip_prefix("~/")
        .or_else(|| input.strip_prefix("~\\"))
    {
        if let Some(home) = home_directory() {
            return home.join(rest);
        }
    }

    PathBuf::from(input)
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
