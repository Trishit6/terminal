//! MariaDB-backed accounts, sessions, and token storage.
//!
//! Passwords are hashed with Argon2id. Session, password-reset, and email
//! verification tokens are generated with the operating system's cryptographic
//! random number generator; only their SHA-256 hashes are persisted, so a
//! database disclosure does not hand out usable credentials.

use crate::error::ApiError;
use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng as PasswordOsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum::http::HeaderMap;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::{FromRow, Row};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration as StdDuration, Instant};
use uuid::Uuid;

/// Minimum accepted password length.
pub const MIN_PASSWORD_LENGTH: usize = 10;
/// Maximum accepted password length (Argon2 and the API both stay far below
/// this; the cap bounds attacker-controlled work).
pub const MAX_PASSWORD_LENGTH: usize = 128;

const SESSION_COOKIE: &str = "kernext_session";
const CSRF_COOKIE: &str = "kernext_csrf";

/// Runtime configuration for the account system.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Adds the `Secure` attribute to cookies. Required outside localhost.
    pub secure_cookies: bool,
    /// Enables the development-only token echo. Never enable in production.
    pub dev_mail: bool,
    /// Public application URL used in emailed links.
    pub public_url: String,
    /// Lifetime of a session created without "remember me".
    pub session_hours: i64,
    /// Lifetime of a "remember me" session.
    pub remember_days: i64,
}

impl AuthConfig {
    /// Reads configuration from the process environment.
    #[must_use]
    pub fn from_env() -> Self {
        let production = std::env::var("KERNEXT_ENV")
            .map(|value| value.eq_ignore_ascii_case("production"))
            .unwrap_or(false);

        let secure_cookies = std::env::var("KERNEXT_SECURE_COOKIES")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(production);

        let dev_mail = !production
            && std::env::var("KERNEXT_DEV_MAIL")
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

        let public_url = std::env::var("APP_PUBLIC_URL")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:7878".to_owned());

        Self {
            secure_cookies,
            dev_mail,
            public_url,
            session_hours: 12,
            remember_days: 30,
        }
    }

    /// Builds a configuration suitable for tests.
    #[must_use]
    pub fn local() -> Self {
        Self {
            secure_cookies: false,
            dev_mail: false,
            public_url: "http://127.0.0.1:7878".to_owned(),
            session_hours: 12,
            remember_days: 30,
        }
    }
}

/// Connection pool plus migration runner.
#[derive(Clone)]
pub struct Database {
    pool: MySqlPool,
}

impl Database {
    /// Opens a pooled connection to MariaDB using a `mysql://` URL.
    pub async fn connect(url: &str) -> Result<Self, ApiError> {
        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .acquire_timeout(StdDuration::from_secs(5))
            .connect(url)
            .await
            .map_err(|error| {
                ApiError::unavailable(format!(
                    "the KERNEXT account database is not reachable: {error}"
                ))
            })?;

        Ok(Self { pool })
    }

    /// Applies the embedded, versioned migrations.
    pub async fn migrate(&self) -> Result<(), ApiError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|error| ApiError::internal(format!("database migration failed: {error}")))
    }

    /// Borrows the pool for route handlers and tests.
    #[must_use]
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }
}

/// Public account data. Never contains the password hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicUser {
    pub id: String,
    pub full_name: String,
    pub email: String,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub account_status: String,
}

/// Account preferences returned with the profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub theme: String,
    pub terminal_preferences: Option<serde_json::Value>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: "dark".to_owned(),
            terminal_preferences: None,
        }
    }
}

/// One active session row exposed to its owner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub current: bool,
}

/// The authenticated principal resolved from a session cookie.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user: PublicUser,
    pub session_id: String,
    pub csrf_token_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct UserRow {
    id: String,
    full_name: String,
    email: String,
    password_hash: String,
    email_verified_at: Option<NaiveDateTime>,
    created_at: NaiveDateTime,
    account_status: String,
}

impl UserRow {
    fn public(&self) -> PublicUser {
        PublicUser {
            id: self.id.clone(),
            full_name: self.full_name.clone(),
            email: self.email.clone(),
            email_verified: self.email_verified_at.is_some(),
            created_at: self.created_at.and_utc(),
            account_status: self.account_status.clone(),
        }
    }
}

#[derive(FromRow)]
struct SessionRow {
    session_id: String,
    csrf_token_hash: String,
    expires_at: NaiveDateTime,
    id: String,
    full_name: String,
    email: String,
    email_verified_at: Option<NaiveDateTime>,
    created_at: NaiveDateTime,
    account_status: String,
}

/// Validates an email address without adding a regex dependency.
#[must_use]
pub fn normalize_email(email: &str) -> Option<String> {
    let email = email.trim().to_ascii_lowercase();

    if email.len() > 320 || email.len() < 5 {
        return None;
    }

    let (local, domain) = email.split_once('@')?;

    if local.is_empty() || domain.is_empty() || domain.contains('@') {
        return None;
    }

    if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
        return None;
    }

    let valid = email.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || matches!(character, '@' | '.' | '_' | '%' | '+' | '-' | '\'')
    });

    valid.then_some(email)
}

/// Validates password length and a minimal complexity floor.
pub fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.chars().count() < MIN_PASSWORD_LENGTH {
        return Err(ApiError::bad_request(format!(
            "the password must be at least {MIN_PASSWORD_LENGTH} characters long"
        )));
    }

    if password.len() > MAX_PASSWORD_LENGTH {
        return Err(ApiError::bad_request(format!(
            "the password must be at most {MAX_PASSWORD_LENGTH} characters long"
        )));
    }

    if !password.chars().any(|character| character.is_alphabetic())
        || !password.chars().any(|character| character.is_numeric())
    {
        return Err(ApiError::bad_request(
            "the password must contain at least one letter and one number",
        ));
    }

    Ok(())
}

/// Validates a display name.
pub fn validate_full_name(name: &str) -> Result<String, ApiError> {
    let name = name.trim();

    if name.chars().count() < 2 || name.chars().count() > 120 {
        return Err(ApiError::bad_request(
            "the full name must be between 2 and 120 characters",
        ));
    }

    if name.chars().any(char::is_control) {
        return Err(ApiError::bad_request(
            "the full name must not contain control characters",
        ));
    }

    Ok(name.to_owned())
}

/// Hashes a password with Argon2id.
pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut PasswordOsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| ApiError::internal(format!("password hashing failed: {error}")))
}

/// Verifies a password against an Argon2 hash.
#[must_use]
pub fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded)
        .map(|hash| {
            Argon2::default()
                .verify_password(password.as_bytes(), &hash)
                .is_ok()
        })
        .unwrap_or(false)
}

/// A stable dummy hash used to equalize unknown-account login attempts.
fn dummy_password_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| {
        hash_password("kernext-timing-equalizer-0").unwrap_or_else(|_| {
            "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$0000000000000000000000000000000000000000000"
                .to_owned()
        })
    })
}

/// Generates a 256-bit random token encoded as lowercase hex.
#[must_use]
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Hashes an opaque token before storage or lookup.
#[must_use]
pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn internal(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(format!("database operation failed: {error}"))
}

impl Database {
    /// Creates an account and its preferences in one transaction.
    ///
    /// Returns `Ok(None)` when the email is already registered, so callers can
    /// answer with a stable conflict error without leaking SQL details.
    pub async fn create_user(
        &self,
        full_name: &str,
        email: &str,
        password: &str,
    ) -> Result<Option<PublicUser>, ApiError> {
        let id = Uuid::new_v4().to_string();
        let password_hash = hash_password(password)?;
        let now = Utc::now().naive_utc();

        let mut transaction = self.pool.begin().await.map_err(internal)?;

        let inserted = sqlx::query(
            "INSERT INTO users (id, full_name, email, password_hash, email_verified_at, \
             created_at, updated_at, account_status) VALUES (?, ?, ?, ?, NULL, ?, ?, 'active')",
        )
        .bind(&id)
        .bind(full_name)
        .bind(email)
        .bind(&password_hash)
        .bind(now)
        .bind(now)
        .execute(&mut *transaction)
        .await;

        match inserted {
            Ok(_) => {}
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
                return Ok(None);
            }
            Err(error) => return Err(internal(error)),
        }

        sqlx::query(
            "INSERT INTO user_preferences (id, user_id, theme, terminal_preferences, created_at, \
             updated_at) VALUES (?, ?, 'dark', NULL, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&id)
        .bind(now)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(internal)?;

        transaction.commit().await.map_err(internal)?;

        Ok(Some(PublicUser {
            id,
            full_name: full_name.to_owned(),
            email: email.to_owned(),
            email_verified: false,
            created_at: now.and_utc(),
            account_status: "active".to_owned(),
        }))
    }

    /// Looks up a user by normalized email.
    pub async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<(PublicUser, String)>, ApiError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, full_name, email, password_hash, email_verified_at, created_at, \
             account_status FROM users WHERE email = ? LIMIT 1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(internal)?;

        Ok(row.map(|row| (row.public(), row.password_hash)))
    }

    /// Loads one user by identifier.
    pub async fn find_user_by_id(&self, id: &str) -> Result<Option<PublicUser>, ApiError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, full_name, email, password_hash, email_verified_at, created_at, \
             account_status FROM users WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(internal)?;

        Ok(row.map(|row| row.public()))
    }

    /// Verifies credentials. Returns `None` for unknown or suspended accounts.
    ///
    /// An Argon2 verification runs even when the email is unknown so the
    /// response time does not disclose account existence.
    pub async fn verify_credentials(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<PublicUser>, ApiError> {
        let found = self.find_user_by_email(email).await?;

        let Some((user, hash)) = found else {
            let _ = verify_password(password, dummy_password_hash());
            return Ok(None);
        };

        if !verify_password(password, &hash) {
            return Ok(None);
        }

        if user.account_status != "active" {
            return Ok(None);
        }

        Ok(Some(user))
    }

    /// Creates a server-side session. Returns the raw cookie token and CSRF token.
    pub async fn create_session(
        &self,
        user_id: &str,
        remember: bool,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
        config: &AuthConfig,
    ) -> Result<NewSession, ApiError> {
        let token = generate_token();
        let csrf = generate_token();
        let now = Utc::now();
        let expires_at = if remember {
            now + Duration::days(config.remember_days)
        } else {
            now + Duration::hours(config.session_hours)
        };

        sqlx::query(
            "INSERT INTO user_sessions (id, user_id, session_token_hash, csrf_token_hash, \
             user_agent, ip_address, expires_at, created_at, last_used_at, revoked_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(hash_token(&token))
        .bind(hash_token(&csrf))
        .bind(user_agent.map(|value| value.chars().take(255).collect::<String>()))
        .bind(ip_address.map(|value| value.chars().take(64).collect::<String>()))
        .bind(expires_at.naive_utc())
        .bind(now.naive_utc())
        .bind(now.naive_utc())
        .execute(&self.pool)
        .await
        .map_err(internal)?;

        Ok(NewSession {
            token,
            csrf_token: csrf,
            expires_at,
        })
    }

    /// Resolves the user for a raw session token and refreshes `last_used_at`.
    pub async fn resolve_session(&self, token: &str) -> Result<Option<AuthUser>, ApiError> {
        let row = sqlx::query_as::<_, SessionRow>(
            "SELECT s.id AS session_id, s.csrf_token_hash, s.expires_at, u.id, u.full_name, \
             u.email, u.email_verified_at, u.created_at, u.account_status \
             FROM user_sessions s JOIN users u ON u.id = s.user_id \
             WHERE s.session_token_hash = ? AND s.revoked_at IS NULL \
             AND s.expires_at > UTC_TIMESTAMP(6) LIMIT 1",
        )
        .bind(hash_token(token))
        .fetch_optional(&self.pool)
        .await
        .map_err(internal)?;

        let Some(row) = row else {
            return Ok(None);
        };

        if row.account_status != "active" {
            return Ok(None);
        }

        let _ =
            sqlx::query("UPDATE user_sessions SET last_used_at = UTC_TIMESTAMP(6) WHERE id = ?")
                .bind(&row.session_id)
                .execute(&self.pool)
                .await;

        Ok(Some(AuthUser {
            user: PublicUser {
                id: row.id,
                full_name: row.full_name,
                email: row.email,
                email_verified: row.email_verified_at.is_some(),
                created_at: row.created_at.and_utc(),
                account_status: row.account_status,
            },
            session_id: row.session_id,
            csrf_token_hash: row.csrf_token_hash,
            expires_at: row.expires_at.and_utc(),
        }))
    }

    /// Revokes one session immediately.
    pub async fn revoke_session(&self, session_id: &str) -> Result<(), ApiError> {
        sqlx::query(
            "UPDATE user_sessions SET revoked_at = UTC_TIMESTAMP(6) \
             WHERE id = ? AND revoked_at IS NULL",
        )
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(internal)?;
        Ok(())
    }

    /// Revokes every session for a user, optionally keeping the current one.
    pub async fn revoke_user_sessions(
        &self,
        user_id: &str,
        except: Option<&str>,
    ) -> Result<(), ApiError> {
        match except {
            Some(session_id) => sqlx::query(
                "UPDATE user_sessions SET revoked_at = UTC_TIMESTAMP(6) \
                 WHERE user_id = ? AND id <> ? AND revoked_at IS NULL",
            )
            .bind(user_id)
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(internal)?,
            None => sqlx::query(
                "UPDATE user_sessions SET revoked_at = UTC_TIMESTAMP(6) \
                 WHERE user_id = ? AND revoked_at IS NULL",
            )
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(internal)?,
        };

        Ok(())
    }

    /// Lists a user's live sessions. Ownership is enforced by the query.
    pub async fn list_sessions(
        &self,
        user_id: &str,
        current_session: &str,
    ) -> Result<Vec<SessionSummary>, ApiError> {
        let rows = sqlx::query(
            "SELECT id, created_at, last_used_at, expires_at FROM user_sessions \
             WHERE user_id = ? AND revoked_at IS NULL AND expires_at > UTC_TIMESTAMP(6) \
             ORDER BY last_used_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(internal)?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let id: String = row.get("id");
                SessionSummary {
                    current: id == current_session,
                    id,
                    created_at: row.get::<NaiveDateTime, _>("created_at").and_utc(),
                    last_used_at: row.get::<NaiveDateTime, _>("last_used_at").and_utc(),
                    expires_at: row.get::<NaiveDateTime, _>("expires_at").and_utc(),
                }
            })
            .collect())
    }

    /// Revokes one of the caller's sessions. Returns `false` when the session
    /// does not belong to that user.
    pub async fn revoke_owned_session(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<bool, ApiError> {
        let result = sqlx::query(
            "UPDATE user_sessions SET revoked_at = UTC_TIMESTAMP(6) \
             WHERE id = ? AND user_id = ? AND revoked_at IS NULL",
        )
        .bind(session_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(internal)?;

        Ok(result.rows_affected() > 0)
    }

    /// Loads the user's preferences, creating a default row when missing.
    pub async fn preferences(&self, user_id: &str) -> Result<Preferences, ApiError> {
        let row = sqlx::query(
            "SELECT theme, terminal_preferences FROM user_preferences WHERE user_id = ? LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(internal)?;

        let Some(row) = row else {
            return Ok(Preferences::default());
        };

        let terminal: Option<serde_json::Value> = row.get("terminal_preferences");
        Ok(Preferences {
            theme: row.get("theme"),
            terminal_preferences: terminal,
        })
    }

    /// Replaces the user's preferences.
    pub async fn save_preferences(
        &self,
        user_id: &str,
        theme: &str,
        terminal: Option<&serde_json::Value>,
    ) -> Result<(), ApiError> {
        let now = Utc::now().naive_utc();
        sqlx::query(
            "INSERT INTO user_preferences (id, user_id, theme, terminal_preferences, created_at, \
             updated_at) VALUES (?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE theme = VALUES(theme), \
             terminal_preferences = VALUES(terminal_preferences), updated_at = VALUES(updated_at)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(theme)
        .bind(terminal)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(internal)?;
        Ok(())
    }

    /// Updates the profile fields a user may change.
    pub async fn update_profile(
        &self,
        user_id: &str,
        full_name: &str,
        email: &str,
    ) -> Result<Option<PublicUser>, ApiError> {
        let result = sqlx::query(
            "UPDATE users SET full_name = ?, email = ?, \
             email_verified_at = IF(email = ?, email_verified_at, NULL), \
             updated_at = UTC_TIMESTAMP(6) WHERE id = ?",
        )
        .bind(full_name)
        .bind(email)
        .bind(email)
        .bind(user_id)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {}
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => return Ok(None),
            Err(error) => return Err(internal(error)),
        }

        self.find_user_by_id(user_id).await
    }

    /// Replaces the password hash and revokes every session except `keep`.
    pub async fn change_password(
        &self,
        user_id: &str,
        password_hash: &str,
        keep: Option<&str>,
    ) -> Result<(), ApiError> {
        let mut transaction = self.pool.begin().await.map_err(internal)?;

        sqlx::query(
            "UPDATE users SET password_hash = ?, updated_at = UTC_TIMESTAMP(6) WHERE id = ?",
        )
        .bind(password_hash)
        .bind(user_id)
        .execute(&mut *transaction)
        .await
        .map_err(internal)?;

        match keep {
            Some(session_id) => {
                sqlx::query(
                    "UPDATE user_sessions SET revoked_at = UTC_TIMESTAMP(6) \
                     WHERE user_id = ? AND id <> ? AND revoked_at IS NULL",
                )
                .bind(user_id)
                .bind(session_id)
                .execute(&mut *transaction)
                .await
                .map_err(internal)?;
            }
            None => {
                sqlx::query(
                    "UPDATE user_sessions SET revoked_at = UTC_TIMESTAMP(6) \
                     WHERE user_id = ? AND revoked_at IS NULL",
                )
                .bind(user_id)
                .execute(&mut *transaction)
                .await
                .map_err(internal)?;
            }
        }

        transaction.commit().await.map_err(internal)?;
        Ok(())
    }

    /// Fetches the current password hash for a user.
    pub async fn password_hash(&self, user_id: &str) -> Result<Option<String>, ApiError> {
        let row = sqlx::query("SELECT password_hash FROM users WHERE id = ? LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(internal)?;

        Ok(row.map(|row| row.get("password_hash")))
    }

    /// Creates a single-use token row, replacing older unused tokens of the
    /// same kind for that user so only the newest link works.
    pub async fn create_token(
        &self,
        table: TokenTable,
        user_id: &str,
        lifetime: Duration,
    ) -> Result<String, ApiError> {
        let token = generate_token();
        let now = Utc::now().naive_utc();
        let expires = (Utc::now() + lifetime).naive_utc();
        let mut transaction = self.pool.begin().await.map_err(internal)?;

        sqlx::query(&format!(
            "UPDATE {} SET used_at = UTC_TIMESTAMP(6) WHERE user_id = ? AND used_at IS NULL",
            table.name()
        ))
        .bind(user_id)
        .execute(&mut *transaction)
        .await
        .map_err(internal)?;

        sqlx::query(&format!(
            "INSERT INTO {} (id, user_id, token_hash, expires_at, used_at, created_at) \
             VALUES (?, ?, ?, ?, NULL, ?)",
            table.name()
        ))
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(hash_token(&token))
        .bind(expires)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(internal)?;

        transaction.commit().await.map_err(internal)?;
        Ok(token)
    }

    /// Resolves a live single-use token to its user.
    pub async fn resolve_token(
        &self,
        table: TokenTable,
        token: &str,
    ) -> Result<Option<(String, String)>, ApiError> {
        let row = sqlx::query(&format!(
            "SELECT id, user_id FROM {} WHERE token_hash = ? AND used_at IS NULL \
             AND expires_at > UTC_TIMESTAMP(6) LIMIT 1",
            table.name()
        ))
        .bind(hash_token(token))
        .fetch_optional(&self.pool)
        .await
        .map_err(internal)?;

        Ok(row.map(|row| (row.get("id"), row.get("user_id"))))
    }

    /// Marks a token as used.
    pub async fn consume_token(&self, table: TokenTable, token_id: &str) -> Result<bool, ApiError> {
        let result = sqlx::query(&format!(
            "UPDATE {} SET used_at = UTC_TIMESTAMP(6) WHERE id = ? AND used_at IS NULL",
            table.name()
        ))
        .bind(token_id)
        .execute(&self.pool)
        .await
        .map_err(internal)?;

        Ok(result.rows_affected() > 0)
    }

    /// Marks a user's email as verified.
    pub async fn mark_email_verified(&self, user_id: &str) -> Result<(), ApiError> {
        sqlx::query(
            "UPDATE users SET email_verified_at = UTC_TIMESTAMP(6), \
             updated_at = UTC_TIMESTAMP(6) WHERE id = ? AND email_verified_at IS NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(internal)?;
        Ok(())
    }

    /// Deletes expired or revoked sessions. Active sessions are never touched.
    pub async fn purge_sessions(&self) -> Result<u64, ApiError> {
        let result = sqlx::query(
            "DELETE FROM user_sessions WHERE expires_at < UTC_TIMESTAMP(6) \
             OR revoked_at IS NOT NULL",
        )
        .execute(&self.pool)
        .await
        .map_err(internal)?;
        Ok(result.rows_affected())
    }
}

/// Token-bearing tables.
#[derive(Debug, Clone, Copy)]
pub enum TokenTable {
    PasswordReset,
    EmailVerification,
}

impl TokenTable {
    const fn name(self) -> &'static str {
        match self {
            Self::PasswordReset => "password_reset_tokens",
            Self::EmailVerification => "email_verification_tokens",
        }
    }
}

/// Values returned when a new session is created.
#[derive(Debug, Clone)]
pub struct NewSession {
    pub token: String,
    pub csrf_token: String,
    pub expires_at: DateTime<Utc>,
}

/// Fixed-window in-memory rate limiter for login and recovery endpoints.
///
/// Keys combine the action, client address, and (for login) the email, so a
/// single abusive client cannot lock out every account.
#[derive(Debug, Default)]
pub struct RateLimiter {
    windows: Mutex<HashMap<String, (Instant, u32)>>,
}

impl RateLimiter {
    /// Records an attempt. Returns `false` when the limit is exhausted.
    pub fn check(&self, key: &str, max: u32, window: StdDuration) -> bool {
        let Ok(mut windows) = self.windows.lock() else {
            return false;
        };

        let now = Instant::now();
        let entry = windows.entry(key.to_owned()).or_insert((now, 0));

        if now.duration_since(entry.0) > window {
            *entry = (now, 1);
            return true;
        }

        entry.1 = entry.1.saturating_add(1);
        entry.1 <= max
    }

    /// Clears a key after a successful attempt.
    pub fn clear(&self, key: &str) {
        if let Ok(mut windows) = self.windows.lock() {
            windows.remove(key);
        }
    }
}

fn cookie_values(headers: &HeaderMap) -> Vec<(String, String)> {
    headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| {
            let pair = pair.trim();
            let (name, value) = pair.split_once('=')?;
            Some((name.trim().to_owned(), value.trim().to_owned()))
        })
        .collect()
}

/// Reads a cookie from the request.
#[must_use]
pub fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    cookie_values(headers)
        .into_iter()
        .find(|(cookie_name, _)| cookie_name == name)
        .map(|(_, value)| value)
}

/// Reads the raw session cookie token.
#[must_use]
pub fn session_cookie(headers: &HeaderMap) -> Option<String> {
    cookie(headers, SESSION_COOKIE)
}

/// Reads the raw CSRF cookie value.
#[must_use]
pub fn csrf_cookie(headers: &HeaderMap) -> Option<String> {
    cookie(headers, CSRF_COOKIE)
}

/// Builds the `Set-Cookie` pair for a freshly issued session.
#[must_use]
pub fn session_cookies(session: &NewSession, remember: bool, config: &AuthConfig) -> [String; 2] {
    let max_age = if remember {
        config.remember_days * 24 * 60 * 60
    } else {
        config.session_hours * 60 * 60
    };
    let secure = if config.secure_cookies {
        "; Secure"
    } else {
        ""
    };

    let session_cookie = format!(
        "{SESSION_COOKIE}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}{secure}",
        session.token
    );
    let csrf_cookie = format!(
        "{CSRF_COOKIE}={}; Path=/; SameSite=Strict; Max-Age={max_age}{secure}",
        session.csrf_token
    );

    [session_cookie, csrf_cookie]
}

/// Builds the `Set-Cookie` pair that clears both cookies.
#[must_use]
pub fn clear_cookies(config: &AuthConfig) -> [String; 2] {
    let secure = if config.secure_cookies {
        "; Secure"
    } else {
        ""
    };
    [
        format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{secure}"),
        format!("{CSRF_COOKIE}=; Path=/; SameSite=Strict; Max-Age=0{secure}"),
    ]
}

/// Verifies the CSRF header against the session's stored CSRF hash.
#[must_use]
pub fn verify_csrf(headers: &HeaderMap, user: &AuthUser) -> bool {
    let header = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok());

    let Some(header) = header else {
        return false;
    };

    hash_token(header) == user.csrf_token_hash
}

/// Extracts a best-effort client address for rate limiting and audit rows.
#[must_use]
pub fn client_address(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "local".to_owned())
}

/// Outbound transactional email.
#[derive(Clone)]
pub struct Mailer {
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    from: Option<Mailbox>,
    public_url: String,
}

impl Mailer {
    /// Builds a mailer from `SMTP_*` environment variables.
    #[must_use]
    pub fn from_env(public_url: &str) -> Self {
        let host = std::env::var("SMTP_HOST")
            .ok()
            .filter(|value| !value.is_empty());
        let from = std::env::var("SMTP_FROM")
            .ok()
            .and_then(|value| value.parse::<Mailbox>().ok());

        let Some(host) = host else {
            return Self {
                transport: None,
                from: None,
                public_url: public_url.to_owned(),
            };
        };

        let port = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(587);

        let builder = if port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&host).map(|builder| builder.port(port))
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
                .map(|builder| builder.port(port))
        };

        let transport = builder.ok().map(|builder| {
            match (
                std::env::var("SMTP_USERNAME"),
                std::env::var("SMTP_PASSWORD"),
            ) {
                (Ok(username), Ok(password)) if !username.is_empty() => builder
                    .credentials(Credentials::new(username, password))
                    .build(),
                _ => builder.build(),
            }
        });

        Self {
            transport,
            from,
            public_url: public_url.to_owned(),
        }
    }

    /// Whether an SMTP transport and sender address are configured.
    #[must_use]
    pub fn configured(&self) -> bool {
        self.transport.is_some() && self.from.is_some()
    }

    /// Public application URL used in links.
    #[must_use]
    pub fn public_url(&self) -> &str {
        &self.public_url
    }

    /// Sends an email. Returns an error message when delivery is unavailable or
    /// fails; callers must not claim success in that case.
    pub async fn send(&self, to: &str, subject: &str, body: String) -> Result<(), String> {
        let (Some(transport), Some(from)) = (&self.transport, &self.from) else {
            return Err("smtp_not_configured".to_owned());
        };

        let message = Message::builder()
            .from(from.clone())
            .to(to
                .parse::<Mailbox>()
                .map_err(|_| "invalid recipient address".to_owned())?)
            .subject(subject)
            .body(body)
            .map_err(|error| error.to_string())?;

        transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
