use crate::repl::Repl;
use crate::startup;
use clap::Parser;
use shell_config::Config;
use shell_core::{SHELL_NAME, ShellError, VERSION};
use shell_line_editor::ShellEditor;
use shell_runtime::Session;
use std::io::{self, IsTerminal};

#[derive(Debug, Parser)]
#[command(
    name = SHELL_NAME,
    bin_name = SHELL_NAME,
    version = VERSION,
    about = "A cross-platform command-line shell built in Rust.",
    long_about = "A cross-platform command-line shell built in Rust.\n\n\
                  Phase 2 provides an interactive REPL with command history, \
                  built-in help, version, clear, and exit commands."
)]
pub struct Cli {}

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
    let editor = ShellEditor::new(config.prompt, config.history_size, config.color)
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
