//! Cross-interface consistency tests.
//!
//! The CLI, the web terminal, and the desktop terminal must behave the same
//! way because they share one engine. These tests drive the HTTP execute
//! endpoint — the exact path the web and desktop clients use — and assert the
//! behaviors the CLI test suite already pins down: help and version text,
//! working-directory tracking, pipelines, redirections, environment
//! variables, exit-code handling, session isolation, and clear/exit actions.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use shell_api::{CreateSessionResponse, ExecuteCommandResponse};
use shell_server::{AppState, router};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "kernext-consistency-{label}-{}-{unique}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temporary directory should be created");
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

async fn send_json(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .expect("request should build");

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router should answer");

    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body should collect")
        .to_bytes()
        .to_vec();

    (status, bytes)
}

/// Creates one session rooted at a fresh temporary directory and returns the
/// router plus the session id.
async fn session() -> (axum::Router, usize, TempDirectory) {
    let directory = TempDirectory::new("session");
    let app = router(AppState::from_env());

    let payload = serde_json::json!({
        "working_directory": directory.path().display().to_string()
    })
    .to_string();

    let (status, body) = send_json(&app, "POST", "/api/v1/sessions", &payload).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

    let created: CreateSessionResponse =
        serde_json::from_slice(&body).expect("create response should be JSON");
    let session = created.session.as_ref().expect("session should be created");
    assert!(created.success, "{created:?}");

    (app, session.id.0, directory)
}

async fn execute(app: &axum::Router, id: usize, command: &str) -> ExecuteCommandResponse {
    let payload = serde_json::json!({ "command": command }).to_string();
    let uri = format!("/api/v1/sessions/{id}/execute");
    let (status, body) = send_json(app, "POST", &uri, &payload).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

    serde_json::from_slice(&body).expect("execute response should be JSON")
}

#[tokio::test]
async fn help_version_and_prompt_match_the_cli() {
    let (app, id, _directory) = session().await;

    let help = execute(&app, id, "help").await;
    assert!(help.success, "{help:?}");
    assert!(
        help.output.starts_with("KERNEXT — Available commands:"),
        "{}",
        help.output
    );

    let version = execute(&app, id, "version").await;
    assert!(version.success, "{version:?}");
    assert!(
        version.output.starts_with("KERNEXT v"),
        "{}",
        version.output
    );

    let shown = execute(&app, id, "pwd").await;
    assert!(shown.success, "{shown:?}");
    assert_eq!(shown.session.prompt, "kernext ~ $ ");
}

#[tokio::test]
async fn pwd_cd_and_ls_track_the_session_directory() {
    let (app, id, directory) = session().await;

    let made = execute(&app, id, "mkdir workspace").await;
    assert!(made.success, "{made:?}");

    let moved = execute(&app, id, "cd workspace").await;
    assert!(moved.success, "{moved:?}");

    // The freshly created directory is empty, exactly as `ls` in the CLI
    // would report it after the change.
    let listed = execute(&app, id, "ls").await;
    assert!(listed.success, "{listed:?}");
    assert!(!listed.output.contains("workspace"), "{}", listed.output);

    let printed = execute(&app, id, "pwd").await;
    assert!(printed.success, "{printed:?}");
    assert!(printed.output.ends_with("workspace"), "{}", printed.output);

    // The session snapshot and the server-side engine must agree.
    assert!(
        printed.session.working_directory.ends_with("workspace"),
        "{}",
        printed.session.working_directory
    );

    // Going back must persist too, and the parent listing shows the directory.
    let back = execute(&app, id, "cd ..").await;
    assert!(back.success, "{back:?}");

    let listed = execute(&app, id, "ls").await;
    assert!(listed.success, "{listed:?}");
    assert!(listed.output.contains("workspace"), "{}", listed.output);

    assert!(
        directory.path().join("workspace").is_dir(),
        "mkdir must create a real directory"
    );
}

#[tokio::test]
async fn pipelines_and_redirections_behave_like_the_cli() {
    let (app, id, _directory) = session().await;

    let piped = execute(&app, id, "echo kernext pipeline | grep kernext").await;
    assert!(piped.success, "{piped:?}");
    assert!(
        piped.output.contains("kernext pipeline"),
        "{}",
        piped.output
    );

    let redirected = execute(&app, id, "echo redirected > notes.txt").await;
    assert!(redirected.success, "{redirected:?}");

    let read_back = execute(&app, id, "cat notes.txt").await;
    assert!(read_back.success, "{read_back:?}");
    assert!(
        read_back.output.contains("redirected"),
        "{}",
        read_back.output
    );
}

#[tokio::test]
async fn environment_variables_and_exit_status_behave_like_the_cli() {
    let (app, id, _directory) = session().await;

    let exported = execute(&app, id, "export CONSISTENCY_MARKER=engine-wide").await;
    assert!(exported.success, "{exported:?}");

    let environment = execute(&app, id, "env").await;
    assert!(environment.success, "{environment:?}");
    assert!(
        environment
            .output
            .contains("CONSISTENCY_MARKER=engine-wide"),
        "{}",
        environment.output
    );

    let failed = execute(&app, id, "false").await;
    assert!(!failed.success, "{failed:?}");

    // `$?` must reflect the failing status, exactly as in the CLI.
    let status = execute(&app, id, "echo $?").await;
    assert!(status.success, "{status:?}");
    assert_eq!(status.output.trim(), "1", "{}", status.output);

    let recovered = execute(&app, id, "echo $?").await;
    assert_eq!(recovered.output.trim(), "0", "{}", recovered.output);
}

#[tokio::test]
async fn clear_and_exit_actions_surface_over_http() {
    let (app, id, _directory) = session().await;

    let cleared = execute(&app, id, "clear").await;
    assert!(cleared.success, "{cleared:?}");
    assert_eq!(cleared.action, "clear");
    assert!(cleared.output.is_empty(), "{}", cleared.output);

    let exited = execute(&app, id, "exit").await;
    assert!(exited.success, "{exited:?}");
    assert_eq!(exited.action, "exit");
}

#[tokio::test]
async fn sessions_are_isolated_from_each_other() {
    let (app_one, id_one, _directory_one) = session().await;
    let (app_two, id_two, _directory_two) = session().await;

    let exported = execute(&app_one, id_one, "export ISOLATION=first-session").await;
    assert!(exported.success, "{exported:?}");

    let made = execute(&app_one, id_one, "mkdir only-in-one").await;
    assert!(made.success, "{made:?}");

    let other = execute(&app_two, id_two, "env").await;
    assert!(other.success, "{other:?}");
    assert!(
        !other.output.contains("ISOLATION"),
        "variables must not leak between sessions: {}",
        other.output
    );

    let other_directory = execute(&app_two, id_two, "ls").await;
    assert!(
        !other_directory.output.contains("only-in-one"),
        "working directories must not leak between sessions: {}",
        other_directory.output
    );
}

#[tokio::test]
async fn command_history_is_per_session() {
    let (app, id, _directory) = session().await;

    let first = execute(&app, id, "echo first").await;
    assert!(first.success, "{first:?}");

    let history = execute(&app, id, "history").await;
    assert!(history.success, "{history:?}");
    assert!(history.output.contains("echo first"), "{}", history.output);
}
