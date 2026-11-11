//! Account, session, and password-recovery HTTP handlers.
//!
//! Mounted under `/api/auth`. Every handler returns typed JSON and never
//! includes password hashes or raw session/reset tokens.

use crate::db::{
    self, AuthConfig, AuthUser, Preferences, PublicUser, RateLimiter, TokenTable, clear_cookies,
    client_address, csrf_cookie, normalize_email, session_cookie, session_cookies,
    validate_full_name, validate_password, verify_csrf,
};
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration as StdDuration;

const LOGIN_ATTEMPTS: u32 = 10;
const RECOVERY_ATTEMPTS: u32 = 5;
const VERIFY_ATTEMPTS: u32 = 10;
const RATE_WINDOW: StdDuration = StdDuration::from_secs(15 * 60);

/// Builds the authentication router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/status", get(status))
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
        .route("/api/auth/profile", patch(update_profile))
        .route("/api/auth/preferences", put(update_preferences))
        .route("/api/auth/forgot-password", post(forgot_password))
        .route("/api/auth/reset-password", post(reset_password))
        .route("/api/auth/verify-email", post(verify_email))
        .route("/api/auth/resend-verification", post(resend_verification))
        .route("/api/auth/change-password", post(change_password))
        .route("/api/auth/sessions", get(list_sessions))
        .route("/api/auth/sessions/{id}", delete(delete_session))
}

/// Status of the account system, used to render honest UI copy.
#[derive(Debug, Serialize)]
struct AuthStatus {
    accounts_enabled: bool,
    email_configured: bool,
    dev_mail: bool,
}

async fn status(State(state): State<AppState>) -> Json<AuthStatus> {
    let accounts = state.accounts();
    Json(AuthStatus {
        accounts_enabled: accounts.is_some(),
        email_configured: accounts
            .as_ref()
            .map(|accounts| accounts.mailer.configured())
            .unwrap_or(false),
        dev_mail: accounts
            .as_ref()
            .map(|accounts| accounts.config.dev_mail)
            .unwrap_or(false),
    })
}

#[derive(Debug, Deserialize)]
struct RegisterRequest {
    full_name: String,
    email: String,
    password: String,
    #[serde(default)]
    accept_terms: bool,
}

#[derive(Debug, Serialize)]
struct RegisterResponse {
    user: PublicUser,
    email_verification_required: bool,
    email_delivery: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    dev_verification_token: Option<String>,
}

async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterRequest>,
) -> Result<Response, ApiError> {
    let accounts = accounts(&state)?;

    if !request.accept_terms {
        return Err(ApiError::bad_request(
            "you must accept the Terms of Service and Privacy Policy to create an account",
        ));
    }

    let full_name = validate_full_name(&request.full_name)?;
    let email = normalize_email(&request.email)
        .ok_or_else(|| ApiError::bad_request("enter a valid email address"))?;
    validate_password(&request.password)?;

    let address = client_address(&headers);
    let limit_key = format!("register:{address}");
    if !accounts
        .limiter
        .check(&limit_key, LOGIN_ATTEMPTS, RATE_WINDOW)
    {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "too many attempts. Wait a few minutes and try again.",
        ));
    }

    let Some(user) = accounts
        .db
        .create_user(&full_name, &email, &request.password)
        .await?
    else {
        return Err(ApiError::conflict(
            "an account with this email address already exists",
        ));
    };

    let (delivery, dev_token) = issue_verification(&accounts, &user).await;
    let session = accounts
        .db
        .create_session(
            &user.id,
            false,
            user_agent(&headers).as_deref(),
            Some(&address),
            &accounts.config,
        )
        .await?;
    let cookies = session_cookies(&session, false, &accounts.config);

    Ok(with_cookies(
        StatusCode::CREATED,
        &cookies,
        Json(RegisterResponse {
            user,
            email_verification_required: true,
            email_delivery: delivery,
            dev_verification_token: dev_token,
        }),
    ))
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
    #[serde(default)]
    remember: bool,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    user: PublicUser,
    preferences: Preferences,
    expires_at: DateTime<Utc>,
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let accounts = accounts(&state)?;
    let email = normalize_email(&request.email)
        .ok_or_else(|| ApiError::bad_request("enter a valid email address"))?;
    let address = client_address(&headers);
    let limit_key = format!("login:{address}:{email}");

    if !accounts
        .limiter
        .check(&limit_key, LOGIN_ATTEMPTS, RATE_WINDOW)
    {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "too many sign-in attempts. Wait a few minutes and try again.",
        ));
    }

    let Some(user) = accounts
        .db
        .verify_credentials(&email, &request.password)
        .await?
    else {
        return Err(ApiError::unauthorized(
            "the email address or password is incorrect",
        ));
    };

    accounts.limiter.clear(&limit_key);

    let session = accounts
        .db
        .create_session(
            &user.id,
            request.remember,
            user_agent(&headers).as_deref(),
            Some(&address),
            &accounts.config,
        )
        .await?;
    let cookies = session_cookies(&session, request.remember, &accounts.config);
    let preferences = accounts.db.preferences(&user.id).await?;

    Ok(with_cookies(
        StatusCode::OK,
        &cookies,
        Json(LoginResponse {
            user,
            preferences,
            expires_at: session.expires_at,
        }),
    ))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ApiError> {
    let accounts = accounts(&state)?;

    // Logout is idempotent and safe to revoke without CSRF: a forged logout
    // request can only sign the victim out, which is harmless compared to
    // other CSRF targets. Always revoke the session when one exists so the
    // client and server state stay in sync.
    if let Some(user) = optional_user(&accounts, &headers).await? {
        accounts.db.revoke_session(&user.session_id).await?;
    }

    Ok(with_cookies(
        StatusCode::NO_CONTENT,
        &clear_cookies(&accounts.config),
        (),
    ))
}

#[derive(Debug, Serialize)]
struct MeResponse {
    user: PublicUser,
    preferences: Preferences,
    session_expires_at: DateTime<Utc>,
    csrf_token: Option<String>,
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, ApiError> {
    let accounts = accounts(&state)?;
    let user = require_user(&accounts, &headers).await?;
    let preferences = accounts.db.preferences(&user.user.id).await?;

    Ok(Json(MeResponse {
        user: user.user,
        preferences,
        session_expires_at: user.expires_at,
        csrf_token: csrf_cookie(&headers),
    }))
}

#[derive(Debug, Deserialize)]
struct UpdateProfileRequest {
    full_name: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct UpdateProfileResponse {
    user: PublicUser,
    email_verification_required: bool,
    email_delivery: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    dev_verification_token: Option<String>,
}

async fn update_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UpdateProfileRequest>,
) -> Result<Json<UpdateProfileResponse>, ApiError> {
    let accounts = accounts(&state)?;
    let user = require_user(&accounts, &headers).await?;
    require_csrf(&headers, &user)?;

    let full_name = validate_full_name(&request.full_name)?;
    let email = normalize_email(&request.email)
        .ok_or_else(|| ApiError::bad_request("enter a valid email address"))?;

    let Some(updated) = accounts
        .db
        .update_profile(&user.user.id, &full_name, &email)
        .await?
    else {
        return Err(ApiError::conflict(
            "an account with this email address already exists",
        ));
    };

    let (delivery, dev_token, verification_required) = if updated.email != user.user.email {
        let (delivery, token) = issue_verification(&accounts, &updated).await;
        (delivery, token, true)
    } else {
        (
            if accounts.mailer.configured() {
                "sent"
            } else {
                "not_configured"
            },
            None,
            false,
        )
    };

    Ok(Json(UpdateProfileResponse {
        user: updated,
        email_verification_required: verification_required,
        email_delivery: delivery,
        dev_verification_token: dev_token,
    }))
}

#[derive(Debug, Deserialize)]
struct PreferencesRequest {
    theme: String,
    #[serde(default)]
    terminal_preferences: Option<serde_json::Value>,
}

async fn update_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PreferencesRequest>,
) -> Result<Json<Preferences>, ApiError> {
    let accounts = accounts(&state)?;
    let user = require_user(&accounts, &headers).await?;
    require_csrf(&headers, &user)?;

    if !matches!(request.theme.as_str(), "dark" | "light" | "system") {
        return Err(ApiError::bad_request(
            "theme must be one of \"dark\", \"light\", or \"system\"",
        ));
    }

    accounts
        .db
        .save_preferences(
            &user.user.id,
            &request.theme,
            request.terminal_preferences.as_ref(),
        )
        .await?;

    Ok(Json(Preferences {
        theme: request.theme,
        terminal_preferences: request.terminal_preferences,
    }))
}

#[derive(Debug, Deserialize)]
struct ForgotPasswordRequest {
    email: String,
}

#[derive(Debug, Serialize)]
struct ForgotPasswordResponse {
    message: String,
    email_delivery: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    dev_reset_token: Option<String>,
}

async fn forgot_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ForgotPasswordRequest>,
) -> Result<Json<ForgotPasswordResponse>, ApiError> {
    let accounts = accounts(&state)?;
    let address = client_address(&headers);

    if !accounts
        .limiter
        .check(&format!("forgot:{address}"), RECOVERY_ATTEMPTS, RATE_WINDOW)
    {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "too many password recovery requests. Wait a few minutes and try again.",
        ));
    }

    let generic = "If an account exists for that address, a password reset link has been issued.";
    let Some(email) = normalize_email(&request.email) else {
        return Ok(Json(ForgotPasswordResponse {
            message: generic.to_owned(),
            email_delivery: "unknown",
            dev_reset_token: None,
        }));
    };

    let Some((user, _hash)) = accounts.db.find_user_by_email(&email).await? else {
        return Ok(Json(ForgotPasswordResponse {
            message: generic.to_owned(),
            email_delivery: "unknown",
            dev_reset_token: None,
        }));
    };

    let token = accounts
        .db
        .create_token(TokenTable::PasswordReset, &user.id, Duration::hours(1))
        .await?;

    let link = format!(
        "{}/reset-password?token={token}",
        accounts.config.public_url
    );
    let body = format!(
        "Hello {},\n\nUse this link to choose a new KERNEXT password. It expires in one hour and works once.\n\n{link}\n\nIf you did not request this, you can ignore this message.\n",
        user.full_name
    );

    if accounts.mailer.configured() {
        match accounts
            .mailer
            .send(&user.email, "Reset your KERNEXT password", body)
            .await
        {
            Ok(()) => {
                return Ok(Json(ForgotPasswordResponse {
                    message: generic.to_owned(),
                    email_delivery: "sent",
                    dev_reset_token: None,
                }));
            }
            Err(error) => {
                tracing::error!("KERNEXT password reset email failed: {error}");
                return Ok(Json(ForgotPasswordResponse {
                    message: generic.to_owned(),
                    email_delivery: "failed",
                    dev_reset_token: None,
                }));
            }
        }
    }

    // Development-only recovery path. It is gated twice: KERNEXT_ENV must not be
    // production, and KERNEXT_DEV_MAIL must be explicitly enabled.
    if accounts.config.dev_mail {
        tracing::warn!(
            "KERNEXT_DEV_MAIL is enabled; returning a development password reset token for {}",
            user.email
        );
        return Ok(Json(ForgotPasswordResponse {
            message:
                "Email delivery is not configured. This server returned a development-only reset token."
                    .to_owned(),
            email_delivery: "development",
            dev_reset_token: Some(token),
        }));
    }

    Ok(Json(ForgotPasswordResponse {
        message: "Password reset email delivery is not configured on this server.".to_owned(),
        email_delivery: "not_configured",
        dev_reset_token: None,
    }))
}

#[derive(Debug, Deserialize)]
struct ResetPasswordRequest {
    token: String,
    password: String,
}

async fn reset_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ResetPasswordRequest>,
) -> Result<Response, ApiError> {
    let accounts = accounts(&state)?;
    let address = client_address(&headers);

    if !accounts
        .limiter
        .check(&format!("reset:{address}"), RECOVERY_ATTEMPTS, RATE_WINDOW)
    {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "too many password reset attempts. Wait a few minutes and try again.",
        ));
    }

    validate_password(&request.password)?;

    let Some((token_id, user_id)) = accounts
        .db
        .resolve_token(TokenTable::PasswordReset, &request.token)
        .await?
    else {
        return Err(ApiError::bad_request(
            "this password reset link is invalid or has expired",
        ));
    };

    if !accounts
        .db
        .consume_token(TokenTable::PasswordReset, &token_id)
        .await?
    {
        return Err(ApiError::bad_request(
            "this password reset link has already been used",
        ));
    }

    let hash = db::hash_password(&request.password)?;
    accounts.db.change_password(&user_id, &hash, None).await?;

    Ok(with_cookies(
        StatusCode::OK,
        &clear_cookies(&accounts.config),
        Json(serde_json::json!({
            "message": "Your password has been changed. Sign in with the new password."
        })),
    ))
}

#[derive(Debug, Deserialize)]
struct TokenRequest {
    token: String,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
    message: String,
}

async fn verify_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<TokenRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    let accounts = accounts(&state)?;
    let address = client_address(&headers);

    if !accounts
        .limiter
        .check(&format!("verify:{address}"), VERIFY_ATTEMPTS, RATE_WINDOW)
    {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "too many verification attempts. Wait a few minutes and try again.",
        ));
    }

    let Some((token_id, user_id)) = accounts
        .db
        .resolve_token(TokenTable::EmailVerification, &request.token)
        .await?
    else {
        return Err(ApiError::bad_request(
            "this verification link is invalid or has expired",
        ));
    };

    accounts
        .db
        .consume_token(TokenTable::EmailVerification, &token_id)
        .await?;
    accounts.db.mark_email_verified(&user_id).await?;

    Ok(Json(MessageResponse {
        message: "Your email address is verified.".to_owned(),
    }))
}

#[derive(Debug, Serialize)]
struct ResendVerificationResponse {
    email_delivery: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    dev_verification_token: Option<String>,
}

async fn resend_verification(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ResendVerificationResponse>, ApiError> {
    let accounts = accounts(&state)?;
    let user = require_user(&accounts, &headers).await?;
    require_csrf(&headers, &user)?;

    if user.user.email_verified {
        return Ok(Json(ResendVerificationResponse {
            email_delivery: "already_verified",
            dev_verification_token: None,
        }));
    }

    let (delivery, dev_token) = issue_verification(&accounts, &user.user).await;
    Ok(Json(ResendVerificationResponse {
        email_delivery: delivery,
        dev_verification_token: dev_token,
    }))
}

#[derive(Debug, Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
    #[serde(default)]
    sign_out_other_sessions: bool,
}

async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    let accounts = accounts(&state)?;
    let user = require_user(&accounts, &headers).await?;
    require_csrf(&headers, &user)?;
    validate_password(&request.new_password)?;

    let Some(hash) = accounts.db.password_hash(&user.user.id).await? else {
        return Err(ApiError::unauthorized("the account no longer exists"));
    };

    if !db::verify_password(&request.current_password, &hash) {
        return Err(ApiError::bad_request("the current password is incorrect"));
    }

    let new_hash = db::hash_password(&request.new_password)?;
    let keep = (!request.sign_out_other_sessions).then_some(user.session_id.as_str());
    accounts
        .db
        .change_password(&user.user.id, &new_hash, keep)
        .await?;

    Ok(Json(MessageResponse {
        message: "Your password has been changed.".to_owned(),
    }))
}

#[derive(Debug, Serialize)]
struct SessionsResponse {
    sessions: Vec<db::SessionSummary>,
}

async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SessionsResponse>, ApiError> {
    let accounts = accounts(&state)?;
    let user = require_user(&accounts, &headers).await?;
    let sessions = accounts
        .db
        .list_sessions(&user.user.id, &user.session_id)
        .await?;

    Ok(Json(SessionsResponse { sessions }))
}

async fn delete_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    let accounts = accounts(&state)?;
    let user = require_user(&accounts, &headers).await?;
    require_csrf(&headers, &user)?;

    if !accounts.db.revoke_owned_session(&user.user.id, &id).await? {
        return Err(ApiError::not_found("that session does not exist"));
    }

    if id == user.session_id {
        return Ok(StatusCode::NO_CONTENT);
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn issue_verification(
    accounts: &crate::state::AuthContext,
    user: &PublicUser,
) -> (&'static str, Option<String>) {
    let token = match accounts
        .db
        .create_token(TokenTable::EmailVerification, &user.id, Duration::hours(24))
        .await
    {
        Ok(token) => token,
        Err(error) => {
            tracing::error!("could not create a verification token: {error}");
            return ("failed", None);
        }
    };

    let link = format!("{}/verify-email?token={token}", accounts.config.public_url);
    let body = format!(
        "Hello {},\n\nConfirm your email address to finish setting up your KERNEXT account.\n\n{link}\n\nThis link expires in 24 hours.\n",
        user.full_name
    );

    if accounts.mailer.configured() {
        return match accounts
            .mailer
            .send(&user.email, "Verify your KERNEXT email address", body)
            .await
        {
            Ok(()) => ("sent", None),
            Err(error) => {
                tracing::error!("KERNEXT verification email failed: {error}");
                ("failed", None)
            }
        };
    }

    if accounts.config.dev_mail {
        tracing::warn!(
            "KERNEXT_DEV_MAIL is enabled; returning a development verification token for {}",
            user.email
        );
        return ("development", Some(token));
    }

    ("not_configured", None)
}

/// Resolves a signed-in account from request cookies for terminal endpoints.
pub(crate) async fn require_account(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthUser, ApiError> {
    let accounts = accounts(state)?;
    require_user(&accounts, headers).await
}

fn accounts(state: &AppState) -> Result<Arc<crate::state::AuthContext>, ApiError> {
    state.accounts().ok_or_else(|| {
        ApiError::unavailable(
            "the KERNEXT account database is not configured. Set DATABASE_URL to enable accounts.",
        )
    })
}

async fn optional_user(
    accounts: &crate::state::AuthContext,
    headers: &HeaderMap,
) -> Result<Option<AuthUser>, ApiError> {
    let Some(token) = session_cookie(headers) else {
        return Ok(None);
    };

    accounts.db.resolve_session(&token).await
}

async fn require_user(
    accounts: &crate::state::AuthContext,
    headers: &HeaderMap,
) -> Result<AuthUser, ApiError> {
    optional_user(accounts, headers)
        .await?
        .ok_or_else(|| ApiError::unauthorized("sign in to access this resource"))
}

fn require_csrf(headers: &HeaderMap, user: &AuthUser) -> Result<(), ApiError> {
    if verify_csrf(headers, user) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "csrf_failed",
            "the CSRF token is missing or does not match this session",
        ))
    }
}

fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

/// Attaches one or more `Set-Cookie` headers to a response.
fn with_cookies<B>(status: StatusCode, cookies: &[String], body: B) -> Response
where
    B: IntoResponse,
{
    let mut response = (status, body).into_response();

    for cookie in cookies {
        if let Ok(value) = HeaderValue::from_str(cookie) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }

    response
}

/// Shared limiter type alias used by [`crate::state::AuthContext`].
pub type SharedRateLimiter = Arc<RateLimiter>;

/// Shared auth configuration alias.
pub type SharedAuthConfig = Arc<AuthConfig>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_email_is_left_unchanged() {
        assert!(normalize_email("").is_none());
        assert!(normalize_email("not-an-email").is_none());
        assert!(normalize_email("user@example").is_none());
        assert_eq!(
            normalize_email(" User@Example.COM ").as_deref(),
            Some("user@example.com")
        );
    }

    #[test]
    fn passwords_require_length_and_complexity() {
        assert!(validate_password("short1").is_err());
        assert!(validate_password("alllettersonly").is_err());
        assert!(validate_password("1234567890").is_err());
        assert!(validate_password("kernext2026secure").is_ok());
    }

    #[test]
    fn tokens_are_hashed_and_unique() {
        let first = db::generate_token();
        let second = db::generate_token();
        assert_ne!(first, second);
        assert_eq!(first.len(), 64);
        assert_eq!(db::hash_token(&first).len(), 64);
        assert_ne!(db::hash_token(&first), first);
    }

    #[test]
    fn rate_limiter_blocks_after_the_limit() {
        let limiter = RateLimiter::default();
        assert!(limiter.check("k", 2, RATE_WINDOW));
        assert!(limiter.check("k", 2, RATE_WINDOW));
        assert!(!limiter.check("k", 2, RATE_WINDOW));
        limiter.clear("k");
        assert!(limiter.check("k", 2, RATE_WINDOW));
    }
}
