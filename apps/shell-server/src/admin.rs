//! KERNEXT administration API.
//!
//! Every handler resolves the signed-in account from the existing session
//! cookie and rejects non-administrators on the server. The frontend guard is
//! convenience only; these checks are the authorization boundary.
//!
//! The console never exposes password hashes, session tokens, CSRF hashes,
//! command input, or terminal output. Audit rows persist the acting account,
//! the target account, a safe event type, and optional non-sensitive metadata.

use crate::db::{
    self, AdminSessionSummary, AdminStats, AdminUserSummary, AuditEntry, AuditPage, AuditQuery,
    AuditRecord, AuthUser, UserPage,
};
use crate::error::ApiError;
use crate::state::{AdminTerminalSession, AppState};
use crate::{auth_routes, ws};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Instant;

const DEFAULT_PAGE_SIZE: u32 = 25;
const MAX_PAGE_SIZE: u32 = 100;
const MAX_SEARCH_LENGTH: usize = 120;

/// Builds the administration router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/admin/overview", get(overview))
        .route("/api/admin/users", get(users))
        .route("/api/admin/users/{id}", get(user_detail).patch(update_user))
        .route("/api/admin/users/{id}/status", post(set_status))
        .route("/api/admin/users/{id}/role", post(set_role))
        .route(
            "/api/admin/users/{id}/sessions",
            get(user_sessions).delete(revoke_sessions),
        )
        .route("/api/admin/terminal-sessions", get(terminal_sessions))
        .route(
            "/api/admin/terminal-sessions/{id}",
            delete(terminate_terminal),
        )
        .route("/api/admin/health", get(health))
        .route("/api/admin/audit-logs", get(audit_logs))
}

// ---------------------------------------------------------------------------
// Process start time
// ---------------------------------------------------------------------------

static STARTED_AT: OnceLock<DateTime<Utc>> = OnceLock::new();
static STARTED_INSTANT: OnceLock<Instant> = OnceLock::new();

/// Marks the start of this server process and records a real startup event.
///
/// Called once from [`crate::run`] after the account database is ready. It is
/// also safe in tests, where it only initializes the timestamps.
pub async fn record_start(state: &AppState) {
    STARTED_AT.get_or_init(Utc::now);
    STARTED_INSTANT.get_or_init(Instant::now);

    let Some(accounts) = state.accounts() else {
        return;
    };

    write_audit(
        &accounts,
        AuditRecord::new("system.startup").metadata(serde_json::json!({
            "version": shell_core::VERSION,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        })),
    )
    .await;
}

fn uptime_seconds() -> u64 {
    STARTED_INSTANT
        .get()
        .map(|started| started.elapsed().as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Authorization
// ---------------------------------------------------------------------------

/// Resolves an active administrator or fails with the correct status.
async fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<AuthUser, ApiError> {
    let user = auth_routes::require_account_active(state, headers).await?;

    if user.user.role != "admin" {
        return Err(ApiError::forbidden("administrator access is required"));
    }

    Ok(user)
}

/// Resolves an administrator and enforces the CSRF double-submit token for
/// state-changing requests.
async fn require_admin_csrf(state: &AppState, headers: &HeaderMap) -> Result<AuthUser, ApiError> {
    let user = require_admin(state, headers).await?;

    if !db::verify_csrf(headers, &user) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "csrf_failed",
            "the CSRF token is missing or does not match this session",
        ));
    }

    Ok(user)
}

fn accounts(state: &AppState) -> Result<std::sync::Arc<crate::state::AuthContext>, ApiError> {
    state.accounts().ok_or_else(|| {
        ApiError::unavailable(
            "the KERNEXT account database is not configured. Set DATABASE_URL to enable accounts.",
        )
    })
}

/// Audits an event, logging (not failing the request) when persistence fails.
async fn write_audit(accounts: &crate::state::AuthContext, record: AuditRecord) {
    let event_type = record.event_type.clone();
    if let Err(error) = accounts.db.record_audit(&record).await {
        tracing::warn!("KERNEXT audit write failed for {event_type}: {error}");
    }
}

// ---------------------------------------------------------------------------
// Pagination helpers
// ---------------------------------------------------------------------------

fn page_window(page: Option<u32>, per_page: Option<u32>) -> (u32, u32) {
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    (page, per_page)
}

fn validate_search(search: Option<String>) -> Result<Option<String>, ApiError> {
    let Some(search) = search else {
        return Ok(None);
    };
    let search = search.trim().to_owned();

    if search.is_empty() {
        return Ok(None);
    }

    if search.chars().count() > MAX_SEARCH_LENGTH {
        return Err(ApiError::bad_request(format!(
            "the search term must be at most {MAX_SEARCH_LENGTH} characters"
        )));
    }

    Ok(Some(search))
}

// ---------------------------------------------------------------------------
// Overview
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct Totals {
    total_users: u64,
    active_users: u64,
    administrators: u64,
    active_sessions: u64,
    active_terminal_sessions: u64,
}

#[derive(Debug, Serialize)]
struct ServiceSummary {
    id: &'static str,
    label: &'static str,
    status: ServiceStatus,
}

#[derive(Debug, Serialize)]
struct OverviewResponse {
    totals: Totals,
    services: Vec<ServiceSummary>,
    latest_registrations: Vec<AdminUserSummary>,
    recent_security_events: Vec<AuditEntry>,
    recent_system_events: Vec<AuditEntry>,
    server: ServerInfo,
    generated_at: DateTime<Utc>,
}

async fn overview(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OverviewResponse>, ApiError> {
    require_admin(&state, &headers).await?;
    let accounts = accounts(&state)?;

    let stats: AdminStats = accounts.db.statistics().await?;
    let latest = accounts.db.latest_registrations(5).await?;
    let security = accounts
        .db
        .audit_logs(&AuditQuery {
            categories: Some(vec!["auth".to_owned(), "password".to_owned()]),
            page: 1,
            per_page: 8,
            ..AuditQuery::default()
        })
        .await?;
    let system = accounts
        .db
        .audit_logs(&AuditQuery {
            categories: Some(vec![
                "user".to_owned(),
                "session".to_owned(),
                "system".to_owned(),
            ]),
            page: 1,
            per_page: 8,
            ..AuditQuery::default()
        })
        .await?;

    let report = build_health(&state, &accounts).await;
    let services = report
        .checks
        .iter()
        .map(|check| ServiceSummary {
            id: check.id,
            label: check.label,
            status: check.status,
        })
        .collect();

    Ok(Json(OverviewResponse {
        totals: Totals {
            total_users: stats.total_users,
            active_users: stats.active_users,
            administrators: stats.administrators,
            active_sessions: stats.active_sessions,
            active_terminal_sessions: state.admin_terminal_sessions().len() as u64,
        },
        services,
        latest_registrations: latest,
        recent_security_events: security.events,
        recent_system_events: system.events,
        server: server_info(),
        generated_at: Utc::now(),
    }))
}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct UsersQuery {
    search: Option<String>,
    role: Option<String>,
    status: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<UsersQuery>,
) -> Result<Json<UserPage>, ApiError> {
    require_admin(&state, &headers).await?;
    let accounts = accounts(&state)?;
    let (page, per_page) = page_window(query.page, query.per_page);
    let role = match query.role.as_deref() {
        None | Some("") | Some("all") => None,
        Some("user" | "admin") => query.role.clone(),
        Some(_) => {
            return Err(ApiError::bad_request(
                "role must be \"user\", \"admin\", or empty",
            ));
        }
    };
    let status = match query.status.as_deref() {
        None | Some("") | Some("all") => None,
        Some("active" | "disabled") => query.status.clone(),
        Some(_) => {
            return Err(ApiError::bad_request(
                "status must be \"active\", \"disabled\", or empty",
            ));
        }
    };

    let page = accounts
        .db
        .admin_users(&db::UserQuery {
            search: validate_search(query.search)?,
            role,
            status,
            page,
            per_page,
        })
        .await?;

    Ok(Json(page))
}

#[derive(Debug, Serialize)]
struct UserDetailResponse {
    user: AdminUserSummary,
    sessions: Vec<AdminSessionSummary>,
}

async fn user_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<UserDetailResponse>, ApiError> {
    require_admin(&state, &headers).await?;
    let accounts = accounts(&state)?;
    let user = load_user(&accounts, &id).await?;
    let sessions = accounts.db.list_admin_sessions(&id).await?;

    Ok(Json(UserDetailResponse { user, sessions }))
}

#[derive(Debug, Deserialize)]
struct UpdateUserRequest {
    full_name: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct UserResponse {
    user: AdminUserSummary,
}

async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let actor = require_admin_csrf(&state, &headers).await?;
    let accounts = accounts(&state)?;
    let target = load_user(&accounts, &id).await?;

    let full_name = db::validate_full_name(&request.full_name)?;
    let email = db::normalize_email(&request.email)
        .ok_or_else(|| ApiError::bad_request("enter a valid email address"))?;
    let email_changed = email != target.email;

    let Some(updated) = accounts
        .db
        .update_admin_user(&id, &full_name, &email)
        .await?
    else {
        return Err(ApiError::conflict(
            "an account with this email address already exists",
        ));
    };

    write_audit(
        &accounts,
        AuditRecord::new("user.updated")
            .actor(&actor.user.id, &actor.user.email)
            .target(&target.id, &target.email)
            .metadata(serde_json::json!({
                "email_changed": email_changed,
                "name_changed": updated.full_name != target.full_name,
            }))
            .ip(db::client_address(&headers)),
    )
    .await;

    Ok(Json(UserResponse { user: updated }))
}

#[derive(Debug, Deserialize)]
struct StatusRequest {
    status: String,
}

async fn set_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<StatusRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let actor = require_admin_csrf(&state, &headers).await?;
    let accounts = accounts(&state)?;

    if !matches!(request.status.as_str(), "active" | "disabled") {
        return Err(ApiError::bad_request(
            "status must be \"active\" or \"disabled\"",
        ));
    }

    let target = load_user(&accounts, &id).await?;

    if target.id == actor.user.id {
        return Err(ApiError::conflict(
            "you cannot change your own account status from the administration console",
        ));
    }

    if target.account_status == request.status {
        return Ok(Json(UserResponse { user: target }));
    }

    if request.status == "disabled"
        && target.role == "admin"
        && target.account_status == "active"
        && accounts.db.count_active_admins().await? <= 1
    {
        return Err(ApiError::conflict(
            "the last active administrator cannot be deactivated",
        ));
    }

    if !accounts.db.set_user_status(&id, &request.status).await? {
        return Err(ApiError::not_found("that account no longer exists"));
    }

    let mut revoked = 0;
    if request.status == "disabled" {
        revoked = accounts.db.revoke_all_user_sessions(&id).await?;
        // A disabled account must not keep a terminal open either.
        for session in state
            .admin_terminal_sessions()
            .into_iter()
            .filter(|session| session.owner.as_deref() == Some(id.as_str()))
        {
            let _ = state.terminate_terminal_session(&session.id);
        }
    }

    let updated = load_user(&accounts, &id).await?;

    write_audit(
        &accounts,
        AuditRecord::new("user.status_changed")
            .actor(&actor.user.id, &actor.user.email)
            .target(&target.id, &target.email)
            .metadata(serde_json::json!({
                "from": target.account_status,
                "to": request.status,
                "sessions_revoked": revoked,
            }))
            .ip(db::client_address(&headers)),
    )
    .await;

    Ok(Json(UserResponse { user: updated }))
}

#[derive(Debug, Deserialize)]
struct RoleRequest {
    role: String,
}

async fn set_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<RoleRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let actor = require_admin_csrf(&state, &headers).await?;
    let accounts = accounts(&state)?;

    if !matches!(request.role.as_str(), "user" | "admin") {
        return Err(ApiError::bad_request("role must be \"user\" or \"admin\""));
    }

    let target = load_user(&accounts, &id).await?;

    if target.id == actor.user.id {
        return Err(ApiError::conflict(
            "you cannot change your own role from the administration console",
        ));
    }

    if target.role == request.role {
        return Ok(Json(UserResponse { user: target }));
    }

    if target.role == "admin"
        && target.account_status == "active"
        && accounts.db.count_active_admins().await? <= 1
    {
        return Err(ApiError::conflict(
            "the last active administrator cannot be demoted",
        ));
    }

    if !accounts.db.set_user_role(&id, &request.role).await? {
        return Err(ApiError::not_found("that account no longer exists"));
    }

    let updated = load_user(&accounts, &id).await?;

    write_audit(
        &accounts,
        AuditRecord::new("user.role_changed")
            .actor(&actor.user.id, &actor.user.email)
            .target(&target.id, &target.email)
            .metadata(serde_json::json!({ "from": target.role, "to": request.role }))
            .ip(db::client_address(&headers)),
    )
    .await;

    Ok(Json(UserResponse { user: updated }))
}

#[derive(Debug, Serialize)]
struct UserSessionsResponse {
    sessions: Vec<AdminSessionSummary>,
}

async fn user_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<UserSessionsResponse>, ApiError> {
    require_admin(&state, &headers).await?;
    let accounts = accounts(&state)?;
    load_user(&accounts, &id).await?;

    Ok(Json(UserSessionsResponse {
        sessions: accounts.db.list_admin_sessions(&id).await?,
    }))
}

#[derive(Debug, Serialize)]
struct RevokeSessionsResponse {
    revoked: u64,
}

async fn revoke_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<RevokeSessionsResponse>, ApiError> {
    let actor = require_admin_csrf(&state, &headers).await?;
    let accounts = accounts(&state)?;
    let target = load_user(&accounts, &id).await?;

    let revoked = accounts.db.revoke_all_user_sessions(&target.id).await?;

    write_audit(
        &accounts,
        AuditRecord::new("user.sessions_revoked")
            .actor(&actor.user.id, &actor.user.email)
            .target(&target.id, &target.email)
            .metadata(serde_json::json!({ "revoked": revoked }))
            .ip(db::client_address(&headers)),
    )
    .await;

    Ok(Json(RevokeSessionsResponse { revoked }))
}

async fn load_user(
    accounts: &crate::state::AuthContext,
    id: &str,
) -> Result<AdminUserSummary, ApiError> {
    accounts
        .db
        .admin_user(id)
        .await?
        .ok_or_else(|| ApiError::not_found("that account does not exist"))
}

// ---------------------------------------------------------------------------
// Terminal sessions
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct TerminalSessionView {
    #[serde(flatten)]
    session: AdminTerminalSession,
    owner_name: Option<String>,
    owner_email: Option<String>,
}

#[derive(Debug, Serialize)]
struct TerminalSessionsResponse {
    sessions: Vec<TerminalSessionView>,
}

async fn terminal_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<TerminalSessionsResponse>, ApiError> {
    require_admin(&state, &headers).await?;
    let accounts = accounts(&state)?;
    let sessions = state.admin_terminal_sessions();
    let owners = sessions
        .iter()
        .filter_map(|session| session.owner.clone())
        .collect::<Vec<_>>();
    let labels = accounts.db.user_labels(&owners).await?;

    let sessions = sessions
        .into_iter()
        .map(|session| {
            let (owner_name, owner_email) = session
                .owner
                .as_ref()
                .and_then(|owner| labels.get(owner))
                .map(|(name, email)| (Some(name.clone()), Some(email.clone())))
                .unwrap_or((None, None));

            TerminalSessionView {
                session,
                owner_name,
                owner_email,
            }
        })
        .collect();

    Ok(Json(TerminalSessionsResponse { sessions }))
}

async fn terminate_terminal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let actor = require_admin_csrf(&state, &headers).await?;
    let accounts = accounts(&state)?;

    let target = state
        .admin_terminal_sessions()
        .into_iter()
        .find(|session| session.id == id);

    if !state.terminate_terminal_session(&id)? {
        return Err(ApiError::not_found(
            "that terminal session is no longer active",
        ));
    }

    let (mode, owner) = target
        .as_ref()
        .map(|session| {
            (
                session.mode.clone(),
                session.owner.clone().unwrap_or_else(|| "local".to_owned()),
            )
        })
        .unwrap_or_else(|| ("unknown".to_owned(), "unknown".to_owned()));

    write_audit(
        &accounts,
        AuditRecord::new("terminal.session_terminated")
            .actor(&actor.user.id, &actor.user.email)
            .target(&owner, format!("terminal {id}"))
            .metadata(serde_json::json!({ "mode": mode, "terminal_id": id }))
            .ip(db::client_address(&headers)),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unavailable,
    Unknown,
}

impl ServiceStatus {
    const fn rank(self) -> u8 {
        match self {
            Self::Healthy => 0,
            Self::Unknown => 1,
            Self::Degraded => 2,
            Self::Unavailable => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthCheck {
    pub id: &'static str,
    pub label: &'static str,
    pub status: ServiceStatus,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerInfo {
    pub product: &'static str,
    pub version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatabaseInfo {
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    pub migrations_applied: Option<i64>,
    pub migrations_expected: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrations_latest: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: ServiceStatus,
    pub checks: Vec<HealthCheck>,
    pub server: ServerInfo,
    pub database: DatabaseInfo,
    pub generated_at: DateTime<Utc>,
}

fn server_info() -> ServerInfo {
    ServerInfo {
        product: shell_core::PRODUCT_NAME,
        version: shell_core::VERSION,
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        uptime_seconds: uptime_seconds(),
        started_at: STARTED_AT.get().copied(),
    }
}

fn expected_migrations() -> i64 {
    i64::try_from(sqlx::migrate!("./migrations").iter().count()).unwrap_or(0)
}

async fn build_health(state: &AppState, accounts: &crate::state::AuthContext) -> HealthResponse {
    let mut checks = vec![HealthCheck {
        id: "api",
        label: "HTTP API",
        status: ServiceStatus::Healthy,
        detail: "The API answered this health request.".to_owned(),
        latency_ms: None,
    }];

    let expected = expected_migrations();
    let mut database = DatabaseInfo {
        configured: true,
        latency_ms: None,
        migrations_applied: None,
        migrations_expected: expected,
        migrations_latest: None,
    };

    let started = Instant::now();
    match sqlx::query("SELECT 1").execute(accounts.db.pool()).await {
        Ok(_) => {
            let latency = started.elapsed().as_millis() as u64;
            database.latency_ms = Some(latency);
            let status = if latency > 300 {
                ServiceStatus::Degraded
            } else {
                ServiceStatus::Healthy
            };
            checks.push(HealthCheck {
                id: "database",
                label: "MariaDB",
                status,
                detail: if status == ServiceStatus::Degraded {
                    format!("Database reachable, but slow ({latency} ms).")
                } else {
                    format!("Database reachable in {latency} ms.")
                },
                latency_ms: Some(latency),
            });
        }
        Err(error) => {
            checks.push(HealthCheck {
                id: "database",
                label: "MariaDB",
                status: ServiceStatus::Unavailable,
                detail: format!("Database query failed: {error}"),
                latency_ms: None,
            });
        }
    }

    match accounts.db.migration_status().await {
        Ok((applied, latest)) => {
            database.migrations_applied = Some(applied);
            database.migrations_latest = latest;
            let status = if applied >= expected {
                ServiceStatus::Healthy
            } else {
                ServiceStatus::Degraded
            };
            checks.push(HealthCheck {
                id: "migrations",
                label: "Database migrations",
                status,
                detail: format!("{applied} of {expected} embedded migrations applied."),
                latency_ms: None,
            });
        }
        Err(error) => {
            checks.push(HealthCheck {
                id: "migrations",
                label: "Database migrations",
                status: ServiceStatus::Degraded,
                detail: format!("Migration status could not be read: {error}"),
                latency_ms: None,
            });
        }
    }

    match accounts.db.verify_schema().await {
        Ok(()) => checks.push(HealthCheck {
            id: "schema",
            label: "Account schema",
            status: ServiceStatus::Healthy,
            detail: "Every required account and audit table is present.".to_owned(),
            latency_ms: None,
        }),
        Err(error) => checks.push(HealthCheck {
            id: "schema",
            label: "Account schema",
            status: ServiceStatus::Unavailable,
            detail: format!("{error}"),
            latency_ms: None,
        }),
    }

    checks.push(HealthCheck {
        id: "websocket",
        label: "WebSocket service",
        status: ServiceStatus::Healthy,
        detail: format!(
            "{} live terminal WebSocket connection(s).",
            state.ws_connections()
        ),
        latency_ms: None,
    });

    checks.push(HealthCheck {
        id: "terminal-engine",
        label: "Terminal engine",
        status: ServiceStatus::Healthy,
        detail: "The native KERNEXT engine is available for engine sessions.".to_owned(),
        latency_ms: None,
    });

    checks.push(match ws::pty_binary() {
        Some(path) => HealthCheck {
            id: "terminal-pty",
            label: "Web terminal (PTY)",
            status: ServiceStatus::Healthy,
            detail: format!("PTY executable resolved at {}.", path.display()),
            latency_ms: None,
        },
        None => HealthCheck {
            id: "terminal-pty",
            label: "Web terminal (PTY)",
            status: ServiceStatus::Degraded,
            detail: "The kernext executable was not found; PTY sessions are unavailable."
                .to_owned(),
            latency_ms: None,
        },
    });

    let status = checks
        .iter()
        .map(|check| check.status)
        .max_by_key(|status| status.rank())
        .unwrap_or(ServiceStatus::Unknown);

    HealthResponse {
        status,
        checks,
        server: server_info(),
        database,
        generated_at: Utc::now(),
    }
}

async fn health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HealthResponse>, ApiError> {
    require_admin(&state, &headers).await?;
    let accounts = accounts(&state)?;
    Ok(Json(build_health(&state, &accounts).await))
}

// ---------------------------------------------------------------------------
// Audit log
// ---------------------------------------------------------------------------

const AUDIT_CATEGORIES: &[&str] = &["auth", "password", "user", "session", "system"];

#[derive(Debug, Deserialize)]
struct AuditQueryParams {
    category: Option<String>,
    event_type: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn audit_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditQueryParams>,
) -> Result<Json<AuditPage>, ApiError> {
    require_admin(&state, &headers).await?;
    let accounts = accounts(&state)?;
    let (page, per_page) = page_window(query.page, query.per_page);

    let category = match query.category.as_deref() {
        None | Some("") | Some("all") => None,
        Some(value) if AUDIT_CATEGORIES.contains(&value) => Some(value.to_owned()),
        Some(_) => {
            return Err(ApiError::bad_request(
                "category must be one of auth, password, user, session, or system",
            ));
        }
    };

    let event_type = query.event_type.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    });
    if let Some(event_type) = &event_type {
        let valid = event_type.len() <= 64
            && event_type
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_');
        if !valid {
            return Err(ApiError::bad_request(
                "event_type must be lowercase letters and underscores",
            ));
        }
    }

    Ok(Json(
        accounts
            .db
            .audit_logs(&AuditQuery {
                category,
                event_type,
                actor_id: None,
                target_user_id: None,
                categories: None,
                page,
                per_page,
            })
            .await?,
    ))
}
