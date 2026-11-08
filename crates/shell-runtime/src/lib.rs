//! Shell session and execution lifecycle.
//!
//! The session owns platform information, the working directory, configuration,
//! application version, and running/stopped state. Terminal I/O stays in the
//! CLI and line-editor crates.

pub mod history;
pub mod jobs;
pub mod variables;

pub use jobs::{JobInfo, JobStatus, JobTable};
pub use shell_config::Config;
use shell_core::{ShellError, VERSION as SHELL_VERSION};
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::variables::VariableStore;
use shell_core::FunctionDef;

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
#[derive(Debug, Clone)]
pub struct Session {
    platform: Platform,
    working_directory: PathBuf,
    config: Config,
    version: &'static str,
    state: SessionState,
    variables: VariableStore,
    functions: std::collections::HashMap<String, FunctionDef>,
    return_value: Option<String>,
    history_limit: Option<usize>,
    jobs: Arc<Mutex<JobTable>>,
    script_depth: usize,
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
            variables: VariableStore::new(),
            functions: std::collections::HashMap::new(),
            return_value: None,
            history_limit: None,
            jobs: Arc::new(Mutex::new(JobTable::new())),
            script_depth: 0,
        })
    }

    /// Creates a session with a custom variable store for testing.
    pub fn with_environment(
        config: Config,
        env: Vec<(String, String)>,
    ) -> Result<Self, ShellError> {
        let working_directory = env::current_dir()
            .map_err(|source| ShellError::io("determine the current working directory", source))?;

        Ok(Self {
            platform: Platform::detect(),
            working_directory,
            config,
            version: SHELL_VERSION,
            state: SessionState::Running,
            variables: VariableStore::with_environment(env),
            functions: std::collections::HashMap::new(),
            return_value: None,
            history_limit: None,
            jobs: Arc::new(Mutex::new(JobTable::new())),
            script_depth: 0,
        })
    }

    /// Platform detected for this session.
    #[must_use]
    pub const fn platform(&self) -> Platform {
        self.platform
    }

    /// Current working directory of the shell session.
    #[must_use]
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// Replaces the session working directory and synchronizes PWD.
    ///
    /// Callers are responsible for validating that the path exists and is a
    /// directory. The session owns this state instead of the process-wide
    /// current directory so independent sessions remain isolated.
    pub fn set_working_directory(&mut self, directory: PathBuf) {
        self.variables.set_pwd(directory.as_path());
        self.working_directory = directory;
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

    /// Returns a reference to the variable store.
    #[must_use]
    pub fn variables(&self) -> &VariableStore {
        &self.variables
    }

    /// Returns a mutable reference to the variable store.
    pub fn variables_mut(&mut self) -> &mut VariableStore {
        &mut self.variables
    }

    /// Gets a variable value (checks locals, then inherited, then specials).
    #[must_use]
    pub fn get_variable(&self, name: &str) -> Option<String> {
        self.variables.get(name)
    }

    /// Sets a local variable.
    pub fn set_variable(&mut self, name: &str, value: String, exported: bool) {
        self.variables.set_local(name, value, exported);
    }

    /// Exports a variable.
    pub fn export_variable(&mut self, name: &str) -> bool {
        self.variables.export(name)
    }

    /// Sets and exports a variable in one operation.
    pub fn set_exported_variable(&mut self, name: &str, value: String) {
        self.variables.set_exported(name, value);
    }

    /// Unsets a variable.
    pub fn unset_variable(&mut self, name: &str) -> bool {
        self.variables.unset(name)
    }

    /// Returns the environment for child processes.
    pub fn child_environment(
        &self,
        overrides: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
        self.variables.child_environment(overrides)
    }

    /// Returns all local variables with their export status.
    pub fn local_variables(&self) -> Vec<(String, String, bool)> {
        self.variables.local_variables()
    }

    /// Returns all inherited environment variables.
    pub fn inherited_variables(&self) -> Vec<(String, String)> {
        self.variables.inherited_variables()
    }

    /// Gets a local variable value (not inherited or special).
    #[must_use]
    pub fn get_local(&self, name: &str) -> Option<String> {
        self.variables.get_local(name)
    }

    /// Removes a local variable without hiding an inherited variable of the
    /// same name. Used to restore function-parameter scope.
    pub fn remove_local(&mut self, name: &str) {
        self.variables.remove_local(name);
    }

    /// Session-only history limit set by `history limit`.
    ///
    /// `None` means the configured `history_size` applies.
    #[must_use]
    pub const fn history_limit(&self) -> Option<usize> {
        self.history_limit
    }

    /// Overrides the configured history limit for this session.
    pub fn set_history_limit(&mut self, limit: usize) {
        self.history_limit = Some(limit.max(1));
    }

    /// Gets the last command exit status ($?).
    #[must_use]
    pub fn last_status(&self) -> i32 {
        self.variables.last_status()
    }

    /// Sets the last command exit status ($?).
    pub fn set_last_status(&mut self, status: i32) {
        self.variables.set_last_status(status);
    }

    /// Gets the shell process ID ($$).
    #[must_use]
    pub fn process_id(&self) -> u32 {
        self.variables.process_id()
    }

    /// Sets a function definition in the session.
    pub fn set_function(&mut self, name: String, func_def: FunctionDef) {
        self.functions.insert(name, func_def);
    }

    /// Gets a function definition from the session.
    pub fn get_function(&self, name: &str) -> Option<&FunctionDef> {
        self.functions.get(name)
    }

    /// Sets a return value for the current function.
    pub fn set_return_value(&mut self, value: Option<String>) {
        self.return_value = value;
    }

    /// Gets the return value from the current function.
    pub fn get_return_value(&self) -> Option<&String> {
        self.return_value.as_ref()
    }

    /// Takes the return value, clearing it.
    pub fn take_return_value(&mut self) -> Option<String> {
        self.return_value.take()
    }

    /// Returns the shared background-job table.
    ///
    /// Cloned sessions share the same table, so a pipeline clone and the
    /// interactive shell see a consistent job list.
    #[must_use]
    pub fn jobs(&self) -> Arc<Mutex<JobTable>> {
        Arc::clone(&self.jobs)
    }

    /// Registers a detached shell-expression job.
    pub fn add_shell_job(&self, command: impl Into<String>) -> u32 {
        self.jobs
            .lock()
            .expect("job table mutex should not be poisoned")
            .add_shell(command)
    }

    /// Registers a background external process.
    pub fn add_external_job(&self, command: impl Into<String>, child: std::process::Child) -> u32 {
        self.jobs
            .lock()
            .expect("job table mutex should not be poisoned")
            .add_external(command, child)
    }

    /// Reports completion of a detached shell-expression job.
    pub fn complete_job(&self, id: u32, success: bool, exit_code: Option<i32>) {
        self.jobs
            .lock()
            .expect("job table mutex should not be poisoned")
            .complete(id, success, exit_code);
    }

    /// Current script nesting depth.
    #[must_use]
    pub const fn script_depth(&self) -> usize {
        self.script_depth
    }

    /// Enters a script invocation, rejecting nesting beyond `limit`.
    pub fn enter_script(&mut self, limit: usize) -> bool {
        if self.script_depth >= limit {
            return false;
        }
        self.script_depth += 1;
        true
    }

    /// Leaves the current script invocation.
    pub fn leave_script(&mut self) {
        self.script_depth = self.script_depth.saturating_sub(1);
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
            history_enabled: true,
            history_file: "history.txt".to_owned(),
            history_filter_secrets: true,
            suggestions_enabled: true,
            completion_hidden_files: false,
            startup_animation: true,
            quiet_startup: false,
            ..Config::default()
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

    #[test]
    fn working_directory_can_be_replaced() {
        let mut session = Session::new(Config::default()).expect("session should start");
        let target = env::temp_dir();

        session.set_working_directory(target.clone());

        assert_eq!(session.working_directory(), target);
    }
}
