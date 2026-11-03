//! Shell session and execution lifecycle.
//!
//! The session owns platform information, the working directory, configuration,
//! application version, and running/stopped state. Terminal I/O stays in the
//! CLI and line-editor crates.

use shell_config::Config;
use shell_core::{ShellError, VERSION as SHELL_VERSION};
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};

/// Operating-system information detected from the Rust standard library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    os: &'static str,
    arch: &'static str,
    family: &'static str,
}

impl Platform {
    /// Detects the platform this binary was compiled for.
    #[must_use]
    pub const fn detect() -> Self {
        Self {
            os: env::consts::OS,
            arch: env::consts::ARCH,
            family: env::consts::FAMILY,
        }
    }

    /// Operating-system name, such as `windows`, `linux`, or `macos`.
    #[must_use]
    pub const fn os(self) -> &'static str {
        self.os
    }

    /// CPU architecture, such as `x86_64` or `aarch64`.
    #[must_use]
    pub const fn arch(self) -> &'static str {
        self.arch
    }

    /// Platform family, such as `unix` or `windows`.
    #[must_use]
    pub const fn family(self) -> &'static str {
        self.family
    }

    /// Human-readable operating-system name for the startup banner.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self.os {
            "windows" => "Windows",
            "linux" => "Linux",
            "macos" => "macOS",
            other => other,
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({}, {})", self.os, self.arch, self.family)
    }
}

/// Whether a session is accepting commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// The session is accepting input.
    Running,
    /// The session has been asked to shut down.
    Stopped,
}

/// A running shell session.
#[derive(Debug)]
pub struct Session {
    platform: Platform,
    working_directory: PathBuf,
    config: Config,
    version: &'static str,
    state: SessionState,
}

impl Session {
    /// Creates a running session from the current process environment.
    ///
    /// The working directory is captured once so later execution code has a
    /// single source of truth for relative paths.
    pub fn new(config: Config) -> Result<Self, ShellError> {
        let working_directory = env::current_dir()
            .map_err(|source| ShellError::io("determine the current working directory", source))?;

        Ok(Self {
            platform: Platform::detect(),
            working_directory,
            config,
            version: SHELL_VERSION,
            state: SessionState::Running,
        })
    }

    /// Platform detected for this session.
    #[must_use]
    pub const fn platform(&self) -> Platform {
        self.platform
    }

    /// Absolute working directory captured when the session started.
    #[must_use]
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// Configuration associated with this session.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Application version associated with this session.
    #[must_use]
    pub const fn version(&self) -> &'static str {
        self.version
    }

    /// Current session lifecycle state.
    #[must_use]
    pub const fn state(&self) -> SessionState {
        self.state
    }

    /// Returns `true` while the session is accepting commands.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self.state, SessionState::Running)
    }

    /// Returns `true` after shutdown has been requested.
    #[must_use]
    pub const fn is_stopped(&self) -> bool {
        matches!(self.state, SessionState::Stopped)
    }

    /// Requests a clean session shutdown.
    pub fn stop(&mut self) {
        self.state = SessionState::Stopped;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detected_platform_matches_standard_library_constants() {
        let platform = Platform::detect();

        assert_eq!(platform.os(), env::consts::OS);
        assert_eq!(platform.arch(), env::consts::ARCH);
        assert_eq!(platform.family(), env::consts::FAMILY);
        assert!(!platform.to_string().is_empty());
        assert!(!platform.display_name().is_empty());
    }

    #[test]
    fn session_captures_an_absolute_working_directory() {
        let session = Session::new(Config::default()).expect("session should start");

        assert!(session.working_directory().is_absolute());
        assert_eq!(session.platform(), Platform::detect());
    }

    #[test]
    fn session_keeps_the_supplied_configuration() {
        let config = Config {
            prompt: "test> ".to_owned(),
            history_size: 12,
            color: false,
        };

        let session = Session::new(config.clone()).expect("session should start");

        assert_eq!(session.config(), &config);
    }

    #[test]
    fn session_starts_running_with_the_application_version() {
        let session = Session::new(Config::default()).expect("session should start");

        assert_eq!(session.state(), SessionState::Running);
        assert!(session.is_running());
        assert!(!session.is_stopped());
        assert_eq!(session.version(), SHELL_VERSION);
    }

    #[test]
    fn stopping_a_session_updates_its_state() {
        let mut session = Session::new(Config::default()).expect("session should start");

        session.stop();

        assert_eq!(session.state(), SessionState::Stopped);
        assert!(!session.is_running());
        assert!(session.is_stopped());
    }
}
