use serde::{Deserialize, Serialize};

/// Unique identifier for a shell session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionId(pub usize);

/// Information about a shell session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session identifier
    pub id: SessionId,
    /// Current working directory
    pub working_directory: String,
    /// Current session state
    pub state: String,
    /// Simplified variable map (name -> value)
    pub variables: std::collections::HashMap<String, String>,
    /// Job count
    pub job_count: usize,
}

/// Request to create a new shell session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    /// Optional configuration
    pub config: Option<String>,
}

/// Response from creating a new shell session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    /// New session identifier
    pub session: SessionInfo,
    /// Whether the session was created successfully
    pub success: bool,
    /// Error message when creation fails
    pub error: Option<String>,
}

/// Request to execute a command in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteCommandRequest {
    /// The command to execute
    pub command: String,
    /// Working directory override
    pub working_directory: Option<String>,
    /// Environment variable overrides
    pub environment: Option<std::collections::HashMap<String, String>>,
    /// Whether to allocate a PTY for interactive sessions
    pub allocate_pty: bool,
}

/// Unique identifier for a command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandExecutionId(pub usize);

// Structured terminal events with session and execution identifiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalEvent {
    /// A new session was created
    SessionCreated { session: SessionInfo },
    /// The session is ready to accept commands
    SessionReady { session: SessionInfo },
    /// A command execution has started
    CommandStarted {
        execution_id: CommandExecutionId,
        session: SessionInfo,
    },
    /// Standard output from a command
    Stdout {
        execution_id: CommandExecutionId,
        session: SessionInfo,
        data: String,
    },
    /// Standard error from a command
    Stderr {
        execution_id: CommandExecutionId,
        session: SessionInfo,
        data: String,
    },
    /// A command execution has completed
    CommandCompleted {
        execution_id: CommandExecutionId,
        session: SessionInfo,
        success: bool,
        exit_code: i32,
    },
    /// A command execution has failed
    CommandFailed {
        execution_id: CommandExecutionId,
        session: SessionInfo,
        error: String,
    },
    /// A process has been started
    ProcessStarted {
        execution_id: CommandExecutionId,
        session: SessionInfo,
    },
    /// A process has exited
    ProcessExited {
        execution_id: CommandExecutionId,
        session: SessionInfo,
        success: bool,
        exit_code: i32,
    },
    /// Terminal resize event
    TerminalResized {
        execution_id: Option<CommandExecutionId>,
        session: SessionInfo,
        rows: u16,
        cols: u16,
    },
    /// Session has been interrupted (Ctrl+C)
    SessionInterrupted { session: SessionInfo },
    /// Session is being closed
    SessionClosed { session: SessionInfo },
}

/// Input to be sent to an interactive terminal session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalInput {
    /// The input line to send
    pub line: String,
    /// Whether this is the final input for this command
    pub final_input: bool,
}

/// Terminal resize dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalResize {
    /// Number of rows
    pub rows: u16,
    /// Number of columns
    pub cols: u16,
}

/// Execution status of a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatus {
    /// Command execution identifier
    pub execution_id: CommandExecutionId,
    /// Whether the command succeeded
    pub success: bool,
    /// Exit code of the process
    pub exit_code: i32,
    /// Human-readable error when the command fails
    pub error: Option<String>,
}

/// Error type for API requests.
#[derive(Debug)]
pub enum ApiError {
    /// Session not found
    SessionNotFound,
    /// Invalid request
    InvalidRequest(String),
    /// Command execution failed
    CommandFailed(String),
    /// Internal error
    Internal(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::SessionNotFound => write!(f, "Session not found"),
            ApiError::InvalidRequest(s) => write!(f, "Invalid request: {s}"),
            ApiError::CommandFailed(s) => write!(f, "Command failed: {s}"),
            ApiError::Internal(s) => write!(f, "Internal error: {s}"),
        }
    }
}

/// Convert a Session to SessionInfo.
pub fn session_to_info() -> SessionInfo {
    SessionInfo {
        id: SessionId(1),
        working_directory: "/tmp".to_string(),
        state: "Running".to_string(),
        variables: std::collections::HashMap::new(),
        job_count: 0,
    }
}
