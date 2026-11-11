//! Session registry and in-process KERNEXT engine instances.

use crate::db::{AuthConfig, Database, Mailer, RateLimiter};
use crate::error::ApiError;
use chrono::{DateTime, Utc};
use serde::Serialize;
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
use std::time::{Duration, Instant, SystemTime};

/// Maximum number of bytes accepted for one command line.
pub const MAX_COMMAND_BYTES: usize = 64 * 1024;

/// One in-process KERNEXT session.
pub struct ShellSession {
    id: SessionId,
    session: Session,
    dispatcher: Dispatcher,
    created_at: Instant,
    last_active: Instant,
    created_wall: SystemTime,
    last_active_wall: SystemTime,
    owner: Option<String>,
    shutdown: Arc<tokio::sync::Notify>,
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
        Self::new_owned(id, request, None)
    }

    /// Creates a session owned by an authenticated account.
    pub fn new_owned(
        id: SessionId,
        request: &CreateSessionRequest,
        owner: Option<String>,
    ) -> Result<Self, ApiError> {
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
        let now_wall = SystemTime::now();

        Ok(Self {
            id,
            session,
            dispatcher,
            created_at: now,
            last_active: now,
            created_wall: now_wall,
            last_active_wall: now_wall,
            owner,
            shutdown: Arc::new(tokio::sync::Notify::new()),
        })
    }

    /// Account that owns this session, when the account system is enabled.
    #[must_use]
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    /// Whether `user_id` may access this session.
    ///
    /// Sessions created without an owner (local-only server, tests, the
    /// desktop runtime) are accessible to local callers.
    #[must_use]
    pub fn is_owned_by(&self, user_id: &str) -> bool {
        match self.owner.as_deref() {
            Some(owner) => owner == user_id,
            None => false,
        }
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

    /// Administrator-facing snapshot of this engine session.
    #[must_use]
    pub fn admin_view(&self) -> AdminTerminalSession {
        AdminTerminalSession {
            id: format!("engine-{}", self.id.0),
            owner: self.owner.clone(),
            mode: "engine".to_owned(),
            created_at: self.created_wall.into(),
            last_active_at: self.last_active_wall.into(),
            status: if self.session.is_running() {
                "active".to_owned()
            } else {
                "stopped".to_owned()
            },
            platform: self.session.platform().display_name().to_owned(),
        }
    }

    /// Executes one command line through the KERNEXT lexer, parser, and AST.
    pub fn execute(&mut self, command: &str) -> ExecuteCommandResponse {
        self.last_active = Instant::now();
        self.last_active_wall = SystemTime::now();
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
        self.shutdown.notify_one();
    }

    /// Signal used by an administrator to close the engine socket.
    #[must_use]
    pub fn shutdown_signal(&self) -> Arc<tokio::sync::Notify> {
        Arc::clone(&self.shutdown)
    }

    /// Applies request-scoped working-directory and environment overrides.
    ///
    /// Only the entries the request actually supplies are recorded. In
    /// particular, a request without a `working_directory` override must not
    /// restore anything, or a `cd` executed by the command would be silently
    /// reverted — the CLI and the engine WebSocket keep it, so the HTTP path
    /// has to as well.
    pub fn begin_overrides(
        &mut self,
        request: &ExecuteCommandRequest,
    ) -> Result<SessionOverrides, ApiError> {
        let mut directory_overridden = false;
        let old_directory = self.session.working_directory().to_path_buf();

        if let Some(directory) = &request.working_directory {
            self.session
                .set_working_directory(resolve_directory(directory)?);
            directory_overridden = true;
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
            directory_overridden,
            old_directory,
            environment,
        })
    }

    /// Restores the state saved by [`ShellSession::begin_overrides`].
    pub fn end_overrides(&mut self, overrides: SessionOverrides) {
        if overrides.directory_overridden {
            self.session.set_working_directory(overrides.old_directory);
        }

        for (name, previous, was_exported) in overrides.environment {
            match previous {
                Some(value) => self.session.set_variable(&name, value, was_exported),
                None => self.session.remove_local(&name),
            }
        }
    }
}

/// Releases a pseudoterminal slot when the socket task ends.
#[derive(Debug)]
pub struct PtyPermit {
    counts: Arc<Mutex<HashMap<String, usize>>>,
    owner: String,
}

impl Drop for PtyPermit {
    fn drop(&mut self) {
        if let Ok(mut counts) = self.counts.lock()
            && let Some(count) = counts.get_mut(&self.owner)
        {
            *count = count.saturating_sub(1);
            if *count == 0 {
                counts.remove(&self.owner);
            }
        }
    }
}

/// State captured before request-scoped overrides are applied.
#[derive(Debug)]
pub struct SessionOverrides {
    directory_overridden: bool,
    old_directory: PathBuf,
    environment: Vec<(String, Option<String>, bool)>,
}

/// One live terminal session as shown in the administration console.
///
/// The dashboard deliberately excludes command input, terminal output, and
/// working-directory file contents. It carries only identity, mode, timing,
/// platform, and lifecycle state.
#[derive(Debug, Clone, Serialize)]
pub struct AdminTerminalSession {
    pub id: String,
    pub owner: Option<String>,
    pub mode: String,
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub status: String,
    pub platform: String,
}

struct PtySessionEntry {
    id: u64,
    owner: String,
    created_at: SystemTime,
    last_active_at: Mutex<SystemTime>,
    platform: String,
    shutdown: Arc<tokio::sync::Notify>,
}

impl PtySessionEntry {
    fn view(&self) -> AdminTerminalSession {
        AdminTerminalSession {
            id: format!("pty-{}", self.id),
            owner: Some(self.owner.clone()),
            mode: "pty".to_owned(),
            created_at: self.created_at.into(),
            last_active_at: self
                .last_active_at
                .lock()
                .map(|value| (*value).into())
                .unwrap_or_else(|_| self.created_at.into()),
            status: "active".to_owned(),
            platform: self.platform.clone(),
        }
    }
}

/// Handle for one registered pseudoterminal connection.
///
/// Dropping it unregisters the session. The WebSocket task selects on
/// [`PtyHandle::shutdown`] so an administrator can close a remote terminal
/// without affecting any other connection.
pub struct PtyHandle {
    entry: Arc<PtySessionEntry>,
    registry: Arc<Mutex<HashMap<u64, Arc<PtySessionEntry>>>>,
}

impl std::fmt::Debug for PtyHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PtyHandle")
            .field("id", &self.entry.id)
            .finish_non_exhaustive()
    }
}

impl PtyHandle {
    /// Public session identifier (`pty-N`).
    #[must_use]
    pub fn id(&self) -> String {
        format!("pty-{}", self.entry.id)
    }

    /// Records client activity for the dashboard.
    pub fn touch(&self) {
        if let Ok(mut last_active) = self.entry.last_active_at.lock() {
            *last_active = SystemTime::now();
        }
    }

    /// Signal used to ask the owning socket task to close the session.
    #[must_use]
    pub fn shutdown(&self) -> Arc<tokio::sync::Notify> {
        Arc::clone(&self.entry.shutdown)
    }
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        if let Ok(mut sessions) = self.registry.lock() {
            sessions.remove(&self.entry.id);
        }
    }
}

/// Increments the live WebSocket counter while held.
pub struct WsConnectionGuard {
    count: Arc<AtomicUsize>,
}

impl Drop for WsConnectionGuard {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::Relaxed);
    }
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
    [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "PRIVATE_KEY",
        "API_KEY",
    ]
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

/// Account system attached to a server instance.
///
/// When this is present, terminal endpoints require a signed-in user and every
/// session is bound to its owner. When it is absent the server keeps its
/// original local-only bearer-token behavior, documented as such.
#[derive(Clone)]
pub struct AuthContext {
    pub db: Arc<Database>,
    pub config: Arc<AuthConfig>,
    pub mailer: Arc<Mailer>,
    pub limiter: Arc<RateLimiter>,
}

impl AuthContext {
    /// Builds the account context from a connected database.
    #[must_use]
    pub fn new(db: Database, config: AuthConfig) -> Self {
        let mailer = Mailer::from_env(&config.public_url);
        Self {
            db: Arc::new(db),
            config: Arc::new(config),
            mailer: Arc::new(mailer),
            limiter: Arc::new(RateLimiter::default()),
        }
    }
}

/// Shared server state.
#[derive(Clone)]
pub struct AppState {
    sessions: Arc<Mutex<HashMap<SessionId, Arc<Mutex<ShellSession>>>>>,
    next_id: Arc<AtomicUsize>,
    token: Option<Arc<str>>,
    max_sessions: usize,
    accounts: Option<Arc<AuthContext>>,
    pty_counts: Arc<Mutex<HashMap<String, usize>>>,
    pty_sessions: Arc<Mutex<HashMap<u64, Arc<PtySessionEntry>>>>,
    next_pty_id: Arc<AtomicUsize>,
    ws_connections: Arc<AtomicUsize>,
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
            accounts: None,
            pty_counts: Arc::new(Mutex::new(HashMap::new())),
            pty_sessions: Arc::new(Mutex::new(HashMap::new())),
            next_pty_id: Arc::new(AtomicUsize::new(1)),
            ws_connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Attaches the MariaDB-backed account system.
    #[must_use]
    pub fn with_accounts(mut self, accounts: AuthContext) -> Self {
        self.accounts = Some(Arc::new(accounts));
        self
    }

    /// Account system, when `DATABASE_URL` was configured.
    #[must_use]
    pub fn accounts(&self) -> Option<Arc<AuthContext>> {
        self.accounts.clone()
    }

    /// Whether terminal access requires an authenticated account.
    #[must_use]
    pub fn requires_accounts(&self) -> bool {
        self.accounts.is_some()
    }

    /// Configured bearer token, when authentication is enabled.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Builds a local-only state with no authentication token.
    ///
    /// The desktop application embeds the session registry directly and never
    /// exposes it over HTTP, so it does not read `KERNEXT_SERVER_TOKEN`.
    #[must_use]
    pub fn local() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicUsize::new(1)),
            token: None,
            max_sessions: 16,
            accounts: None,
            pty_counts: Arc::new(Mutex::new(HashMap::new())),
            pty_sessions: Arc::new(Mutex::new(HashMap::new())),
            next_pty_id: Arc::new(AtomicUsize::new(1)),
            ws_connections: Arc::new(AtomicUsize::new(0)),
        }
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
        self.create_owned(request, None)
    }

    /// Creates a session bound to an account.
    pub fn create_owned(
        &self,
        request: &CreateSessionRequest,
        owner: Option<&str>,
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

        let user_sessions = match owner {
            Some(owner) => sessions
                .values()
                .filter(|session| {
                    session
                        .lock()
                        .map(|guard| guard.is_owned_by(owner))
                        .unwrap_or(false)
                })
                .count(),
            None => 0,
        };

        if owner.is_some() && user_sessions >= self.max_sessions_per_user() {
            return Err(ApiError::conflict(
                "this account already has the maximum number of terminal sessions",
            ));
        }

        let id = SessionId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let shell = ShellSession::new_owned(id, request, owner.map(ToOwned::to_owned))?;
        let session = Arc::new(Mutex::new(shell));
        sessions.insert(id, Arc::clone(&session));

        Ok(session)
    }

    fn max_sessions_per_user(&self) -> usize {
        4
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
        self.list_filtered(None)
    }

    /// Lists sessions owned by one account.
    #[must_use]
    pub fn list_owned(&self, user_id: &str) -> Vec<SessionInfo> {
        self.list_filtered(Some(user_id))
    }

    fn list_filtered(&self, owner: Option<&str>) -> Vec<SessionInfo> {
        let sessions = match self.sessions.lock() {
            Ok(guard) => guard.values().cloned().collect::<Vec<_>>(),
            Err(_) => return Vec::new(),
        };

        let mut listed = sessions
            .into_iter()
            .filter_map(|session| {
                let guard = session.lock().ok()?;
                let visible = match owner {
                    Some(owner) => guard.is_owned_by(owner),
                    None => true,
                };
                visible.then(|| guard.info())
            })
            .collect::<Vec<_>>();

        listed.sort_by_key(|session| session.id);
        listed
    }

    /// Reserves a pseudoterminal slot for an account (or `local` when the
    /// server runs without accounts). Release it by dropping the returned
    /// permit, which happens when the WebSocket task ends.
    pub fn acquire_pty(&self, owner: &str) -> Result<PtyPermit, ApiError> {
        let limit = if self.accounts.is_some() {
            2
        } else {
            self.max_sessions
        };
        let mut counts = self
            .pty_counts
            .lock()
            .map_err(|_| ApiError::internal("pseudoterminal registry lock was poisoned"))?;
        let count = counts.entry(owner.to_owned()).or_insert(0);

        if *count >= limit {
            return Err(ApiError::conflict(
                "this account already has the maximum number of pseudoterminals",
            ));
        }

        *count += 1;

        Ok(PtyPermit {
            counts: Arc::clone(&self.pty_counts),
            owner: owner.to_owned(),
        })
    }

    /// Registers one live pseudoterminal connection for the dashboard.
    pub fn register_pty(&self, owner: &str) -> Result<PtyHandle, ApiError> {
        let mut sessions = self
            .pty_sessions
            .lock()
            .map_err(|_| ApiError::internal("pseudoterminal registry lock was poisoned"))?;
        let id = self.next_pty_id.fetch_add(1, Ordering::Relaxed) as u64;
        let now = SystemTime::now();
        let entry = Arc::new(PtySessionEntry {
            id,
            owner: owner.to_owned(),
            created_at: now,
            last_active_at: Mutex::new(now),
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            shutdown: Arc::new(tokio::sync::Notify::new()),
        });

        sessions.insert(id, Arc::clone(&entry));

        Ok(PtyHandle {
            entry,
            registry: Arc::clone(&self.pty_sessions),
        })
    }

    /// Number of live terminal WebSocket connections.
    #[must_use]
    pub fn ws_connections(&self) -> usize {
        self.ws_connections.load(Ordering::Relaxed)
    }

    /// Counts one WebSocket connection until the returned guard is dropped.
    #[must_use]
    pub fn ws_connection_guard(&self) -> WsConnectionGuard {
        self.ws_connections.fetch_add(1, Ordering::Relaxed);
        WsConnectionGuard {
            count: Arc::clone(&self.ws_connections),
        }
    }

    /// Every live engine and pseudoterminal session, newest first.
    #[must_use]
    pub fn admin_terminal_sessions(&self) -> Vec<AdminTerminalSession> {
        let mut listed = Vec::new();

        if let Ok(sessions) = self.sessions.lock() {
            for session in sessions.values() {
                if let Ok(guard) = session.lock() {
                    listed.push(guard.admin_view());
                }
            }
        }

        if let Ok(sessions) = self.pty_sessions.lock() {
            for session in sessions.values() {
                listed.push(session.view());
            }
        }

        listed.sort_by_key(|session| std::cmp::Reverse(session.created_at));
        listed
    }

    /// Terminates one live terminal session by its dashboard identifier.
    ///
    /// Engine sessions are stopped and removed from the registry; the owning
    /// engine socket observes the stop signal. Pseudoterminals receive a
    /// shutdown notification, and their socket task kills the child process
    /// exactly as it does on a normal disconnect. Other sessions are
    /// untouched.
    pub fn terminate_terminal_session(&self, id: &str) -> Result<bool, ApiError> {
        if let Some(raw) = id.strip_prefix("engine-") {
            let Ok(number) = raw.parse::<usize>() else {
                return Ok(false);
            };
            let Some(session) = self.remove(SessionId(number)) else {
                return Ok(false);
            };
            if let Ok(mut guard) = session.lock() {
                guard.stop();
            }
            return Ok(true);
        }

        if let Some(raw) = id.strip_prefix("pty-") {
            let Ok(number) = raw.parse::<u64>() else {
                return Ok(false);
            };
            let entry = self
                .pty_sessions
                .lock()
                .ok()
                .and_then(|mut sessions| sessions.remove(&number));
            let Some(entry) = entry else {
                return Ok(false);
            };
            entry.shutdown.notify_one();
            return Ok(true);
        }

        Ok(false)
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
            working_directory: Some(std::env::temp_dir().to_string_lossy().into_owned()),
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
    fn cd_persists_when_no_directory_override_is_supplied() {
        let mut session = new_session();
        let original = session.info().working_directory;
        let request = ExecuteCommandRequest {
            command: "cd ..".to_owned(),
            ..ExecuteCommandRequest::default()
        };

        let overrides = session
            .begin_overrides(&request)
            .expect("plain requests should not fail");
        let response = session.execute(&request.command);
        session.end_overrides(overrides);

        assert!(response.success, "{response:?}");
        let moved = session.info().working_directory;
        assert_ne!(
            moved, original,
            "cd executed by the command must persist, like the CLI"
        );
        assert_eq!(
            response.session.working_directory, moved,
            "the response snapshot must report the directory after the command"
        );

        // And a request-scoped override still wins for one command only.
        let request = ExecuteCommandRequest {
            command: "pwd".to_owned(),
            working_directory: Some(std::env::temp_dir().to_string_lossy().into_owned()),
            ..ExecuteCommandRequest::default()
        };
        let overrides = session
            .begin_overrides(&request)
            .expect("override should apply");
        let response = session.execute(&request.command);
        session.end_overrides(overrides);

        assert!(response.success, "{response:?}");
        assert!(
            response.output.contains("Temp") || response.output.contains("tmp"),
            "{}",
            response.output
        );
        assert_eq!(session.info().working_directory, moved);
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
