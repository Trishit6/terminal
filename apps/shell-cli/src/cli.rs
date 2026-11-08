use crate::repl::Repl;
use crate::startup::{self, Startup};
use clap::Parser;
use shell_builtins::Dispatcher;
use shell_config::Config;
use shell_core::{PRODUCT_NAME, SHELL_NAME, ShellError, VERSION};
use shell_line_editor::{CompletionContext, EditorSettings, ShellEditor};
use shell_runtime::Session;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::io::{self, IsTerminal};
use std::sync::{Arc, RwLock};

#[derive(Debug, Parser)]
#[command(
    name = SHELL_NAME,
    bin_name = SHELL_NAME,
    version = VERSION,
    about = "KERNEXT — The Next Core of Computing.",
    long_about = "KERNEXT — The Next Core of Computing.\n\n\
                  An independent, native Rust shell with its own lexer, parser, \
                  evaluator, built-in commands, pipelines, redirections, and \
                  external process executor."
)]
pub struct Cli {
    /// Skip the startup banner and animation entirely.
    #[arg(long)]
    pub quiet: bool,

    /// Show the startup banner without the reveal animation.
    #[arg(long = "no-animation")]
    pub no_animation: bool,
}

/// Wrapper to provide completion context from Session.
struct SessionCompletionContext {
    session: RwLock<Session>,
    builtins: Vec<String>,
}

impl CompletionContext for SessionCompletionContext {
    fn builtins(&self) -> &[String] {
        &self.builtins
    }

    fn executables(&self) -> Vec<String> {
        // Use the shell's current PATH, not the process PATH captured at
        // startup, so `PATH=...` is reflected immediately.
        let session = self.session.read().ok();
        let path = session
            .as_ref()
            .and_then(|session| session.get_variable("PATH"));
        let path_dirs = path.as_deref().map_or_else(Vec::new, |path| {
            std::env::split_paths(OsStr::new(path)).collect()
        });

        let mut executables = Vec::new();
        // `PATHEXT` is semicolon-separated; `split_paths` would use the
        // platform path separator and split it incorrectly. `.bat` and `.cmd`
        // are excluded because KERNEXT never delegates to another shell.
        #[cfg(windows)]
        let pathext: Vec<String> = session
            .as_ref()
            .and_then(|session| session.get_variable("PATHEXT"))
            .or_else(|| std::env::var("PATHEXT").ok())
            .map_or_else(
                || vec![".exe".to_string(), ".com".to_string()],
                |ext| {
                    ext.split(';')
                        .map(str::trim)
                        .filter(|extension| !extension.is_empty())
                        .map(str::to_ascii_lowercase)
                        .collect()
                },
            );

        for dir in path_dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    #[cfg(windows)]
                    {
                        let lower = name.to_ascii_lowercase();
                        let is_exe = pathext.iter().any(|ext| lower.ends_with(ext))
                            && !lower.ends_with(".bat")
                            && !lower.ends_with(".cmd");
                        if is_exe && entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                            executables.push(name);
                        }
                    }
                    #[cfg(not(windows))]
                    {
                        use std::os::unix::fs::MetadataExt;
                        if let Ok(meta) = entry.metadata() {
                            if meta.is_file() && meta.mode() & 0o111 != 0 {
                                executables.push(name);
                            }
                        }
                    }
                }
            }
        }

        executables.sort();
        executables.dedup();
        executables
    }

    fn variables(&self) -> Vec<String> {
        let Ok(session) = self.session.read() else {
            return Vec::new();
        };

        let mut variables: Vec<String> = session
            .local_variables()
            .into_iter()
            .map(|(name, _, _)| name)
            .collect();
        variables.extend(
            session
                .inherited_variables()
                .into_iter()
                .map(|(name, _)| name),
        );
        variables.sort();
        variables.dedup();
        variables
    }

    fn exported_variables(&self) -> Vec<String> {
        let Ok(session) = self.session.read() else {
            return Vec::new();
        };

        let locals = session.local_variables();
        let shadowed: HashSet<String> = locals.iter().map(|(name, _, _)| name.clone()).collect();
        let mut exported: Vec<String> = locals
            .into_iter()
            .filter(|(_, _, exported)| *exported)
            .map(|(name, _, _)| name)
            .collect();
        exported.extend(
            session
                .inherited_variables()
                .into_iter()
                .filter(|(name, _)| !shadowed.contains(name))
                .map(|(name, _)| name),
        );
        exported.sort();
        exported.dedup();
        exported
    }

    fn working_directory(&self) -> std::path::PathBuf {
        self.session.read().map_or_else(
            |_| std::path::PathBuf::from("."),
            |session| session.working_directory().to_path_buf(),
        )
    }

    fn refresh(&self, session: &Session) {
        if let Ok(mut snapshot) = self.session.write() {
            *snapshot = session.clone();
        }
    }
}

pub fn run() -> Result<i32, ShellError> {
    let arguments = Cli::parse();

    if !io::stdin().is_terminal() {
        return Err(ShellError::message(
            "start interactive shell",
            "standard input is not a terminal; KERNEXT is an interactive shell",
        ));
    }

    let config = load_config();
    let interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
    let mode = startup::resolve_mode(
        &config,
        arguments.quiet,
        arguments.no_animation,
        interactive,
    );
    let color = config.color && interactive && startup::supports_color();
    let width = startup::terminal_width();
    let theme = config.theme();

    let mut startup = Startup::new(io::stdout(), mode, color, width)
        .with_theme(theme)
        .with_animation_speed(&config.animation_speed);
    if config.apply_terminal_font && interactive {
        startup
            .apply_terminal_font(&config.terminal_font)
            .map_err(startup_error)?;
    }
    startup.intro().map_err(startup_error)?;

    let stage = "Initializing KERNEXT Core";
    startup.stage_begin(stage).map_err(startup_error)?;
    let session = match Session::new(config.clone()) {
        Ok(session) => session,
        Err(error) => {
            let _ = startup.stage_failed(stage, &error.to_string());
            let _ = startup.abort();
            return Err(error);
        }
    };
    startup.stage_ok(stage).map_err(startup_error)?;

    let stage = "Preparing Runtime Environment";
    startup.stage_begin(stage).map_err(startup_error)?;
    let history_file = if config.history_enabled {
        config.history_path()
    } else {
        None
    };
    startup.stage_ok(stage).map_err(startup_error)?;
    startup.session_summary(&session).map_err(startup_error)?;

    let stage = "Loading Command Engine";
    startup.stage_begin(stage).map_err(startup_error)?;
    let dispatcher = Dispatcher::new();
    startup.stage_ok(stage).map_err(startup_error)?;

    let builtins: Vec<String> = shell_builtins::DEFAULT_BUILTINS
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    let completion_context = Arc::new(SessionCompletionContext {
        session: RwLock::new(session.clone()),
        builtins: builtins.clone(),
    });

    let stage = "Configuring Shell Interface";
    startup.stage_begin(stage).map_err(startup_error)?;
    let editor_settings = EditorSettings {
        history_filter_secrets: config.history_filter_secrets,
        suggestions_enabled: config.suggestions_enabled,
        completion_hidden_files: config.completion_hidden_files,
    };
    let prompt_color = color.then(|| theme.prompt.foreground());
    let editor = match ShellEditor::with_session_settings(
        config.prompt,
        config.history_size,
        color,
        history_file,
        builtins,
        completion_context,
        editor_settings,
    )
    .map(|editor| editor.with_prompt_color(prompt_color))
    {
        Ok(editor) => editor,
        Err(error) => {
            let shell_error = ShellError::message("initialize line editor", error.to_string());
            let _ = startup.stage_failed(stage, &shell_error.to_string());
            let _ = startup.abort();
            return Err(shell_error);
        }
    };
    startup.stage_ok(stage).map_err(startup_error)?;

    let stage = "Starting Session";
    startup.stage_begin(stage).map_err(startup_error)?;
    let mut repl = Repl::with_dispatcher(session, editor, io::stdout(), dispatcher);
    startup.stage_ok(stage).map_err(startup_error)?;
    startup.welcome().map_err(startup_error)?;

    match repl.run() {
        Ok(()) => Ok(repl.exit_status().unwrap_or(0)),
        Err(error) => {
            let _ = startup.abort();
            Err(error)
        }
    }
}

/// Loads the user configuration, falling back to defaults on a read or parse
/// error so a damaged configuration file cannot make the shell unusable.
fn load_config() -> Config {
    let Some(path) = Config::default_path() else {
        return Config::default();
    };

    match Config::load_or_default(&path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{PRODUCT_NAME}: warning: {error}; using built-in defaults");
            Config::default()
        }
    }
}

fn startup_error(source: io::Error) -> ShellError {
    ShellError::io("write startup output", source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn command_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn help_and_version_are_available() {
        let help = Cli::try_parse_from(["kernext", "--help"])
            .expect_err("--help should short-circuit parsing");
        let version = Cli::try_parse_from(["kernext", "--version"])
            .expect_err("--version should short-circuit parsing");

        assert_eq!(help.kind(), clap::error::ErrorKind::DisplayHelp);
        assert_eq!(version.kind(), clap::error::ErrorKind::DisplayVersion);
    }

    #[test]
    fn startup_flags_parse_without_affecting_other_arguments() {
        let quiet = Cli::try_parse_from(["kernext", "--quiet"]).expect("--quiet should parse");
        let plain = Cli::try_parse_from(["kernext", "--no-animation"])
            .expect("--no-animation should parse");

        assert!(quiet.quiet);
        assert!(!quiet.no_animation);
        assert!(!plain.quiet);
        assert!(plain.no_animation);
    }
}
