//! WebSocket terminal endpoints.
//!
//! Two transports share the same message vocabulary:
//!
//! - `/api/v1/sessions/{id}/ws` executes each command through the in-process
//!   KERNEXT engine and streams the resulting text.
//! - `/api/v1/pty` launches the real `kernext` executable on a pseudoterminal
//!   (ConPTY on Windows, `openpty` on Unix) and relays raw terminal bytes. This
//!   is what gives the browser full-screen interactive programs, resize, and
//!   Ctrl+C.

use crate::auth::authorize_ws;
use crate::error::ApiError;
use crate::state::{AppState, ShellSession};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use shell_api::{ClientMessage, OutputStream, ServerMessage, SessionId};
use shell_builtins::{ExternalCommand, PtyProcess, PtySize};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug, Default, Deserialize)]
struct WsAuth {
    token: Option<String>,
}

/// Upgrades a request to a line-oriented KERNEXT session socket.
pub async fn session_socket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(id): Path<usize>,
    headers: HeaderMap,
    Query(auth): Query<WsAuth>,
) -> Result<Response, ApiError> {
    authorize_ws(&state, &headers, auth.token.as_deref())?;

    let session = state
        .get(SessionId(id))
        .ok_or_else(|| ApiError::not_found(format!("session {id} does not exist")))?;

    Ok(ws
        .max_message_size(1024 * 1024)
        .on_upgrade(move |socket| handle_session_socket(socket, session)))
}

/// Upgrades a request to a raw pseudoterminal socket.
pub async fn pty_socket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(auth): Query<WsAuth>,
) -> Result<Response, ApiError> {
    authorize_ws(&state, &headers, auth.token.as_deref())?;

    let binary = pty_binary().ok_or_else(|| {
        ApiError::unavailable(
            "the kernext executable was not found; build it with `cargo build -p shell-cli` \
             or set KERNEXT_BIN",
        )
    })?;

    Ok(ws
        .max_message_size(1024 * 1024)
        .on_upgrade(move |socket| handle_pty_socket(socket, binary)))
}

async fn handle_session_socket(socket: WebSocket, session: Arc<Mutex<ShellSession>>) {
    let (mut sink, mut stream) = socket.split();

    let ready = match session.lock() {
        Ok(guard) => Some(guard.info()),
        Err(_) => None,
    };

    let Some(ready) = ready else {
        let _ = send_error(&mut sink, "the session lock was poisoned").await;
        return;
    };

    if send_message(
        &mut sink,
        &ServerMessage::Ready {
            session: ready.clone(),
            prompt: ready.prompt.clone(),
        },
    )
    .await
    .is_err()
    {
        return;
    }

    while let Some(message) = stream.next().await {
        let Ok(message) = message else {
            break;
        };

        let text = match message {
            Message::Text(text) => text.to_string(),
            Message::Close(_) => break,
            Message::Ping(payload) => {
                if sink.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
                continue;
            }
            _ => continue,
        };

        let client = match serde_json::from_str::<ClientMessage>(&text) {
            Ok(client) => client,
            Err(error) => {
                if send_error(&mut sink, format!("invalid message: {error}"))
                    .await
                    .is_err()
                {
                    break;
                }
                continue;
            }
        };

        match client {
            ClientMessage::Execute { command } => {
                if command.trim().is_empty() {
                    continue;
                }

                let session = Arc::clone(&session);
                let result = tokio::task::spawn_blocking(move || {
                    let mut guard = session
                        .lock()
                        .map_err(|_| "the session lock was poisoned".to_owned())?;
                    Ok::<_, String>(guard.execute(&command))
                })
                .await;

                match result {
                    Ok(Ok(response)) => {
                        if !response.output.is_empty()
                            && send_message(
                                &mut sink,
                                &ServerMessage::Output {
                                    stream: OutputStream::Stdout,
                                    data: response.output.clone(),
                                },
                            )
                            .await
                            .is_err()
                        {
                            break;
                        }

                        let action = response.action.clone();
                        if send_message(
                            &mut sink,
                            &ServerMessage::Result {
                                success: response.success,
                                exit_code: response.exit_code,
                                action: action.clone(),
                                session: response.session,
                            },
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }

                        if action == "exit" {
                            let _ = send_message(&mut sink, &ServerMessage::SessionClosed).await;
                            break;
                        }
                    }
                    Ok(Err(error)) => {
                        if send_error(&mut sink, error).await.is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        if send_error(&mut sink, format!("command task failed: {error}"))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            ClientMessage::Clear => {
                if send_message(
                    &mut sink,
                    &ServerMessage::Output {
                        stream: OutputStream::System,
                        data: "\x1b[2J\x1b[3J\x1b[H".to_owned(),
                    },
                )
                .await
                .is_err()
                {
                    break;
                }
            }
            ClientMessage::Ping => {
                if send_message(&mut sink, &ServerMessage::Pong).await.is_err() {
                    break;
                }
            }
            ClientMessage::Interrupt => {
                if send_message(
                    &mut sink,
                    &ServerMessage::Output {
                        stream: OutputStream::System,
                        data: "^C\n".to_owned(),
                    },
                )
                .await
                .is_err()
                {
                    break;
                }
            }
            ClientMessage::Input { .. } => {
                if send_error(
                    &mut sink,
                    "raw input is only supported by /api/v1/pty",
                )
                .await
                .is_err()
                {
                    break;
                }
            }
            ClientMessage::Resize { .. } => {
                // Line-oriented sessions do not own a terminal window. The
                // client still stores its own size for rendering.
            }
        }
    }
}

async fn handle_pty_socket(socket: WebSocket, binary: PathBuf) {
    let (mut sink, mut stream) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(64);

    let writer_task = tokio::spawn(async move {
        while let Some(message) = out_rx.recv().await {
            if sink.send(message).await.is_err() {
                break;
            }
        }
    });

    let _ = out_tx
        .send(text_message(&ServerMessage::PtyReady {
            prompt: "kernext ~ $ ".to_owned(),
        }))
        .await;

    let command = ExternalCommand::new(binary.to_string_lossy().into_owned()).arg("--no-animation");
    let mut process = match PtyProcess::spawn(&command, PtySize::new(24, 80)) {
        Ok(process) => process,
        Err(error) => {
            let _ = out_tx
                .send(text_message(&ServerMessage::Error {
                    message: format!("could not start kernext on a pseudoterminal: {error}"),
                }))
                .await;
            return;
        }
    };

    let (Some(mut reader), Some(mut writer)) = (process.take_reader(), process.take_writer()) else {
        let _ = out_tx
            .send(text_message(&ServerMessage::Error {
                message: "the pseudoterminal did not expose its streams".to_owned(),
            }))
            .await;
        return;
    };

    let resizer = process.resizer();

    // The child is owned by this task so its handles live for the session.
    tokio::task::spawn_blocking(move || {
        let _ = process.wait();
    });

    let reader_tx = out_tx.clone();
    tokio::task::spawn_blocking(move || {
        let mut buffer = [0u8; 8192];

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    if reader_tx
                        .blocking_send(Message::Binary(buffer[..read].to_vec().into()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let (input_tx, mut input_rx) = mpsc::channel::<Vec<u8>>(64);
    tokio::task::spawn_blocking(move || {
        while let Some(bytes) = input_rx.blocking_recv() {
            if writer.write_all(&bytes).is_err() {
                break;
            }
        }
    });

    while let Some(message) = stream.next().await {
        let Ok(message) = message else {
            break;
        };

        let text = match message {
            Message::Text(text) => text.to_string(),
            Message::Close(_) => break,
            Message::Ping(payload) => {
                if out_tx.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
                continue;
            }
            _ => continue,
        };

        match serde_json::from_str::<ClientMessage>(&text) {
            Ok(ClientMessage::Input { data }) => {
                if input_tx.send(data.into_bytes()).await.is_err() {
                    break;
                }
            }
            Ok(ClientMessage::Execute { command }) => {
                if input_tx
                    .send(format!("{command}\r").into_bytes())
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Ok(ClientMessage::Resize { rows, cols }) => {
                let _ = resizer.resize(PtySize::new(rows, cols));
            }
            Ok(ClientMessage::Interrupt) => {
                if input_tx.send(vec![0x03]).await.is_err() {
                    break;
                }
            }
            Ok(ClientMessage::Clear) => {
                if input_tx.send(b"clear\r".to_vec()).await.is_err() {
                    break;
                }
            }
            Ok(ClientMessage::Ping) => {
                if out_tx
                    .send(text_message(&ServerMessage::Pong))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(error) => {
                if out_tx
                    .send(text_message(&ServerMessage::Error {
                        message: format!("invalid message: {error}"),
                    }))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    }

    // Closing the input stream lets a well-behaved shell exit. The `exit`
    // command is sent first so a foreground REPL does not treat EOF as an
    // incomplete line.
    let _ = input_tx.send(b"exit\r".to_vec()).await;
    drop(input_tx);
    drop(out_tx);
    let _ = writer_task.await;
}

async fn send_message(
    sink: &mut SplitSink<WebSocket, Message>,
    message: &ServerMessage,
) -> Result<(), axum::Error> {
    sink.send(text_message(message)).await
}

async fn send_error(
    sink: &mut SplitSink<WebSocket, Message>,
    message: impl Into<String>,
) -> Result<(), axum::Error> {
    send_message(
        sink,
        &ServerMessage::Error {
            message: message.into(),
        },
    )
    .await
}

fn text_message(message: &ServerMessage) -> Message {
    Message::Text(
        serde_json::to_string(message)
            .expect("KERNEXT server messages are serializable")
            .into(),
    )
}

/// Resolves the `kernext` executable used by the PTY endpoint.
///
/// Resolution order: `KERNEXT_BIN`, a sibling of the server executable, then
/// `PATH` through the native resolver. The result is only a candidate; the PTY
/// launch still validates it.
#[must_use]
pub fn pty_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("KERNEXT_BIN") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            for name in ["kernext.exe", "kernext"] {
                let candidate = directory.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    let working_directory = std::env::current_dir().ok()?;
    shell_builtins::resolve_executable("kernext", &working_directory).ok()
}
