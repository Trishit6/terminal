//! WebSocket integration coverage.
//!
//! These tests start a real Axum listener on a loopback port, create a real
//! in-process KERNEXT session, and execute commands over the same wire protocol
//! the browser terminal uses.

use futures_util::{SinkExt, StreamExt};
use shell_api::{ClientMessage, CreateSessionRequest, ServerMessage};
use shell_server::{AppState, router};
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn websocket_executes_commands_through_the_engine() {
    let state = AppState::from_env();
    let session = state
        .create(&CreateSessionRequest::default())
        .expect("session should start");
    let id = session.lock().expect("session should lock").id().0;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener.local_addr().expect("address should exist");
    let app = router(state);

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server should run");
    });

    let (mut socket, _response) = connect_async(format!("ws://{address}/api/v1/sessions/{id}/ws"))
        .await
        .expect("websocket should connect");

    let ready = socket
        .next()
        .await
        .expect("ready frame should arrive")
        .expect("ready frame should be valid");
    let Message::Text(ready) = ready else {
        panic!("expected a text ready frame");
    };
    let ready: ServerMessage = serde_json::from_str(&ready).expect("ready frame should be JSON");
    assert!(matches!(ready, ServerMessage::Ready { .. }));

    socket
        .send(Message::Text(
            serde_json::to_string(&ClientMessage::Execute {
                command: "echo ws-hello".to_owned(),
            })
            .expect("client message should serialize")
            .into(),
        ))
        .await
        .expect("command should send");

    let mut output = String::new();
    let mut result = None;

    for _ in 0..8 {
        let frame = socket
            .next()
            .await
            .expect("frame should arrive")
            .expect("frame should be valid");

        let Message::Text(text) = frame else {
            continue;
        };
        let message: ServerMessage =
            serde_json::from_str(&text).expect("server frame should be JSON");

        match message {
            ServerMessage::Output { data, .. } => output.push_str(&data),
            ServerMessage::Result {
                success,
                exit_code,
                action,
                ..
            } => {
                result = Some((success, exit_code, action));
                break;
            }
            ServerMessage::Error { message } => panic!("server error: {message}"),
            _ => {}
        }
    }

    assert!(
        output.contains("ws-hello"),
        "output did not contain the command result: {output:?}"
    );
    assert_eq!(result, Some((true, None, "continue".to_owned())));

    drop(socket);
    server.abort();
}
