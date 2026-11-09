//! Serializable contracts shared by the KERNEXT CLI, server, web terminal, and
//! desktop application.
//!
//! This crate intentionally contains no runtime logic. The Axum server owns the
//! `Session` and `Dispatcher` instances and maps their results into these types.
//! Keeping the wire format here means the web and desktop clients never depend
//! on Rust internals.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Version segment used by the HTTP API (`/api/v1`).
pub const API_VERSION: &str = "v1";

/// Product name used in health responses and client labels.
pub const PRODUCT_NAME: &str = "KERNEXT";

/// Unique identifier for a shell session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SessionId(pub usize);

impl std::fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Information about a live shell session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session identifier.
    pub id: SessionId,
    /// Current working directory as an absolute path.
    pub working_directory: String,
    /// Lifecycle state: `running` or `stopped`.
    pub state: String,
    /// Local variables whose values are safe to display.
    pub variables: HashMap<String, String>,
    /// Number of jobs known to the session.
    pub job_count: usize,
    /// Human-readable platform name.
    pub platform: String,
    /// KERNEXT version.
    pub version: String,
    /// Prompt text used by terminal clients.
    pub prompt: String,
}

/// Request to create a new shell session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    /// Optional absolute working directory. Defaults to the server process cwd.
    #[serde(default)]
    pub working_directory: Option<String>,
    /// Optional environment overrides applied when the session is created.
    #[serde(default)]
    pub environment: Option<HashMap<String, String>>,
    /// Reserved for a future serialized configuration payload.
    #[serde(default)]
    pub config: Option<String>,
}

/// Response returned by `POST /api/v1/sessions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    /// Whether the session was created.
    pub success: bool,
    /// Created session when `success` is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionInfo>,
    /// Human-readable error when `success` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request to execute one command line in a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecuteCommandRequest {
    /// Command line parsed by the KERNEXT lexer and parser.
    pub command: String,
    /// Optional working directory override for this command only.
    #[serde(default)]
    pub working_directory: Option<String>,
    /// Optional environment overrides for this command only.
    #[serde(default)]
    pub environment: Option<HashMap<String, String>>,
    /// Reserved for callers that want a pseudoterminal-backed execution.
    #[serde(default)]
    pub allocate_pty: bool,
}

/// Result of executing one command line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteCommandResponse {
    /// Whether the command succeeded.
    pub success: bool,
    /// Combined stdout/stderr text produced by the command.
    pub output: String,
    /// Exit code when one is available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// REPL action requested by the command: `continue`, `clear`, `clear_history`, `exit`.
    pub action: String,
    /// Session state after execution.
    pub session: SessionInfo,
}

/// Response returned by `GET /api/v1/sessions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListResponse {
    /// All live sessions, ordered by identifier.
    pub sessions: Vec<SessionInfo>,
}

/// Health response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Always `ok` when the server can answer.
    pub status: String,
    /// Product name.
    pub product: String,
    /// KERNEXT version.
    pub version: String,
    /// API version segment.
    pub api_version: String,
    /// Number of live sessions.
    pub sessions: usize,
    /// Host platform, for example `windows-x86_64`.
    #[serde(default)]
    pub platform: String,
    /// Whether the server can launch the native `kernext` PTY process.
    #[serde(default)]
    pub pty_available: bool,
}

/// Error body returned by REST endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorBody {
    /// Stable machine-readable code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Streams a terminal client can label output with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    /// Standard output and built-in output.
    Stdout,
    /// Standard error and engine diagnostics.
    Stderr,
    /// Client-facing system notices (connection, clear, exit).
    System,
}

/// Message sent from a terminal client to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Execute one command line through the KERNEXT engine.
    Execute {
        /// The command line.
        command: String,
    },
    /// Send raw terminal input. Used by the PTY bridge.
    Input {
        /// Input bytes encoded as a UTF-8 string.
        data: String,
    },
    /// Resize the terminal.
    Resize {
        /// Rows.
        rows: u16,
        /// Columns.
        cols: u16,
    },
    /// Request an interrupt of the running command.
    Interrupt,
    /// Clear the client scrollback.
    Clear,
    /// Keepalive.
    Ping,
}

/// Message sent from the server to a terminal client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Sent once when the connection is established.
    Ready {
        /// Session snapshot.
        session: SessionInfo,
        /// Terminal prompt.
        prompt: String,
    },
    /// Sent once when a raw pseudoterminal bridge is ready.
    PtyReady {
        /// Terminal prompt reported by the process.
        prompt: String,
    },
    /// Command or PTY output.
    Output {
        /// Which stream produced the data.
        stream: OutputStream,
        /// UTF-8 text. May contain ANSI escape sequences.
        data: String,
    },
    /// Sent after a command finishes.
    Result {
        /// Whether the command succeeded.
        success: bool,
        /// Exit code when available.
        #[serde(skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
        /// REPL action requested by the command.
        action: String,
        /// Session state after execution.
        session: SessionInfo,
    },
    /// Non-fatal error reported to the client.
    Error {
        /// Human-readable message.
        message: String,
    },
    /// Keepalive response.
    Pong,
    /// The session is closing.
    SessionClosed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_messages_use_a_typed_tag() {
        let encoded = serde_json::to_string(&ClientMessage::Execute {
            command: "echo hi".to_owned(),
        })
        .expect("client message should serialize");

        assert!(encoded.contains("\"type\":\"execute\""));
        assert!(encoded.contains("\"command\":\"echo hi\""));
    }

    #[test]
    fn server_messages_round_trip() {
        let message = ServerMessage::Output {
            stream: OutputStream::Stdout,
            data: "hello\n".to_owned(),
        };

        let encoded = serde_json::to_string(&message).expect("message should serialize");
        let decoded: ServerMessage =
            serde_json::from_str(&encoded).expect("message should deserialize");

        match decoded {
            ServerMessage::Output { stream, data } => {
                assert_eq!(stream, OutputStream::Stdout);
                assert_eq!(data, "hello\n");
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
}
