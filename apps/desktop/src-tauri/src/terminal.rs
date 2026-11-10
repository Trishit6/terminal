//! Terminal session management for the desktop application.
//!
//! Engine sessions reuse the same `Session`/`Dispatcher` registry as the
//! local Axum server. PTY sessions spawn this executable in worker mode on a
//! genuine pseudoterminal, so the desktop terminal gets the same interactive
//! behavior as the CLI: line editing, resize, Ctrl+C, and full-screen programs.

use crate::state::{DesktopSessionSummary, DesktopState, PtySession};
use serde::Serialize;
use shell_api::{CreateSessionRequest, ExecuteCommandResponse, SessionId, SessionInfo};
use shell_builtins::{ExternalCommand, ProcessError, PtyProcess, PtySize};
use shell_config::Config;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tauri::{AppHandle, Emitter};

/// Event carrying PTY output to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct PtyOutputEvent {
    /// PTY session identifier.
    pub session_id: u32,
    /// UTF-8 text read from the worker.
    pub data: String,
}

/// Event sent when a PTY session is ready.
#[derive(Debug, Clone, Serialize)]
pub struct PtyReadyEvent {
    /// PTY session identifier.
    pub session_id: u32,
    /// Prompt used by the worker.
    pub prompt: String,
}

/// Event sent when a PTY session exits.
#[derive(Debug, Clone, Serialize)]
pub struct PtyClosedEvent {
    /// PTY session identifier.
    pub session_id: u32,
}

/// Creates a new in-process engine session.
///
/// The request's working directory is filled with the configured default when
/// the caller does not provide one.
///
/// # Errors
///
/// Returns a human-readable error string when the session cannot be started.
pub fn create_engine_session(
    state: &DesktopState,
    request: CreateSessionRequest,
) -> Result<SessionInfo, String> {
    let mut request = request;

    if request.working_directory.is_none() {
        request.working_directory = Some(
            state
                .default_working_directory()
                .to_string_lossy()
                .into_owned(),
        );
    }

    let session = map_api_error(state.sessions.create(&request))?;
    let info = session
        .lock()
        .map_err(|_| "engine session lock was poisoned".to_owned())?
        .info();

    Ok(info)
}

/// Executes one command line in an engine session.
///
/// # Errors
///
/// Returns a human-readable error string when the session is missing, the
/// engine fails, or the blocking task cannot be scheduled.
pub async fn execute_engine_session(
    state: &DesktopState,
    id: usize,
    command: String,
) -> Result<ExecuteCommandResponse, String> {
    let session = state
        .sessions
        .get(SessionId(id))
        .ok_or_else(|| format!("engine session {id} does not exist"))?;

    let command_for_task = command.clone();
    let response = tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session
            .lock()
            .map_err(|_| "engine session lock was poisoned".to_owned())?;
        Ok::<_, String>(guard.execute(&command_for_task))
    })
    .await
    .map_err(|error| format!("engine execution task failed: {error}"))??;

    state.record_recent_command(&command, response.exit_code);
    Ok(response)
}

/// Removes an engine session.
///
/// Returns `true` when the session existed and was removed.
pub fn delete_engine_session(state: &DesktopState, id: usize) -> bool {
    state.sessions.remove(SessionId(id)).is_some()
}

/// Lists engine sessions.
#[must_use]
pub fn list_engine_sessions(state: &DesktopState) -> Vec<SessionInfo> {
    state.sessions.list()
}

/// Lists PTY sessions.
#[must_use]
pub fn list_pty_sessions(state: &DesktopState) -> Vec<DesktopSessionSummary> {
    state.pty.list()
}

/// Starts a KERNEXT worker on a pseudoterminal.
///
/// The worker is this same executable started with
/// `KERNEXT_DESKTOP_WORKER=1`, so a desktop build never needs a separate shell
/// sidecar. Output is streamed to the frontend with the
/// `desktop-pty-output` event.
///
/// # Errors
///
/// Returns a human-readable error string when the executable cannot be
/// located, the pseudoterminal cannot be created, or the worker cannot start.
pub async fn create_pty_session(
    app: AppHandle,
    state: &DesktopState,
    rows: u16,
    cols: u16,
) -> Result<DesktopSessionSummary, String> {
    let working_directory = state.default_working_directory();
    let executable = worker_executable()?;
    let prompt = Config::default().prompt;
    let registry = state.pty.clone();
    let spawn_working_directory = working_directory.clone();

    let spawn_result = tauri::async_runtime::spawn_blocking(move || {
        let mut command = ExternalCommand::new(executable.to_string_lossy().into_owned());
        command = command
            .current_dir(spawn_working_directory.clone())
            .env("KERNEXT_DESKTOP_WORKER", "1");

        PtyProcess::spawn(&command, PtySize::new(rows, cols))
    })
    .await
    .map_err(|error| format!("pseudoterminal task failed: {error}"))?;

    let mut process = spawn_result.map_err(pty_spawn_error)?;

    let reader = process
        .take_reader()
        .ok_or_else(|| "pseudoterminal reader was already taken".to_owned())?;
    let writer = process
        .take_writer()
        .ok_or_else(|| "pseudoterminal writer was already taken".to_owned())?;
    let resizer = process.resizer();

    let id = state.pty.next_id();
    let session = Arc::new(PtySession::new(
        id,
        working_directory.clone(),
        prompt.clone(),
        writer,
        resizer,
    ));

    state.pty.insert(Arc::clone(&session));

    let reader_session_id = id;
    let reader_app = app.clone();
    thread::spawn(move || {
        let mut reader = reader;
        let mut buffer = [0u8; 8192];

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let data = String::from_utf8_lossy(&buffer[..count]).into_owned();
                    let _ = reader_app.emit(
                        "desktop-pty-output",
                        PtyOutputEvent {
                            session_id: reader_session_id,
                            data,
                        },
                    );
                }
                Err(error) => {
                    let _ = reader_app.emit(
                        "desktop-pty-output",
                        PtyOutputEvent {
                            session_id: reader_session_id,
                            data: format!("\r\nkernext: terminal read error: {error}\r\n"),
                        },
                    );
                    break;
                }
            }
        }
    });

    let monitor_registry = registry.clone();
    let monitor_app = app.clone();
    thread::spawn(move || {
        let mut process = process;
        let status = process.wait();

        if let Some(session) = monitor_registry.remove(id) {
            session.close();
        }

        let _ = monitor_app.emit("desktop-pty-closed", PtyClosedEvent { session_id: id });

        if let Err(error) = status {
            let _ = monitor_app.emit(
                "desktop-pty-output",
                PtyOutputEvent {
                    session_id: id,
                    data: format!("\r\nkernext: worker exited with an error: {error}\r\n"),
                },
            );
        }
    });

    let _ = app.emit(
        "desktop-pty-ready",
        PtyReadyEvent {
            session_id: id,
            prompt: prompt.clone(),
        },
    );

    Ok(DesktopSessionSummary::from_pty(&session))
}

/// Writes input to a PTY session.
///
/// # Errors
///
/// Returns a human-readable error string when the session is missing or the
/// write fails.
pub fn pty_write(state: &DesktopState, id: u32, data: &str) -> Result<(), String> {
    let session = state
        .pty
        .get(id)
        .ok_or_else(|| format!("pseudoterminal session {id} does not exist"))?;

    session.write_input(data.as_bytes())
}

/// Resizes a PTY session.
///
/// # Errors
///
/// Returns a human-readable error string when the resize is refused.
pub fn pty_resize(state: &DesktopState, id: u32, rows: u16, cols: u16) -> Result<(), String> {
    let session = state
        .pty
        .get(id)
        .ok_or_else(|| format!("pseudoterminal session {id} does not exist"))?;

    session.resize(rows, cols)
}

/// Sends Ctrl+C to a PTY session.
///
/// # Errors
///
/// Returns a human-readable error string when the interrupt cannot be written.
pub fn pty_interrupt(state: &DesktopState, id: u32) -> Result<(), String> {
    let session = state
        .pty
        .get(id)
        .ok_or_else(|| format!("pseudoterminal session {id} does not exist"))?;

    session.interrupt()
}

/// Closes a PTY session.
///
/// Returns `true` when the session existed and was closed.
pub fn pty_close(state: &DesktopState, id: u32) -> bool {
    state
        .pty
        .remove(id)
        .map(|session| session.close())
        .is_some()
}

/// Resolves the desktop executable used as the worker process.
///
/// # Errors
///
/// Returns a human-readable error string when the current executable cannot
/// be located.
fn worker_executable() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|error| format!("could not locate the desktop executable: {error}"))
}

/// Converts the server API error type into a plain string for Tauri commands.
fn map_api_error<T>(result: Result<T, shell_server::ApiError>) -> Result<T, String> {
    result.map_err(|error| error.to_string())
}

/// Formats a pseudoterminal spawn error.
fn pty_spawn_error(error: ProcessError) -> String {
    format!("could not start the KERNEXT pseudoterminal worker: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::DesktopSettings;

    fn test_state() -> DesktopState {
        DesktopState::new(
            DesktopSettings::default(),
            std::env::temp_dir().join("kernext-desktop-terminal-test"),
        )
    }

    #[test]
    fn engine_sessions_execute_commands() {
        let state = test_state();
        let info = create_engine_session(&state, CreateSessionRequest::default())
            .expect("engine session should start");

        assert_eq!(info.state, "running");
        assert!(!info.working_directory.is_empty());
    }

    #[test]
    fn deleting_a_missing_engine_session_is_false() {
        let state = test_state();
        assert!(!delete_engine_session(&state, 999));
    }

    #[test]
    fn pty_close_of_missing_session_is_false() {
        let state = test_state();
        assert!(!pty_close(&state, 999));
    }
}
