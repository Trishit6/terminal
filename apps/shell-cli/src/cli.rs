use crate::repl::Repl;
use crate::startup;
use clap::Parser;
use shell_config::Config;
use shell_core::{SHELL_NAME, ShellError, VERSION};
use shell_line_editor::{CompletionContext, ShellEditor};
use shell_runtime::Session;
use std::io::{self, IsTerminal};
use std::sync::Arc;

#[derive(Debug, Parser)]
#[command(
    name = SHELL_NAME,
    bin_name = SHELL_NAME,
    version = VERSION,
    about = "A cross-platform command-line shell built in Rust.",
    long_about = "A cross-platform command-line shell built in Rust.\n\n\
                  Phase 9 provides an interactive REPL with persistent history, \
                  tab completion, and command suggestions."
)]
pub struct Cli {}

/// Wrapper to provide completion context from Session.
struct SessionCompletionContext {
    session: Session,
    builtins: Vec<String>,
}

impl CompletionContext for SessionCompletionContext {
    fn builtins(&self) -> &[String] {
        &self.builtins
    }

    fn executables(&self) -> Vec<String> {
        // Use the same PATH resolution as the shell
        let path_dirs = std::env::var_os("PATH")
            .map_or_else(Vec::new, |path| std::env::split_paths(&path).collect());

        let mut executables = Vec::new();
        #[cfg(windows)]
        let pathext = std::env::var_os("PATHEXT").map_or_else(
            || vec![".exe".to_string(), ".com".to_string(), ".bat".to_string()],
            |ext| {
                std::env::split_paths(&ext)
                    .map(|p| p.to_string_lossy().to_string())
                    .collect()
            },
        );

        for dir in path_dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    #[cfg(windows)]
                    {
                        let is_exe = pathext.iter().any(|ext| name.ends_with(ext));
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
        self.session
            .local_variables()
            .into_iter()
            .map(|(k, _, _)| k)
            .collect()
    }

    fn exported_variables(&self) -> Vec<String> {
        self.session
            .local_variables()
            .into_iter()
            .filter(|(_, _, exported)| *exported)
            .map(|(k, _, _)| k)
            .collect()
    }

    fn working_directory(&self) -> std::path::PathBuf {
        self.session.working_directory().to_path_buf()
    }
}

pub fn run() -> Result<(), ShellError> {
    let _arguments = Cli::parse();

    if !io::stdin().is_terminal() {
        return Err(ShellError::message(
            "start interactive shell",
            "standard input is not a terminal",
        ));
    }

    let config = Config::default();
    let session = Session::new(config.clone())?;

    // Load persistent history if enabled
    let history_file = if config.history_enabled {
        config.history_path()
    } else {
        None
    };

    let builtins: Vec<String> = shell_builtins::DEFAULT_BUILTINS
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    let completion_context = Arc::new(SessionCompletionContext {
        session: session.clone(),
        builtins: builtins.clone(),
    });

    let editor = ShellEditor::with_session(
        config.prompt,
        config.history_size,
        config.color,
        history_file,
        builtins,
        completion_context,
    )
    .map_err(|error| ShellError::message("initialize line editor", error.to_string()))?;

    startup::write_banner(&startup::render(&session))?;

    let mut repl = Repl::new(session, editor, io::stdout());
    repl.run()
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
        let help = Cli::try_parse_from(["custom-shell", "--help"])
            .expect_err("--help should short-circuit parsing");
        let version = Cli::try_parse_from(["custom-shell", "--version"])
            .expect_err("--version should short-circuit parsing");

        assert_eq!(help.kind(), clap::error::ErrorKind::DisplayHelp);
        assert_eq!(version.kind(), clap::error::ErrorKind::DisplayVersion);
    }
}
