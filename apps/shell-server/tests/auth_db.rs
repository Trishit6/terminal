//! MariaDB-backed authentication and account-ownership integration tests.
//!
//! These tests need a disposable MySQL/MariaDB database. They run only when
//! `KERNEXT_TEST_DATABASE_URL` is set (CI supplies a service container), or
//! when `DATABASE_URL` points at a database whose name contains `test`. They
//! never run against a production database, and they do not create or drop
//! databases themselves: the schema comes from the embedded migrations, which
//! are idempotent.

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::json;
use shell_server::db::{AuthConfig, Database};
use shell_server::state::AuthContext;
use shell_server::{AppState, router};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

const PASSWORD: &str = "kernext-test-123";
const NEW_PASSWORD: &str = "kernext-test-456";

struct TestApp {
    app: axum::Router,
    db: Database,
}

/// Tests run in parallel; serialize the embedded migration runner so two
/// tests cannot insert the same migration version at the same time.
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

    Some(TestApp {
        app: router(state),
        db,
    })
}

/// Selects an explicitly isolated test database only.
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

    // Refuse to write into a database that is not clearly a test database.
    let database = value.rsplit('/').next().unwrap_or_default();
    if database.contains("test") {
        Some(value)
    } else {
        eprintln!(
            "skipping MariaDB auth tests: DATABASE_URL does not look like an isolated test database"
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
        "kernext-test-{label}-{}-{nanos}@example.com",
        std::process::id()
    )
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

async fn register(app: &axum::Router, email: &str, jar: &mut Jar) -> serde_json::Value {
    let (status, headers, body) = send(
        app,
        json_request(
            "POST",
            "/api/auth/register",
            &json!({
                "full_name": "KERNEXT Test",
                "email": email,
                "password": PASSWORD,
                "accept_terms": true,
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    jar.absorb(&headers);
    body
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
async fn registration_login_logout_roundtrip() {
    let Some(test) = setup().await else {
        return;
    };
    let email = unique_email("roundtrip");
    let mut jar = Jar::default();

    let registered = register(&test.app, &email, &mut jar).await;
    assert_eq!(registered["user"]["email"], email);
    assert_eq!(registered["email_verification_required"], true);

    // Duplicate registration is a stable conflict, not a database error.
    let (status, _headers, body) = send(
        &test.app,
        json_request(
            "POST",
            "/api/auth/register",
            &json!({
                "full_name": "KERNEXT Test",
                "email": email,
                "password": PASSWORD,
                "accept_terms": true,
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    // Unauthenticated access is rejected.
    let (status, _headers, _body) =
        send(&test.app, request("GET", "/api/auth/me", None, None, None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _headers, me) = send(
        &test.app,
        request("GET", "/api/auth/me", None, Some(&jar), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me["user"]["email"], email);
    assert!(
        !me.to_string().contains("password_hash"),
        "public profiles must never expose password material"
    );

    let old_cookie = jar.get("kernext_session").map(ToOwned::to_owned);

    // Logout revokes the server-side session and clears cookies.
    let (status, headers, _body) = send(
        &test.app,
        request("POST", "/api/auth/logout", None, Some(&jar), None),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let mut cleared = Jar::default();
    cleared.absorb(&headers);
    assert!(
        cleared.values.is_empty(),
        "logout should clear both cookies"
    );

    // The revoked cookie cannot be reused even if the client kept it.
    let cookie = HeaderValue::from_str(&format!("kernext_session={}", old_cookie.unwrap()))
        .expect("cookie should build");
    let (status, _headers, _body) = send(
        &test.app,
        Request::builder()
            .uri("/api/auth/me")
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn csrf_protection_guards_state_changing_requests() {
    let Some(test) = setup().await else {
        return;
    };
    let email = unique_email("csrf");
    let mut jar = Jar::default();
    register(&test.app, &email, &mut jar).await;

    let profile = json!({ "full_name": "KERNEXT Renamed", "email": email });

    let (status, _headers, body) = send(
        &test.app,
        request(
            "PATCH",
            "/api/auth/profile",
            Some(&profile),
            Some(&jar),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "csrf_failed");

    let csrf = jar.get("kernext_csrf").expect("csrf cookie").to_owned();
    let (status, _headers, body) = send(
        &test.app,
        request(
            "PATCH",
            "/api/auth/profile",
            Some(&profile),
            Some(&jar),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["full_name"], "KERNEXT Renamed");
}

#[tokio::test]
async fn password_reset_is_single_use_and_revokes_sessions() {
    let Some(test) = setup().await else {
        return;
    };
    let email = unique_email("reset");
    let mut jar = Jar::default();
    register(&test.app, &email, &mut jar).await;
    let old_cookie = jar
        .get("kernext_session")
        .expect("session cookie")
        .to_owned();

    let (status, _headers, body) = send(
        &test.app,
        json_request(
            "POST",
            "/api/auth/forgot-password",
            &json!({ "email": email }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let token = body["dev_reset_token"]
        .as_str()
        .expect("development mail should return the token")
        .to_owned();

    let (status, _headers, body) = send(
        &test.app,
        json_request(
            "POST",
            "/api/auth/reset-password",
            &json!({ "token": token, "password": NEW_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // Every existing session is invalidated.
    let cookie = HeaderValue::from_str(&format!("kernext_session={old_cookie}"))
        .expect("cookie should build");
    let (status, _headers, _body) = send(
        &test.app,
        Request::builder()
            .uri("/api/auth/me")
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // The old password no longer works, the new one does.
    let mut stale = Jar::default();
    let (status, _body) = login(&test.app, &email, PASSWORD, &mut stale).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let mut fresh = Jar::default();
    let (status, _body) = login(&test.app, &email, NEW_PASSWORD, &mut fresh).await;
    assert_eq!(status, StatusCode::OK);

    // A consumed reset token is rejected.
    let (status, _headers, body) = send(
        &test.app,
        json_request(
            "POST",
            "/api/auth/reset-password",
            &json!({ "token": token, "password": NEW_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn email_verification_updates_the_public_profile() {
    let Some(test) = setup().await else {
        return;
    };
    let email = unique_email("verify");
    let mut jar = Jar::default();
    let registered = register(&test.app, &email, &mut jar).await;
    let token = registered["dev_verification_token"]
        .as_str()
        .expect("development mail should return the token")
        .to_owned();

    let (status, _headers, body) = send(
        &test.app,
        json_request("POST", "/api/auth/verify-email", &json!({ "token": token })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, _headers, me) = send(
        &test.app,
        request("GET", "/api/auth/me", None, Some(&jar), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["email_verified"], true);

    let csrf = jar.get("kernext_csrf").expect("csrf cookie").to_owned();
    let (status, _headers, body) = send(
        &test.app,
        request(
            "POST",
            "/api/auth/resend-verification",
            None,
            Some(&jar),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["email_delivery"], "already_verified");
}

#[tokio::test]
async fn terminal_sessions_are_scoped_to_their_owner() {
    let Some(test) = setup().await else {
        return;
    };
    let mut alice = Jar::default();
    register(&test.app, &unique_email("alice"), &mut alice).await;
    let mut bob = Jar::default();
    register(&test.app, &unique_email("bob"), &mut bob).await;

    // Creating a terminal without an account is rejected in account mode.
    let (status, _headers, _body) = send(
        &test.app,
        json_request("POST", "/api/v1/sessions", &json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _headers, created) = send(
        &test.app,
        request(
            "POST",
            "/api/v1/sessions",
            Some(&json!({})),
            Some(&alice),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let id = created["session"]["id"]
        .as_u64()
        .expect("session id should be numeric");

    // Bob cannot see or control Alice's terminal.
    let (status, _headers, listed) = send(
        &test.app,
        request("GET", "/api/v1/sessions", None, Some(&bob), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["sessions"].as_array().map(Vec::len), Some(0));

    for method in ["GET", "DELETE"] {
        let (status, _headers, _body) = send(
            &test.app,
            request(
                method,
                &format!("/api/v1/sessions/{id}"),
                None,
                Some(&bob),
                None,
            ),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "{method} by a non-owner must not reveal the session"
        );
    }

    let (status, _headers, _body) = send(
        &test.app,
        request(
            "POST",
            &format!("/api/v1/sessions/{id}/execute"),
            Some(&json!({ "command": "echo stolen" })),
            Some(&bob),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The owner can execute on it.
    let (status, _headers, body) = send(
        &test.app,
        request(
            "POST",
            &format!("/api/v1/sessions/{id}/execute"),
            Some(&json!({ "command": "echo owned" })),
            Some(&alice),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body["output"]
            .as_str()
            .unwrap_or_default()
            .contains("owned")
    );
}

#[tokio::test]
async fn only_password_and_token_hashes_are_persisted() {
    let Some(test) = setup().await else {
        return;
    };
    let email = unique_email("hashes");
    let mut jar = Jar::default();
    register(&test.app, &email, &mut jar).await;

    let password_hash: String =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE email = ?")
            .bind(&email)
            .fetch_one(test.db.pool())
            .await
            .expect("user row should exist");
    assert!(
        password_hash.starts_with("$argon2id$"),
        "passwords must be Argon2id hashes, got {password_hash}"
    );
    assert!(!password_hash.contains(PASSWORD));

    let raw_session = jar
        .get("kernext_session")
        .expect("session cookie")
        .to_owned();
    let stored: String = sqlx::query_scalar(
        "SELECT session_token_hash FROM user_sessions WHERE session_token_hash = ?",
    )
    .bind(shell_server::db::hash_token(&raw_session))
    .fetch_one(test.db.pool())
    .await
    .expect("session row should store only the hash");
    assert_ne!(stored, raw_session);
    assert_eq!(stored.len(), 64);
}

#[tokio::test]
async fn admin_seed_is_idempotent_and_forces_a_password_change() {
    let Some(test) = setup().await else {
        return;
    };
    let email = unique_email("admin");

    let first = test
        .db
        .seed_admin("admin", &email, "password")
        .await
        .expect("admin seed should run");
    assert_eq!(first, shell_server::db::AdminSeed::Created);

    let second = test
        .db
        .seed_admin("admin", &email, "different-password")
        .await
        .expect("second admin seed should run");
    assert_eq!(
        second,
        shell_server::db::AdminSeed::Existing,
        "re-running the seed must not duplicate or overwrite the account"
    );

    // Only an Argon2id hash is stored; the seeded password is not plaintext.
    let (hash, role, must_change): (String, String, bool) = sqlx::query_as(
        "SELECT password_hash, role, must_change_password FROM users WHERE email = ?",
    )
    .bind(&email)
    .fetch_one(test.db.pool())
    .await
    .expect("admin row should exist");
    assert!(hash.starts_with("$argon2id$"));
    assert_ne!(hash, "password");
    assert_eq!(role, "admin");
    assert!(must_change);

    let mut jar = Jar::default();
    let (status, logged_in) = login(&test.app, &email, "password", &mut jar).await;
    assert_eq!(status, StatusCode::OK, "{logged_in}");
    assert_eq!(logged_in["user"]["role"], "admin");
    assert_eq!(logged_in["user"]["must_change_password"], true);
    assert!(
        !logged_in.to_string().contains("password_hash"),
        "login must never return password material"
    );

    // Admin APIs stay blocked until the initial password is replaced.
    let (status, _headers, body) = send(
        &test.app,
        request("GET", "/api/admin/users", None, Some(&jar), None),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "password_change_required");

    let csrf = jar.get("kernext_csrf").expect("csrf cookie").to_owned();
    let (status, _headers, body) = send(
        &test.app,
        request(
            "POST",
            "/api/auth/change-password",
            Some(&json!({
                "current_password": "password",
                "new_password": "kernext-admin-789",
                "sign_out_other_sessions": false,
            })),
            Some(&jar),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, _headers, me) = send(
        &test.app,
        request("GET", "/api/auth/me", None, Some(&jar), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["must_change_password"], false);

    let (status, _headers, users) = send(
        &test.app,
        request("GET", "/api/admin/users", None, Some(&jar), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{users}");
    assert!(
        users["users"]
            .as_array()
            .is_some_and(|users| users.iter().any(|user| user["email"] == email)),
        "admin list should include the seeded administrator"
    );
}

#[tokio::test]
async fn normal_users_cannot_reach_admin_endpoints_or_promote_themselves() {
    let Some(test) = setup().await else {
        return;
    };
    let email = unique_email("member");
    let mut jar = Jar::default();
    register(&test.app, &email, &mut jar).await;

    let (status, _headers, body) = send(
        &test.app,
        request("GET", "/api/admin/users", None, Some(&jar), None),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "forbidden");

    // Extra role fields in a profile update are ignored, never applied.
    let csrf = jar.get("kernext_csrf").expect("csrf cookie").to_owned();
    let (status, _headers, body) = send(
        &test.app,
        request(
            "PATCH",
            "/api/auth/profile",
            Some(&json!({
                "full_name": "Still A Member",
                "email": email,
                "role": "admin",
                "account_status": "active",
            })),
            Some(&jar),
            Some(&csrf),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["role"], "user");

    let (status, _headers, me) = send(
        &test.app,
        request("GET", "/api/auth/me", None, Some(&jar), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["role"], "user");
}

#[tokio::test]
async fn repeated_failed_logins_are_rate_limited() {
    let Some(test) = setup().await else {
        return;
    };
    let email = unique_email("ratelimit");
    let mut jar = Jar::default();

    for _ in 0..10 {
        let (status, _body) = login(&test.app, &email, "wrong-password-1", &mut jar).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    let (status, body) = login(&test.app, &email, "wrong-password-1", &mut jar).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
    assert_eq!(body["code"], "rate_limited");
}
