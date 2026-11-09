//! Session registry and in-process KERNEXT engine instances.

use crate::error::ApiError;
use shell_api::{
    CreateSessionRequest, ExecuteCommandRequest, ExecuteCommandResponse, SessionId, SessionInfo,
};
use shell_builtins::{CommandAction, Dispatcher, ProcessOptions, StdioMode};
use shell_config::Config;
use shell_runtime::Session;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Maximum number of bytes accepted for one command line.
pub const MAX_COMMAND_BYTES: usize = 64 * 1024;

/// One in-process KERNEXT session.
pub struct ShellSession {
    id: SessionId,
    session: Session,
    dispatcher: Dispatcher,
    created_at: Instant,
    last_active: Instant,
}

impl std::fmt::Debug for ShellSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ShellSession")
            .field("id", &self.id)
            .field("working_directory", &self.session.working_directory())
            .finish_non_exhaustive()
    }
}

impl ShellSession {
    /// Creates a session with the server's capture-oriented process I/O.
    pub fn new(id: SessionId, request: &CreateSessionRequest) -> Result<Self, ApiError> {
        let config = load_config();
        let mut session = Session::new(config)
            .map_err(|error| ApiError::internal(format!("could not start shell: {error}")))?;

        if let Some(environment) = &request.environment {
            for (name, value) in environment {
                validate_variable_name(name)?;
                session.set_exported_variable(name, value.clone());
            }
        }

        if let Some(directory) = &request.working_directory {
            session.set_working_directory(resolve_directory(directory)?);
        }

        let dispatcher =
            Dispatcher::with_options(ProcessOptions::new().with_stdio(StdioMode::Capture));
        let now = Instant::now();

        Ok(Self {
            id,
            session,
            dispatcher,
            created_at: now,
            last_active: now,
        })
    }

    /// Session identifier.
    #[must_use]
    pub const fn id(&self) -> SessionId {
        self.id
    }

    /// Time the session was created.
    #[must_use]
    pub const fn created_at(&self) -> Instant {
        self.created_at
    }

    /// Time the session last executed a command.
    #[must_use]
    pub const fn last_active(&self) -> Instant {
        self.last_active
    }

    /// Snapshot suitable for JSON responses.
    #[must_use]
    pub fn info(&self) -> SessionInfo {
        let mut variables = HashMap::new();

        for (name, value, _exported) in self.session.local_variables() {
            if !is_sensitive_name(&name) {
                variables.insert(name, value);
            }
        }

        let job_count = self
            .session
            .jobs()
            .lock()
            .map(|mut table| table.snapshot().len())
            .unwrap_or(0);

        SessionInfo {
            id: self.id,
            working_directory: self
                .session
                .working_directory()
                .to_string_lossy()
                .into_owned(),
            state: if self.session.is_running() {
                "running".to_owned()
            } else {
                "stopped".to_owned()
            },
            variables,
            job_count,
            platform: self.session.platform().display_name().to_owned(),
            version: self.session.version().to_owned(),
            prompt: self.session.config().prompt.clone(),
        }
    }

    /// Executes one command line through the KERNEXT lexer, parser, and AST.
    pub fn execute(&mut self, command: &str) -> ExecuteCommandResponse {
        self.last_active = Instant::now();
        let outcome = self.dispatcher.execute(&mut self.session, command);
        let action = action_name(outcome.action()).to_owned();

        if outcome.action() == CommandAction::Exit {
            self.session.stop();
        }

        ExecuteCommandResponse {
            success: outcome.success(),
            output: outcome.output().unwrap_or_default().to_owned(),
            exit_code: outcome.exit_code(),
            action,
            session: self.info(),
        }
    }

    /// Stops the session so clients see a terminal lifecycle state.
    pub fn stop(&mut self) {
        self.session.stop();
    }

    /// Applies request-scoped working-directory and environment overrides.
    pub fn begin_overrides(
        &mut self,
        request: &ExecuteCommandRequest,
    ) -> Result<SessionOverrides, ApiError> {
        let old_directory = self.session.working_directory().to_path_buf();

        if let Some(directory) = &request.working_directory {
            self.session
                .set_working_directory(resolve_directory(directory)?);
        }

        let exported: HashMap<String, bool> = self
            .session
            .local_variables()
            .into_iter()
            .map(|(name, _value, exported)| (name, exported))
            .collect();

        let mut environment = Vec::new();

        if let Some(overrides) = &request.environment {
            for (name, value) in overrides {
                validate_variable_name(name)?;
                let previous = self.session.get_local(name);
                let was_exported = exported.get(name).copied().unwrap_or(false);
                environment.push((name.clone(), previous, was_exported));
                self.session.set_exported_variable(name, value.clone());
            }
        }

        Ok(SessionOverrides {
            old_directory,
            environment,
        })
    }

    /// Restores the state saved by [`ShellSession::begin_overrides`].
    pub fn end_overrides(&mut self, overrides: SessionOverrides) {
        self.session
            .set_working_directory(overrides.old_directory);

        for (name, previous, was_exported) in overrides.environment {
            match previous {
                Some(value) => self.session.set_variable(&name, value, was_exported),
                None => self.session.remove_local(&name),
            }
        }
    }
}

/// State captured before request-scoped overrides are applied.
#[derive(Debug)]
pub struct SessionOverrides {
    old_directory: PathBuf,
    environment: Vec<(String, Option<String>, bool)>,
}

fn load_config() -> Config {
    Config::default_path()
        .map(|path| Config::load_or_default(&path).unwrap_or_default())
        .unwrap_or_default()
}

fn resolve_directory(value: &str) -> Result<PathBuf, ApiError> {
    let candidate = PathBuf::from(value);
    let canonical = std::fs::canonicalize(&candidate).map_err(|error| {
        ApiError::bad_request(format!(
            "working directory {value:?} could not be resolved: {error}"
        ))
    })?;

    if !canonical.is_dir() {
        return Err(ApiError::bad_request(format!(
            "working directory {value:?} is not a directory"
        )));
    }

    Ok(canonical)
}

fn validate_variable_name(name: &str) -> Result<(), ApiError> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');

    if valid {
        Ok(())
    } else {
        Err(ApiError::bad_request(format!(
            "environment variable name {name:?} is invalid"
        )))
    }
}

fn is_sensitive_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    ["TOKEN", "SECRET", "PASSWORD", "PASSWD", "PRIVATE_KEY", "API_KEY"]
        .iter()
        .any(|needle| upper.contains(needle))
}

/// Maps a REPL action onto the API vocabulary.
#[must_use]
pub const fn action_name(action: CommandAction) -> &'static str {
    match action {
        CommandAction::Continue => "continue",
        CommandAction::ClearScreen => "clear",
        CommandAction::ClearHistory => "clear_history",
        CommandAction::Exit => "exit",
    }
}

/// Shared server state.
#[derive(Clone)]
pub struct AppState {
    sessions: Arc<Mutex<HashMap<SessionId, Arc<Mutex<ShellSession>>>>>,
    next_id: Arc<AtomicUsize>,
    token: Option<Arc<str>>,
    max_sessions: usize,
}

impl AppState {
    /// Builds state from environment variables.
    ///
    /// - `KERNEXT_SERVER_TOKEN` enables bearer-token authentication.
    /// - `KERNEXT_MAX_SESSIONS` caps concurrent sessions (default 16).
    #[must_use]
    pub fn from_env() -> Self {
        let token = std::env::var("KERNEXT_SERVER_TOKEN")
            .ok()
            .filter(|value| !value.is_empty())
            .map(Arc::<str>::from);

        let max_sessions = std::env::var("KERNEXT_MAX_SESSIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(16);

        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicUsize::new(1)),
            token,
            max_sessions,
        }
    }

    /// Configured bearer token, when authentication is enabled.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Number of live sessions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sessions.lock().map(|guard| guard.len()).unwrap_or(0)
    }

    /// Returns `true` when no sessions exist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Creates and registers a new session.
    pub fn create(
        &self,
        request: &CreateSessionRequest,
    ) -> Result<Arc<Mutex<ShellSession>>, ApiError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| ApiError::internal("session registry lock was poisoned"))?;

        if sessions.len() >= self.max_sessions {
            return Err(ApiError::conflict(format!(
                "the server already has {} sessions",
                self.max_sessions
            )));
        }

        let id = SessionId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let shell = ShellSession::new(id, request)?;
        let session = Arc::new(Mutex::new(shell));
        sessions.insert(id, Arc::clone(&session));

        Ok(session)
    }

    /// Looks up a session by identifier.
    #[must_use]
    pub fn get(&self, id: SessionId) -> Option<Arc<Mutex<ShellSession>>> {
        self.sessions
            .lock()
            .ok()
            .and_then(|guard| guard.get(&id).cloned())
    }

    /// Removes a session.
    pub fn remove(&self, id: SessionId) -> Option<Arc<Mutex<ShellSession>>> {
        self.sessions
            .lock()
            .ok()
            .and_then(|mut guard| guard.remove(&id))
    }

    /// Lists sessions ordered by identifier.
    #[must_use]
    pub fn list(&self) -> Vec<SessionInfo> {
        let sessions = match self.sessions.lock() {
            Ok(guard) => guard.values().cloned().collect::<Vec<_>>(),
            Err(_) => return Vec::new(),
        };

        let mut listed = sessions
            .into_iter()
            .filter_map(|session| session.lock().ok().map(|guard| guard.info()))
            .collect::<Vec<_>>();

        listed.sort_by_key(|session| session.id);
        listed
    }

    /// Removes sessions that have been idle for longer than `max_idle`.
    pub fn cleanup_idle(&self, max_idle: Duration) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };

        let now = Instant::now();
        sessions.retain(|_id, session| {
            session
                .lock()
                .map(|guard| now.duration_since(guard.last_active()) < max_idle)
                .unwrap_or(false)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_session() -> ShellSession {
        ShellSession::new(SessionId(1), &CreateSessionRequest::default())
            .expect("session should start")
    }

    #[test]
    fn echo_runs_through_the_engine() {
        let mut session = new_session();
        let response = session.execute("echo hello");

        assert!(response.success, "{response:?}");
        assert!(response.output.contains("hello"), "{response:?}");
        assert_eq!(response.action, "continue");
    }

    #[test]
    fn exit_stops_the_session() {
        let mut session = new_session();
        let response = session.execute("exit 7");

        assert_eq!(response.action, "exit");
        assert_eq!(response.exit_code, Some(7));
        assert_eq!(response.session.state, "stopped");
    }

    #[test]
    fn request_overrides_are_restored() {
        let mut session = new_session();
        let original = session.info().working_directory;
        let request = ExecuteCommandRequest {
            command: "pwd".to_owned(),
            working_directory: Some(
                std::env::temp_dir()
                    .to_string_lossy()
                    .into_owned(),
            ),
            environment: Some(HashMap::from([(
                "KERNEXT_TEST_OVERRIDE".to_owned(),
                "present".to_owned(),
            )])),
            allocate_pty: false,
        };

        let overrides = session
            .begin_overrides(&request)
            .expect("overrides should apply");
        let response = session.execute(&request.command);
        session.end_overrides(overrides);

        assert!(response.success, "{response:?}");
        assert_eq!(session.info().working_directory, original);
        assert!(
            !session
                .info()
                .variables
                .contains_key("KERNEXT_TEST_OVERRIDE")
        );
    }

    #[test]
    fn unknown_working_directories_are_rejected() {
        let mut session = new_session();
        let request = ExecuteCommandRequest {
            command: "pwd".to_owned(),
            working_directory: Some("this-directory-does-not-exist-kernext".to_owned()),
            ..ExecuteCommandRequest::default()
        };

        assert!(session.begin_overrides(&request).is_err());
    }
}

