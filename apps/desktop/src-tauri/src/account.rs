//! Desktop account bridge.
//!
//! The desktop authenticates against a KERNEXT server, but only this Rust
//! runtime ever sees the session cookie. It is held in the operating system
//! credential store (Windows Credential Manager, macOS Keychain, or Secret
//! Service on Linux) and attached to server requests here. The webview receives
//! public profile data only.
//!
//! None of this is required for local shell use: if the server or database is
//! unreachable, account commands fail with a clear message while terminal,
//! filesystem, and settings commands keep working.

use keyring::Entry;
use serde::{Deserialize, Serialize};
use shell_server::db::{Preferences, PublicUser};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopAuthState {
    pub accounts_enabled: bool,
    pub email_configured: bool,
    pub dev_mail: bool,
    pub signed_in: bool,
    pub server_reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<PublicUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<Preferences>,
}

const KEYRING_SERVICE: &str = "dev.kernext.desktop";
const KEYRING_ENTRY: &str = "account-session";
const SESSION_COOKIE: &str = "kernext_session";
const CSRF_COOKIE: &str = "kernext_csrf";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSession {
    cookie: String,
    csrf: String,
}

/// Desktop account state, held in Tauri state and never exposing raw tokens
/// to the webview. The session cookie is stored in the OS credential store.
#[derive(Clone)]
pub struct AccountState {
    client: reqwest::Client,
    base_url: String,
    session: Arc<Mutex<Option<StoredSession>>>,
    user: Arc<Mutex<Option<PublicUser>>>,
    preferences: Arc<Mutex<Option<Preferences>>>,
    persistent: Arc<Mutex<bool>>,
}

impl Default for AccountState {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountState {
    /// Builds the state and restores a persisted session when one exists.
    #[must_use]
    pub fn new() -> Self {
        let base_url = std::env::var("KERNEXT_SERVER_URL")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:7878".to_owned())
            .trim_end_matches('/')
            .to_owned();

        let session = load_persisted();
        let persistent = session.is_some();

        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            base_url,
            session: Arc::new(Mutex::new(session)),
            user: Arc::new(Mutex::new(None)),
            preferences: Arc::new(Mutex::new(None)),
            persistent: Arc::new(Mutex::new(persistent)),
        }
    }

    fn stored(&self) -> Option<StoredSession> {
        self.session.lock().ok().and_then(|guard| guard.clone())
    }

    fn set_session(&self, session: Option<StoredSession>) {
        if let Ok(mut guard) = self.session.lock() {
            *guard = session.clone();
        }

        match session {
            Some(session) => {
                if persist(&session).is_ok() {
                    if let Ok(mut persistent) = self.persistent.lock() {
                        *persistent = true;
                    }
                }
            }
            None => {
                clear_persisted();
                if let Ok(mut persistent) = self.persistent.lock() {
                    *persistent = false;
                }
            }
        }
    }

    fn set_user(&self, user: Option<PublicUser>, preferences: Option<Preferences>) {
        if let Ok(mut guard) = self.user.lock() {
            *guard = user;
        }
        if let Ok(mut guard) = self.preferences.lock() {
            *guard = preferences;
        }
    }

    fn cookie_header(&self) -> Option<String> {
        self.stored().map(|session| {
            format!(
                "{SESSION_COOKIE}={}; {CSRF_COOKIE}={}",
                session.cookie, session.csrf
            )
        })
    }

    fn current_csrf(&self) -> Option<String> {
        self.stored().map(|session| session.csrf)
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
        csrf: bool,
    ) -> Result<(u16, serde_json::Value), String> {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.base_url));

        if let Some(cookie) = self.cookie_header() {
            request = request.header(reqwest::header::COOKIE, cookie);
        }

        if csrf {
            let token = self
                .current_csrf()
                .ok_or_else(|| "sign in again to continue".to_owned())?;
            request = request.header("x-csrf-token", token);
        }

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.map_err(|error| {
            format!(
                "The KERNEXT account server is not reachable at {}. Local terminal features still work. ({error})",
                self.base_url
            )
        })?;

        let status = response.status().as_u16();

        // Capture rotated session cookies before the body is consumed.
        // Track whether either cookie was updated so we persist even when
        // the server rotates only the CSRF cookie (no session cookie).
        let mut updated = self.stored();
        let mut has_cookie = false;
        for value in response.headers().get_all(reqwest::header::SET_COOKIE) {
            let Ok(value) = value.to_str() else {
                continue;
            };
            let Some((name, rest)) = value.split_once('=') else {
                continue;
            };
            let cookie_value = rest.split(';').next().unwrap_or("").to_owned();

            match name.trim() {
                SESSION_COOKIE => {
                    updated
                        .get_or_insert_with(|| StoredSession {
                            cookie: String::new(),
                            csrf: String::new(),
                        })
                        .cookie = cookie_value;
                    has_cookie = true;
                }
                CSRF_COOKIE => {
                    updated
                        .get_or_insert_with(|| StoredSession {
                            cookie: String::new(),
                            csrf: String::new(),
                        })
                        .csrf = cookie_value;
                    has_cookie = true;
                }
                _ => {}
            }
        }

        if let Some(updated) = updated {
            if has_cookie {
                self.set_session(Some(updated));
            }
        }

        let text = response.text().await.unwrap_or_default();
        let value = if text.trim().is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(serde_json::Value::Null)
        };

        Ok((status, value))
    }

    fn failure(status: u16, body: &serde_json::Value) -> String {
        body.get("message")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("The account server answered HTTP {status}."))
    }

    async fn reload_session(&self) -> Result<(), String> {
        let (status, body) = self
            .request(reqwest::Method::GET, "/api/auth/me", None, false)
            .await?;

        if status == 401 {
            self.set_session(None);
            self.set_user(None, None);
            return Ok(());
        }

        if status != 200 {
            return Err(Self::failure(status, &body));
        }

        let user: PublicUser = serde_json::from_value(
            body.get("user")
                .cloned()
                .ok_or_else(|| "the account response did not include a user".to_owned())?,
        )
        .map_err(|error| format!("the account response was malformed: {error}"))?;

        let preferences = serde_json::from_value(body.get("preferences").cloned().unwrap_or_else(
            || serde_json::json!({ "theme": "dark", "terminal_preferences": null }),
        ))
        .unwrap_or(Preferences {
            theme: "dark".to_owned(),
            terminal_preferences: None,
        });

        self.set_user(Some(user), Some(preferences));
        Ok(())
    }

    /// Returns the current account state, restoring a persisted session.
    pub async fn state(&self) -> DesktopAuthState {
        let mut accounts_enabled = false;
        let mut email_configured = false;
        let mut dev_mail = false;
        let mut server_reachable = false;

        if let Ok((status, body)) = self
            .request(reqwest::Method::GET, "/api/auth/status", None, false)
            .await
        {
            if status == 200 {
                server_reachable = true;
                accounts_enabled = body
                    .get("accounts_enabled")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                email_configured = body
                    .get("email_configured")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                dev_mail = body
                    .get("dev_mail")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
            }
        }

        if self.stored().is_some() && server_reachable {
            let _ = self.reload_session().await;
        }

        let user = self.user.lock().ok().and_then(|guard| guard.clone());
        let preferences = self.preferences.lock().ok().and_then(|guard| guard.clone());

        DesktopAuthState {
            accounts_enabled,
            email_configured,
            dev_mail,
            signed_in: user.is_some(),
            server_reachable,
            user,
            preferences,
        }
    }

    /// Signs in and stores the session cookie in the OS credential store.
    pub async fn sign_in(
        &self,
        email: String,
        password: String,
        remember: bool,
    ) -> Result<PublicUser, String> {
        let (status, body) = self
            .request(
                reqwest::Method::POST,
                "/api/auth/login",
                Some(serde_json::json!({
                    "email": email,
                    "password": password,
                    "remember": remember,
                })),
                false,
            )
            .await?;

        if status != 200 {
            return Err(Self::failure(status, &body));
        }

        self.reload_session().await?;
        self.user
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
            .ok_or_else(|| "the sign-in response did not include a user".to_owned())
    }

    /// Creates an account and keeps the returned session.
    pub async fn register(
        &self,
        full_name: String,
        email: String,
        password: String,
        accept_terms: bool,
    ) -> Result<(PublicUser, String, String), String> {
        let (status, body) = self
            .request(
                reqwest::Method::POST,
                "/api/auth/register",
                Some(serde_json::json!({
                    "full_name": full_name,
                    "email": email,
                    "password": password,
                    "accept_terms": accept_terms,
                })),
                false,
            )
            .await?;

        if status != 201 {
            return Err(Self::failure(status, &body));
        }

        let user: PublicUser = serde_json::from_value(
            body.get("user")
                .cloned()
                .ok_or_else(|| "the registration response did not include a user".to_owned())?,
        )
        .map_err(|error| format!("the registration response was malformed: {error}"))?;

        let preferences = Preferences {
            theme: "dark".to_owned(),
            terminal_preferences: None,
        };
        self.set_user(Some(user.clone()), Some(preferences));
        let delivery = body
            .get("email_delivery")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned();

        Ok((user, "Account created.".to_owned(), delivery))
    }

    /// Revokes the server session and clears local credentials.
    pub async fn sign_out(&self) -> Result<(), String> {
        if self.stored().is_some() {
            let _ = self
                .request(reqwest::Method::POST, "/api/auth/logout", None, true)
                .await;
        }

        self.set_session(None);
        self.set_user(None, None);
        Ok(())
    }

    pub async fn forgot_password(&self, email: String) -> Result<(String, String), String> {
        let (status, body) = self
            .request(
                reqwest::Method::POST,
                "/api/auth/forgot-password",
                Some(serde_json::json!({ "email": email })),
                false,
            )
            .await?;

        if status != 200 {
            return Err(Self::failure(status, &body));
        }

        Ok((
            body.get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("If an account exists, a reset link has been issued.")
                .to_owned(),
            body.get("email_delivery")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_owned(),
        ))
    }

    pub async fn reset_password(&self, token: String, password: String) -> Result<String, String> {
        let (status, body) = self
            .request(
                reqwest::Method::POST,
                "/api/auth/reset-password",
                Some(serde_json::json!({ "token": token, "password": password })),
                false,
            )
            .await?;

        if status != 200 {
            return Err(Self::failure(status, &body));
        }

        self.set_session(None);
        self.set_user(None, None);

        Ok(message(&body))
    }

    pub async fn verify_email(&self, token: String) -> Result<String, String> {
        let (status, body) = self
            .request(
                reqwest::Method::POST,
                "/api/auth/verify-email",
                Some(serde_json::json!({ "token": token })),
                false,
            )
            .await?;

        if status != 200 {
            return Err(Self::failure(status, &body));
        }

        if self.stored().is_some() {
            let _ = self.reload_session().await;
        }

        Ok(message(&body))
    }

    pub async fn change_password(
        &self,
        current_password: String,
        new_password: String,
    ) -> Result<String, String> {
        let (status, body) = self
            .request(
                reqwest::Method::POST,
                "/api/auth/change-password",
                Some(serde_json::json!({
                    "current_password": current_password,
                    "new_password": new_password,
                })),
                true,
            )
            .await?;

        if status != 200 {
            return Err(Self::failure(status, &body));
        }

        Ok(message(&body))
    }

    pub async fn update_profile(
        &self,
        full_name: String,
        email: String,
    ) -> Result<PublicUser, String> {
        let (status, body) = self
            .request(
                reqwest::Method::PATCH,
                "/api/auth/profile",
                Some(serde_json::json!({ "full_name": full_name, "email": email })),
                true,
            )
            .await?;

        if status != 200 {
            return Err(Self::failure(status, &body));
        }

        let user: PublicUser = serde_json::from_value(
            body.get("user")
                .cloned()
                .ok_or_else(|| "the profile response did not include a user".to_owned())?,
        )
        .map_err(|error| format!("the profile response was malformed: {error}"))?;

        if let Ok(mut guard) = self.user.lock() {
            *guard = Some(user.clone());
        }

        Ok(user)
    }

    pub async fn sessions(&self) -> Result<Vec<serde_json::Value>, String> {
        let (status, body) = self
            .request(reqwest::Method::GET, "/api/auth/sessions", None, false)
            .await?;

        if status != 200 {
            return Err(Self::failure(status, &body));
        }

        Ok(body
            .get("sessions")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    pub async fn revoke_session(&self, id: String) -> Result<(), String> {
        let (status, body) = self
            .request(
                reqwest::Method::DELETE,
                &format!("/api/auth/sessions/{id}"),
                None,
                true,
            )
            .await?;

        if status != 204 {
            return Err(Self::failure(status, &body));
        }

        Ok(())
    }

    pub async fn resend_verification(&self) -> Result<String, String> {
        let (status, body) = self
            .request(
                reqwest::Method::POST,
                "/api/auth/resend-verification",
                None,
                true,
            )
            .await?;

        if status != 200 {
            return Err(Self::failure(status, &body));
        }

        Ok(body
            .get("email_delivery")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned())
    }

    pub async fn update_preferences(&self, theme: String) -> Result<Preferences, String> {
        let (status, body) = self
            .request(
                reqwest::Method::PUT,
                "/api/auth/preferences",
                Some(serde_json::json!({ "theme": theme })),
                true,
            )
            .await?;

        if status != 200 {
            return Err(Self::failure(status, &body));
        }

        let preferences: Preferences = serde_json::from_value(body)
            .map_err(|error| format!("the preferences response was malformed: {error}"))?;

        if let Ok(mut guard) = self.preferences.lock() {
            *guard = Some(preferences.clone());
        }

        Ok(preferences)
    }

    /// Whether the current session survived a credential-store restart.
    #[must_use]
    pub fn persistent(&self) -> bool {
        self.persistent.lock().map(|value| *value).unwrap_or(false)
    }
}

fn message(body: &serde_json::Value) -> String {
    body.get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Done.")
        .to_owned()
}

fn entry() -> Option<Entry> {
    Entry::new(KEYRING_SERVICE, KEYRING_ENTRY).ok()
}

fn load_persisted() -> Option<StoredSession> {
    let password = entry()?.get_password().ok()?;
    serde_json::from_str(&password).ok()
}

fn persist(session: &StoredSession) -> Result<(), String> {
    let entry = entry().ok_or_else(|| "the OS credential store is unavailable".to_owned())?;
    let payload = serde_json::to_string(session)
        .map_err(|error| format!("could not encode the session: {error}"))?;
    entry
        .set_password(&payload)
        .map_err(|error| format!("could not store the session: {error}"))
}

fn clear_persisted() {
    if let Some(entry) = entry() {
        let _ = entry.delete_credential();
    }
}
