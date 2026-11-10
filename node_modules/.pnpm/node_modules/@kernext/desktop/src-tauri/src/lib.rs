//! KERNEXT desktop application runtime.
//!
//! This crate owns the Tauri 2 window and exposes the local shell engine,
//! pseudoterminal sessions, filesystem operations, command catalog, and
//! desktop settings as Tauri commands. The web frontend is reused for the
//! user interface, but the desktop never routes commands through the public
//! Axum server: it calls this local Rust runtime directly.

pub mod command_catalog;
pub mod filesystem;
pub mod settings;
pub mod state;
pub mod terminal;

use settings::{DesktopSettings, SettingsError};
use shell_api::{API_VERSION, HealthResponse};
use shell_core::{PRODUCT_NAME, VERSION};
use state::{DesktopSessionSummary, DesktopState, SessionKind};
use tauri::{AppHandle, Manager, State};

/// Starts the desktop application.
///
/// # Errors
///
/// Returns a [`tauri::Error`] when the configuration directory cannot be
/// resolved, the window cannot be created, or the event loop fails.
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;

            let settings = DesktopSettings::load(&config_dir)
                .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)?;

            app.manage(DesktopState::new(settings, config_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            desktop_health,
            desktop_dashboard,
            desktop_create_session,
            desktop_execute_session,
            desktop_delete_session,
            desktop_list_sessions,
            desktop_create_pty_session,
            desktop_pty_write,
            desktop_pty_resize,
            desktop_pty_interrupt,
            desktop_pty_close,
            desktop_fs_list,
            desktop_fs_create_directory,
            desktop_fs_rename,
            desktop_fs_delete,
            desktop_fs_open,
            desktop_fs_session_directory,
            desktop_command_catalog,
            desktop_settings_get,
            desktop_settings_save,
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}

/// Returns the local runtime health snapshot.
#[tauri::command]
fn desktop_health(state: State<'_, DesktopState>) -> HealthResponse {
    let engine_sessions = state.sessions.len();
    let pty_sessions = state.pty.list().len();

    HealthResponse {
        status: "ok".to_owned(),
        product: PRODUCT_NAME.to_owned(),
        version: VERSION.to_owned(),
        api_version: API_VERSION.to_owned(),
        sessions: engine_sessions + pty_sessions,
        platform: state::platform_label().to_owned(),
        pty_available: true,
    }
}

/// Returns the dashboard snapshot for the desktop home screen.
#[tauri::command]
fn desktop_dashboard(state: State<'_, DesktopState>) -> state::DesktopDashboard {
    state.dashboard()
}

/// Creates a new in-process engine session.
#[tauri::command]
fn desktop_create_session(
    state: State<'_, DesktopState>,
    request: shell_api::CreateSessionRequest,
) -> Result<shell_api::SessionInfo, String> {
    terminal::create_engine_session(&state, request)
}

/// Executes one command line in an engine session.
#[tauri::command]
async fn desktop_execute_session(
    state: State<'_, DesktopState>,
    session_id: usize,
    command: String,
) -> Result<shell_api::ExecuteCommandResponse, String> {
    terminal::execute_engine_session(&state, session_id, command).await
}

/// Deletes an engine session.
#[tauri::command]
fn desktop_delete_session(state: State<'_, DesktopState>, session_id: usize) -> bool {
    terminal::delete_engine_session(&state, session_id)
}

/// Lists all engine and PTY sessions.
#[tauri::command]
fn desktop_list_sessions(state: State<'_, DesktopState>) -> Vec<DesktopSessionSummary> {
    state.dashboard().sessions
}

/// Starts a KERNEXT worker on a pseudoterminal.
#[tauri::command]
async fn desktop_create_pty_session(
    app: AppHandle,
    state: State<'_, DesktopState>,
    rows: u16,
    cols: u16,
) -> Result<DesktopSessionSummary, String> {
    terminal::create_pty_session(app, &state, rows, cols).await
}

/// Writes input to a PTY session.
#[tauri::command]
fn desktop_pty_write(
    state: State<'_, DesktopState>,
    session_id: u32,
    data: String,
) -> Result<(), String> {
    terminal::pty_write(&state, session_id, &data)
}

/// Resizes a PTY session.
#[tauri::command]
fn desktop_pty_resize(
    state: State<'_, DesktopState>,
    session_id: u32,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    terminal::pty_resize(&state, session_id, rows, cols)
}

/// Sends Ctrl+C to a PTY session.
#[tauri::command]
fn desktop_pty_interrupt(state: State<'_, DesktopState>, session_id: u32) -> Result<(), String> {
    terminal::pty_interrupt(&state, session_id)
}

/// Closes a PTY session.
#[tauri::command]
fn desktop_pty_close(state: State<'_, DesktopState>, session_id: u32) -> bool {
    terminal::pty_close(&state, session_id)
}

/// Lists a directory in the file explorer.
#[tauri::command]
fn desktop_fs_list(
    state: State<'_, DesktopState>,
    path: Option<String>,
) -> Result<filesystem::DirectoryListing, String> {
    filesystem::list_directory(&state, path)
}

/// Creates a directory.
#[tauri::command]
fn desktop_fs_create_directory(path: String) -> Result<(), String> {
    filesystem::create_directory(path)
}

/// Renames a file or directory.
#[tauri::command]
fn desktop_fs_rename(from: String, to: String) -> Result<(), String> {
    filesystem::rename_entry(from, to)
}

/// Deletes a file or directory.
#[tauri::command]
fn desktop_fs_delete(path: String, recursive: bool) -> Result<(), String> {
    filesystem::delete_entry(path, recursive)
}

/// Opens a path with the operating system's default application.
#[tauri::command]
fn desktop_fs_open(app: AppHandle, path: String) -> Result<(), String> {
    filesystem::open_path(&app, path)
}

/// Returns the working directory of a terminal session.
#[tauri::command]
fn desktop_fs_session_directory(
    state: State<'_, DesktopState>,
    session_id: u32,
    kind: SessionKind,
) -> Result<String, String> {
    filesystem::session_working_directory(&state, session_id, kind)
}

/// Returns the command catalog for this host.
#[tauri::command]
fn desktop_command_catalog(
    state: State<'_, DesktopState>,
) -> Vec<command_catalog::CommandCatalogEntry> {
    command_catalog::command_catalog(&state)
}

/// Returns the current desktop settings.
#[tauri::command]
fn desktop_settings_get(state: State<'_, DesktopState>) -> Result<DesktopSettings, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|_| "desktop settings lock was poisoned".to_owned())
}

/// Validates, persists, and applies new desktop settings.
#[tauri::command]
fn desktop_settings_save(
    state: State<'_, DesktopState>,
    settings: DesktopSettings,
) -> Result<DesktopSettings, String> {
    settings
        .validate()
        .map_err(|error: SettingsError| error.to_string())?;

    DesktopSettings::save(&state.settings_dir, &settings)
        .map_err(|error: SettingsError| error.to_string())?;

    let mut current = state
        .settings
        .lock()
        .map_err(|_| "desktop settings lock was poisoned".to_owned())?;
    *current = settings.clone();

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_names_are_stable() {
        assert_eq!(PRODUCT_NAME, "KERNEXT");
        assert_eq!(API_VERSION, "v1");
    }
}
