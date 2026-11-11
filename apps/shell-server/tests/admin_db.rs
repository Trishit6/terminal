//! MariaDB-backed administration integration tests.
//!
//! Like `auth_db.rs`, these tests need a disposable MySQL/MariaDB database.
//! They run only when `KERNEXT_TEST_DATABASE_URL` is set, or when
//! `DATABASE_URL` names a database containing `test`. They never create or
//! drop databases and never touch a production database.

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::json;
use shell_server::db::{AdminSeed, AuthConfig, Database};
use shell_server::state::AuthContext;
use shell_server::{AppState, router};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

const PASSWORD: &str = "kernext-test-123";
const ADMIN_PASSWORD: &str = "kernext-admin-901";

struct TestApp {
    app: axum::Router,
    db: Database,
    admin: Jar,
    admin_id: String,
    admin_email: String,
}

static MIGRATION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup() -> Option<TestApp> {
    let url = test_database_url()?;
    let db = Database::connect(&url)
        .await
        .expect("test database should connect");
    {
        let _guard = MIGRATION_LOCK.lock().await;
        db.migrate().await.expect("migrations should apply");
    }

    let config = AuthConfig {
        secure_cookies: false,
        dev_mail: true,
        public_url: "http://127.0.0.1:5173".to_owned(),
        session_hours: 12,
        remember_days: 30,
    };
    let accounts = AuthContext::new(db.clone(), config);
    let state = AppState::local().with_accounts(accounts);
    let app = router(state);

    let admin_email = unique_email("console-admin");
    let seeded = db
        .seed_admin("KERNEXT Administrator", &admin_email, "password")
        .await
        .expect("admin seed should run");
    assert_eq!(seeded, AdminSeed::Created);

    let mut admin = Jar::default();
    let (status, body) = login(&app, &admin_email, "password", &mut admin).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let admin_id = body["user"]["id"]
        .as_str()
        .expect("login returns the account id")
        .to_owned();

    // The seeded administrator must replace the initial password before the
    // console is usable; this also proves the audit/authorization path is not
    // special-cased for the seed.
    let csrf = admin.get("kernext_csrf").expect("csrf cookie").to_owned();
    let (status, _headers, body) = send(
        &app,
        request(
            "POST",
            "/api/auth/change-password",
            Some(&json!({
                "current_password": "password",
                "new_password": ADMIN_PASSWORD,
                "sign_out_other_sessions": false,
            })),
            Some(&admin),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    Some(TestApp {
        app,
        db,
        admin,
        admin_id,
        admin_email,
    })
}

fn test_database_url() -> Option<String> {
    if let Ok(value) = std::env::var("KERNEXT_TEST_DATABASE_URL")
        && !value.trim().is_empty()
    {
        return Some(value);
    }

    let value = std::env::var("DATABASE_URL").ok()?;
    if value.trim().is_empty() {
        return None;
    }

    let database = value.rsplit('/').next().unwrap_or_default();
    if database.contains("test") {
        Some(value)
    } else {
        eprintln!(
            "skipping MariaDB admin tests: DATABASE_URL does not look like an isolated test database"
        );
        None
    }
}

fn unique_email(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after the epoch")
        .as_nanos();
    format!(
        "kernext-admin-{label}-{}-{nanos}@example.com",
        std::process::id()
    )
}

fn unique_token(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after the epoch")
        .as_nanos();
    format!("{label}{nanos}")
}

#[derive(Default)]
struct Jar {
    values: HashMap<String, String>,
}

impl Jar {
    fn absorb(&mut self, headers: &HeaderMap) {
        for value in headers.get_all(header::SET_COOKIE) {
            let Ok(value) = value.to_str() else { continue };
            let pair = value.split(';').next().unwrap_or_default();
            let Some((name, value)) = pair.split_once('=') else {
                continue;
            };
            if value.is_empty() {
                self.values.remove(name);
            } else {
                self.values.insert(name.to_owned(), value.to_owned());
            }
        }
    }

    fn cookie_header(&self) -> Option<HeaderValue> {
        if self.values.is_empty() {
            return None;
        }
        let joined = self
            .values
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        HeaderValue::from_str(&joined).ok()
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }
}

fn json_request(method: &str, uri: &str, body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

fn request(
    method: &str,
    uri: &str,
    body: Option<&serde_json::Value>,
    jar: Option<&Jar>,
    csrf: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);

    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some(cookie) = jar.and_then(Jar::cookie_header) {
        builder = builder.header(header::COOKIE, cookie);
    }
    if let Some(csrf) = csrf {
        builder = builder.header("x-csrf-token", csrf);
    }

    builder
        .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
        .expect("request should build")
}

async fn send(
    app: &axum::Router,
    request: Request<Body>,
) -> (StatusCode, HeaderMap, serde_json::Value) {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router should answer");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body should collect")
        .to_bytes();
    let body = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, headers, body)
}

fn admin_csrf(test: &TestApp) -> String {
    test.admin
        .get("kernext_csrf")
        .expect("admin csrf cookie")
        .to_owned()
}

async fn register(app: &axum::Router, email: &str) -> (Jar, String) {
    let mut jar = Jar::default();
    let (status, headers, body) = send(
        app,
        json_request(
            "POST",
            "/api/auth/register",
            &json!({
                "full_name": "KERNEXT Member",
                "email": email,
                "password": PASSWORD,
                "accept_terms": true,
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    jar.absorb(&headers);
    let id = body["user"]["id"]
        .as_str()
        .expect("registration returns the account id")
        .to_owned();
    (jar, id)
}

async fn login(
    app: &axum::Router,
    email: &str,
    password: &str,
    jar: &mut Jar,
) -> (StatusCode, serde_json::Value) {
    let (status, headers, body) = send(
        app,
        json_request(
            "POST",
            "/api/auth/login",
            &json!({ "email": email, "password": password }),
        ),
    )
    .await;
    jar.absorb(&headers);
    (status, body)
}

#[tokio::test]
async fn admin_endpoints_require_authentication() {
    let Some(test) = setup().await else {
        return;
    };

    for path in [
        "/api/admin/overview",
        "/api/admin/users",
        "/api/admin/health",
        "/api/admin/audit-logs",
        "/api/admin/terminal-sessions",
    ] {
        let (status, _headers, body) =
            send(&test.app, request("GET", path, None, None, None)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}: {body}");
    }
}

#[tokio::test]
async fn normal_users_are_rejected_by_every_admin_surface() {
    let Some(test) = setup().await else {
        return;
    };
    let (member, member_id) = register(&test.app, &unique_email("member")).await;
    let member_csrf = member.get("kernext_csrf").expect("csrf cookie").to_owned();

    for path in [
        "/api/admin/overview",
        "/api/admin/users",
        "/api/admin/health",
        "/api/admin/audit-logs",
        "/api/admin/terminal-sessions",
        &format!("/api/admin/users/{member_id}"),
        &format!("/api/admin/users/{member_id}/sessions"),
    ] {
        let (status, _headers, body) =
            send(&test.app, request("GET", path, None, Some(&member), None)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}: {body}");
        assert_eq!(body["code"], "forbidden");
    }

    // A member cannot promote themselves through the admin route either.
    let (status, _headers, body) = send(
        &test.app,
        request(
            "POST",
            &format!("/api/admin/users/{member_id}/role"),
            Some(&json!({ "role": "admin" })),
            Some(&member),
            Some(&member_csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

#[tokio::test]
async fn user_list_search_and_pagination_are_database_backed() {
    let Some(test) = setup().await else {
        return;
    };

    let token = unique_token("searchable");
    for suffix in ["a", "b"] {
        test.db
            .create_user(
                &format!("Zebra {token} {suffix}"),
                &unique_email(&format!("{token}-{suffix}")),
                PASSWORD,
            )
            .await
            .expect("test account should be created");
    }
    test.db
        .create_user("Unrelated Account", &unique_email("unrelated"), PASSWORD)
        .await
        .expect("test account should be created");

    let (status, _headers, first) = send(
        &test.app,
        request(
            "GET",
            &format!("/api/admin/users?search={token}&per_page=1&page=1"),
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["total"], 2, "{first}");
    assert_eq!(first["page"], 1);
    assert_eq!(first["per_page"], 1);
    assert_eq!(first["users"].as_array().map(Vec::len), Some(1), "{first}");
    assert!(
        !first.to_string().contains("password_hash"),
        "the admin list must never expose password material"
    );

    let (status, _headers, second) = send(
        &test.app,
        request(
            "GET",
            &format!("/api/admin/users?search={token}&per_page=1&page=2"),
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{second}");
    assert_eq!(second["users"].as_array().map(Vec::len), Some(1));

    let first_id = first["users"][0]["id"].clone();
    let second_id = second["users"][0]["id"].clone();
    assert_ne!(
        first_id, second_id,
        "pagination must not repeat the same row"
    );

    // A wildcard in the search term is escaped, not treated as a match-all.
    let (status, _headers, wildcard) = send(
        &test.app,
        request(
            "GET",
            "/api/admin/users?search=%25&per_page=5",
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{wildcard}");
    assert_eq!(wildcard["total"], 0, "{wildcard}");
}

#[tokio::test]
async fn role_changes_are_validated_audited_and_self_protected() {
    let Some(test) = setup().await else {
        return;
    };
    let (member, member_id) = register(&test.app, &unique_email("role-target")).await;
    let csrf = admin_csrf(&test);

    let (status, _headers, body) = send(
        &test.app,
        request(
            "POST",
            &format!("/api/admin/users/{member_id}/role"),
            Some(&json!({ "role": "admin" })),
            Some(&test.admin),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["role"], "admin");

    let (status, _headers, body) = send(
        &test.app,
        request(
            "POST",
            &format!("/api/admin/users/{member_id}/role"),
            Some(&json!({ "role": "user" })),
            Some(&test.admin),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["role"], "user");

    // Invalid roles are rejected rather than coerced.
    let (status, _headers, body) = send(
        &test.app,
        request(
            "POST",
            &format!("/api/admin/users/{member_id}/role"),
            Some(&json!({ "role": "superuser" })),
            Some(&test.admin),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // Self-promotion/demotion from the console is refused.
    let (status, _headers, body) = send(
        &test.app,
        request(
            "POST",
            &format!("/api/admin/users/{}/role", test.admin_id),
            Some(&json!({ "role": "user" })),
            Some(&test.admin),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    // The member's own session still works as a member, never as an admin.
    let (status, _headers, me) = send(
        &test.app,
        request("GET", "/api/auth/me", None, Some(&member), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["role"], "user");

    let (status, _headers, audit) = send(
        &test.app,
        request(
            "GET",
            "/api/admin/audit-logs?event_type=user.role_changed&per_page=100",
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{audit}");
    assert!(
        audit["events"]
            .as_array()
            .is_some_and(|events| events.iter().any(|event| {
                event["target_user_id"] == member_id.as_str()
                    && event["actor_email"] == test.admin_email.as_str()
            })),
        "role changes must be persisted with actor and target: {audit}"
    );
}

#[tokio::test]
async fn deactivation_revokes_access_and_reactivation_restores_it() {
    let Some(test) = setup().await else {
        return;
    };
    let email = unique_email("status-target");
    let (member, member_id) = register(&test.app, &email).await;
    let csrf = admin_csrf(&test);

    let (status, _headers, body) = send(
        &test.app,
        request(
            "POST",
            &format!("/api/admin/users/{member_id}/status"),
            Some(&json!({ "status": "disabled" })),
            Some(&test.admin),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["account_status"], "disabled");

    // The existing session is invalid immediately.
    let (status, _headers, _body) = send(
        &test.app,
        request("GET", "/api/auth/me", None, Some(&member), None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let mut fresh = Jar::default();
    let (status, _body) = login(&test.app, &email, PASSWORD, &mut fresh).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Self-deactivation is blocked.
    let (status, _headers, body) = send(
        &test.app,
        request(
            "POST",
            &format!("/api/admin/users/{}/status", test.admin_id),
            Some(&json!({ "status": "disabled" })),
            Some(&test.admin),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    let (status, _headers, body) = send(
        &test.app,
        request(
            "POST",
            &format!("/api/admin/users/{member_id}/status"),
            Some(&json!({ "status": "active" })),
            Some(&test.admin),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["account_status"], "active");

    let mut restored = Jar::default();
    let (status, body) = login(&test.app, &email, PASSWORD, &mut restored).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, _headers, audit) = send(
        &test.app,
        request(
            "GET",
            "/api/admin/audit-logs?event_type=user.status_changed&per_page=100",
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{audit}");
    assert!(
        audit["events"].as_array().is_some_and(|events| events
            .iter()
            .any(|event| event["target_user_id"] == member_id.as_str())),
        "account status changes must be audited: {audit}"
    );
}

#[tokio::test]
async fn administrators_can_revoke_every_account_session() {
    let Some(test) = setup().await else {
        return;
    };
    let email = unique_email("revoke-target");
    let (first, member_id) = register(&test.app, &email).await;
    let mut second = Jar::default();
    let (status, body) = login(&test.app, &email, PASSWORD, &mut second).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, _headers, body) = send(
        &test.app,
        request(
            "DELETE",
            &format!("/api/admin/users/{member_id}/sessions"),
            None,
            Some(&test.admin),
            Some(&admin_csrf(&test)),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body["revoked"].as_u64().unwrap_or(0) >= 2,
        "both live sessions must be revoked: {body}"
    );

    for jar in [&first, &second] {
        let (status, _headers, _body) = send(
            &test.app,
            request("GET", "/api/auth/me", None, Some(jar), None),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    let (status, _headers, audit) = send(
        &test.app,
        request(
            "GET",
            "/api/admin/audit-logs?event_type=user.sessions_revoked&per_page=50",
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{audit}");
    assert!(
        audit["events"].as_array().is_some_and(|events| events
            .iter()
            .any(|event| event["target_user_id"] == member_id.as_str())),
        "session revocation must be audited"
    );
}

#[tokio::test]
async fn terminal_sessions_are_listed_safely_and_can_be_terminated() {
    let Some(test) = setup().await else {
        return;
    };
    let member_email = unique_email("terminal-owner");
    let (member, member_id) = register(&test.app, &member_email).await;

    let (status, _headers, created) = send(
        &test.app,
        request(
            "POST",
            "/api/v1/sessions",
            Some(&json!({})),
            Some(&member),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let session_id = created["session"]["id"]
        .as_u64()
        .expect("session id should be numeric");
    let dashboard_id = format!("engine-{session_id}");

    let (status, _headers, listed) = send(
        &test.app,
        request(
            "GET",
            "/api/admin/terminal-sessions",
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let session = listed["sessions"]
        .as_array()
        .expect("sessions array")
        .iter()
        .find(|session| session["id"] == dashboard_id.as_str())
        .unwrap_or_else(|| panic!("terminal session should be listed: {listed}"));

    assert_eq!(session["owner"], member_id.as_str());
    assert_eq!(session["owner_email"], member_email.as_str());
    assert_eq!(session["mode"], "engine");
    let rendered = listed.to_string();
    assert!(
        !rendered.contains("\"output\""),
        "no terminal output may be exposed"
    );
    assert!(
        !rendered.contains("\"command\""),
        "no command input may be exposed"
    );
    assert!(
        !rendered.contains("\"variables\""),
        "no shell variables may be exposed"
    );

    let (status, _headers, _body) = send(
        &test.app,
        request(
            "DELETE",
            &format!("/api/admin/terminal-sessions/{dashboard_id}"),
            None,
            Some(&test.admin),
            Some(&admin_csrf(&test)),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _headers, _body) = send(
        &test.app,
        request(
            "GET",
            &format!("/api/v1/sessions/{session_id}"),
            None,
            Some(&member),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "the terminated session must be gone"
    );

    let (status, _headers, _body) = send(
        &test.app,
        request(
            "DELETE",
            &format!("/api/admin/terminal-sessions/{dashboard_id}"),
            None,
            Some(&test.admin),
            Some(&admin_csrf(&test)),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _headers, audit) = send(
        &test.app,
        request(
            "GET",
            "/api/admin/audit-logs?event_type=terminal.session_terminated&per_page=50",
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{audit}");
    assert!(
        audit["events"].as_array().is_some_and(|events| {
            events
                .iter()
                .any(|event| event["metadata"]["terminal_id"] == dashboard_id.as_str())
        }),
        "termination must be audited: {audit}"
    );
}

#[tokio::test]
async fn health_checks_reflect_real_database_state() {
    let Some(test) = setup().await else {
        return;
    };

    let (status, _headers, body) = send(
        &test.app,
        request("GET", "/api/admin/health", None, Some(&test.admin), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        matches!(body["status"].as_str(), Some("healthy" | "degraded")),
        "{body}"
    );
    assert_eq!(body["server"]["version"], shell_core::VERSION);
    assert_eq!(body["database"]["configured"], true);
    assert_eq!(
        body["database"]["migrations_applied"],
        body["database"]["migrations_expected"]
    );
    assert!(
        body["checks"]
            .as_array()
            .is_some_and(|checks| checks.iter().any(|check| check["id"] == "database")),
        "database check must be present"
    );
    let rendered = body.to_string();
    assert!(!rendered.contains("DATABASE_URL"));
    assert!(!rendered.contains("password"));
}

#[tokio::test]
async fn local_mode_administration_is_unavailable() {
    let app = router(AppState::local());
    let (status, _headers, body) =
        send(&app, request("GET", "/api/admin/health", None, None, None)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["code"], "unavailable");
}

#[tokio::test]
async fn audit_log_is_admin_only_filtered_and_paginated() {
    let Some(test) = setup().await else {
        return;
    };
    let (member, _member_id) = register(&test.app, &unique_email("audit-member")).await;

    let (status, _headers, _body) = send(
        &test.app,
        request("GET", "/api/admin/audit-logs", None, Some(&member), None),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _headers, body) = send(
        &test.app,
        request(
            "GET",
            "/api/admin/audit-logs?category=not-a-category",
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let (status, _headers, page) = send(
        &test.app,
        request(
            "GET",
            "/api/admin/audit-logs?category=auth&per_page=3",
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    let events = page["events"].as_array().expect("events array");
    assert!(events.len() <= 3);
    assert!(
        events.iter().all(|event| event["event_type"]
            .as_str()
            .is_some_and(|kind| kind.starts_with("auth."))),
        "category filter must not leak other event types: {page}"
    );
    assert!(page["total"].as_u64().unwrap_or(0) >= 1);

    let (status, _headers, empty) = send(
        &test.app,
        request(
            "GET",
            "/api/admin/audit-logs?event_type=bogus.event&per_page=5",
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{empty}");
    assert_eq!(empty["total"], 0);
    assert_eq!(empty["events"].as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn sign_in_records_last_login_and_an_audit_event() {
    let Some(test) = setup().await else {
        return;
    };

    let (status, _headers, body) = send(
        &test.app,
        request(
            "GET",
            &format!(
                "/api/admin/users?search={}&per_page=5",
                test.admin_email.replace('@', "%40")
            ),
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let user = &body["users"][0];
    assert_eq!(user["email"], test.admin_email.as_str());
    assert!(
        user["last_login_at"].is_string(),
        "a successful sign-in must record last_login_at: {body}"
    );

    let (status, _headers, body) = send(
        &test.app,
        request(
            "GET",
            "/api/admin/audit-logs?event_type=auth.login&per_page=50",
            None,
            Some(&test.admin),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body["events"].as_array().is_some_and(|events| events
            .iter()
            .any(|event| event["actor_email"] == test.admin_email.as_str())),
        "successful sign-ins must be audited: {body}"
    );
}
