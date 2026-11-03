//! Startup banner rendering and output.

use shell_core::ShellError;
use shell_runtime::Session;
use std::io::{self, Write};

const BANNER_WIDTH: usize = 40;

/// Builds the complete startup banner without performing I/O.
#[must_use]
pub fn render(session: &Session) -> String {
    let rule = "=".repeat(BANNER_WIDTH);

    format!(
        "{rule}\n\
         {:^width$}\n\
         {:^width$}\n\
         {rule}\n\
         \n\
         Custom Shell v{version}\n\
         Platform: {platform}\n\
         Working directory: {directory}\n\
         Type 'help' for available commands.\n\
         Type 'exit' to close the shell.\n\
         \n",
        "CUSTOM SHELL",
        "Independent Rust CLI",
        rule = rule,
        width = BANNER_WIDTH,
        version = session.version(),
        platform = session.platform().display_name(),
        directory = session.working_directory().display(),
    )
}

/// Writes the startup banner to standard output and flushes it.
pub fn write_banner(banner: &str) -> Result<(), ShellError> {
    let stdout = io::stdout();
    let mut output = stdout.lock();

    output
        .write_all(banner.as_bytes())
        .and_then(|()| output.flush())
        .map_err(|source| ShellError::io("write startup output", source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_config::Config;

    #[test]
    fn banner_contains_name_version_platform_and_directory() {
        let session = Session::new(Config::default()).expect("session should start");

        let banner = render(&session);

        assert!(banner.contains("CUSTOM SHELL"));
        assert!(banner.contains("Independent Rust CLI"));
        assert!(banner.contains(&format!("Custom Shell v{}", shell_core::VERSION)));
        assert!(banner.contains(session.platform().display_name()));
        assert!(banner.contains(&session.working_directory().display().to_string()));
        assert!(banner.contains("Type 'help' for available commands."));
        assert!(banner.contains("Type 'exit' to close the shell."));
    }
}
