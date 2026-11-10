//! Shared desktop application state.
//!
//! The desktop shell keeps two kinds of terminal session:
//!
//! - **engine** sessions run the KERNEXT evaluator inside this process, exactly
//!   like the local Axum server does.
//! - **pty** sessions spawn this same executable with
//!   `KERNEXT_DESKTOP_WORKER=1` on a pseudoterminal, which yields a real
//!   interactive KERNEXT process with full line editing, resize, and Ctrl+C.
//!
//! Session state is never exposed over the network; Tauri commands call into
//! this state directly.

use crate::settings::DesktopSettings;
use chrono::Local;
use serde::{Deserialize, Serialize};
use shell_builtins::{PtyResizer, PtySize};
use shell_core::VERSION;
use shell_server::AppState;
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Maximum number of recent commands retained for the dashboard.
const RECENT_COMMAND_LIMIT: usize = 40;

/// Terminal session kind reported to the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    /// In-process KERNEXT engine session.
    Engine,
    /// Real KERNEXT process on a pseudoterminal.
    Pty,
}

impl SessionKind {
    /// Stable machine-readable label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Engine => "engine",
            Self::Pty => "pty",
        }
    }
}

/// Unified session metadata used by the desktop dashboard and session list.
#[derive(Debug, Clone, Serialize)]
pub struct DesktopSessionSummary {
    /// Session identifier.
    pub id: u32,
    /// `engine` or `pty`.
    pub kind: SessionKind,
    /// Current working directory.
    pub working_directory: String,
    /// Lifecycle state.
    pub state: String,
    /// Host platform, for example `windows-x86_64`.
    pub platform: String,
    /// KERNEXT version.
    pub version: String,
    /// Prompt shown by this session.
    pub prompt: String,
    /// Number of jobs tracked by the session.
    pub job_count: usize,
}

impl DesktopSessionSummary {
    /// Builds an engine summary from the shared API session snapshot.
    #[must_use]
    pub fn from_engine(info: &shell_api::SessionInfo) -> Self {
        Self {
            id: info.id.0 as u32,
            kind: SessionKind::Engine,
            working_directory: info.working_directory.clone(),
            state: info.state.clone(),
            platform: info.platform.clone(),
            version: info.version.clone(),
            prompt: info.prompt.clone(),
            job_count: info.job_count,
        }
    }

    /// Builds a PTY summary from the local registry entry.
    #[must_use]
    pub fn from_pty(session: &PtySession) -> Self {
        Self {
            id: session.id,
            kind: SessionKind::Pty,
            working_directory: session.working_directory.to_string_lossy().into_owned(),
            state: "running".to_owned(),
            platform: platform_label().to_owned(),
            version: VERSION.to_owned(),
            prompt: session.prompt.clone(),
            job_count: 0,
        }
    }
}

/// One recently executed command, shown on the dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct RecentCommand {
    /// Command line as submitted.
    pub command: String,
    /// Exit status when known.
    pub exit_code: Option<i32>,
    /// RFC 3339 timestamp of execution.
    pub timestamp: String,
}

/// A live pseudoterminal session.
///
/// The `PtyProcess` itself is owned by a monitor thread that waits for the
/// child to exit. This struct only holds the handles the desktop needs after
/// the child is running: the writable end of the terminal, its resizer, and
/// the metadata shown in the UI.
pub struct PtySession {
    /// Session identifier.
    pub id: u32,
    /// Working directory the worker started in.
    pub working_directory: PathBuf,
    /// Prompt used by the worker.
    pub prompt: String,
    /// Writable end of the pseudoterminal.
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    /// Handle used to resize the pseudoterminal.
    resizer: Mutex<Option<PtyResizer>>,
    /// Time the session was created.
    created_at: Instant,
}

impl PtySession {
    /// Creates a session from the handles returned by `PtyProcess::spawn`.
    #[must_use]
    pub fn new(
        id: u32,
        working_directory: PathBuf,
        prompt: String,
        writer: Box<dyn Write + Send>,
        resizer: PtyResizer,
    ) -> Self {
        Self {
            id,
            working_directory,
            prompt,
            writer: Mutex::new(Some(writer)),
            resizer: Mutex::new(Some(resizer)),
            created_at: Instant::now(),
        }
    }

    /// Time the session was created.
    #[must_use]
    pub const fn created_at(&self) -> Instant {
        self.created_at
    }

    /// Writes raw input to the worker.
    ///
    /// # Errors
    ///
    /// Returns a human-readable message when the writer is gone or the write
    /// fails.
    pub fn write_input(&self, data: &[u8]) -> Result<(), String> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| "pseudoterminal writer lock was poisoned".to_owned())?;

        if let Some(writer) = writer.as_mut() {
            writer
                .write_all(data)
                .map_err(|error| format!("could not write to pseudoterminal: {error}"))?;
            writer
                .flush()
                .map_err(|error| format!("could not flush pseudoterminal: {error}"))?;
            Ok(())
        } else {
            Err("pseudoterminal session is closed".to_owned())
        }
    }

    /// Resizes the pseudoterminal.
    ///
    /// # Errors
    ///
    /// Returns a human-readable message when the resize is refused.
    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), String> {
        let guard = self
            .resizer
            .lock()
            .map_err(|_| "pseudoterminal resizer lock was poisoned".to_owned())?;

        if let Some(resizer) = *guard {
            resizer
                .resize(PtySize::new(rows, cols))
                .map_err(|error| format!("could not resize pseudoterminal: {error}"))
        } else {
            Err("pseudoterminal session is closed".to_owned())
        }
    }

    /// Sends Ctrl+C to the worker.
    ///
    /// # Errors
    ///
    /// Returns a human-readable message when the interrupt cannot be written.
    pub fn interrupt(&self) -> Result<(), String> {
        self.write_input(&[0x03])
    }

    /// Requests a graceful exit and releases the desktop-side handles.
    pub fn close(&self) {
        let writer = self.writer.lock().ok().and_then(|mut guard| guard.take());

        if let Some(mut writer) = writer {
            let _ = writer.write_all(b"exit\r\n");
            let _ = writer.flush();
        }

        if let Ok(mut resizer) = self.resizer.lock() {
            resizer.take();
        }
    }
}

/// Registry of live pseudoterminal sessions.
#[derive(Clone, Default)]
pub struct PtyRegistry {
    sessions: Arc<Mutex<HashMap<u32, Arc<PtySession>>>>,
    next_id: Arc<AtomicU32>,
}

impl PtyRegistry {
    /// Inserts a session and returns its identifier.
    pub fn insert(&self, session: Arc<PtySession>) -> u32 {
        let id = session.id;

        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(id, session);
        }

        id
    }

    /// Looks up a session.
    #[must_use]
    pub fn get(&self, id: u32) -> Option<Arc<PtySession>> {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&id).cloned())
    }

    /// Removes a session.
    pub fn remove(&self, id: u32) -> Option<Arc<PtySession>> {
        self.sessions
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.remove(&id))
    }

    /// Lists PTY sessions ordered by identifier.
    #[must_use]
    pub fn list(&self) -> Vec<DesktopSessionSummary> {
        let sessions = match self.sessions.lock() {
            Ok(guard) => guard.values().cloned().collect::<Vec<_>>(),
            Err(_) => return Vec::new(),
        };

        let mut listed = sessions
            .into_iter()
            .map(|session| DesktopSessionSummary::from_pty(&session))
            .collect::<Vec<_>>();

        listed.sort_by_key(|summary| summary.id);
        listed
    }

    /// Allocates the next session identifier.
    #[must_use]
    pub fn next_id(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

/// Dashboard snapshot assembled from live state.
#[derive(Debug, Clone, Serialize)]
pub struct DesktopDashboard {
    /// KERNEXT version.
    pub version: String,
    /// Host platform, for example `windows-x86_64`.
    pub platform: String,
    /// Working directory used for new sessions.
    pub working_directory: String,
    /// All live engine and PTY sessions.
    pub sessions: Vec<DesktopSessionSummary>,
    /// Recently executed engine commands.
    pub recent_commands: Vec<RecentCommand>,
    /// Whether pseudoterminal sessions can be started on this host.
    pub pty_available: bool,
    /// Seconds since the desktop runtime started.
    pub uptime_seconds: u64,
}

/// Root state managed by Tauri.
pub struct DesktopState {
    /// In-process engine session registry, shared with the local server code.
    pub sessions: AppState,
    /// Live pseudoterminal sessions.
    pub pty: PtyRegistry,
    /// Desktop presentation settings.
    pub settings: Mutex<DesktopSettings>,
    /// Directory that stores the settings file.
    pub settings_dir: PathBuf,
    /// Recently executed commands.
    recent_commands: Mutex<VecDeque<RecentCommand>>,
    /// Runtime start time.
    started_at: Instant,
}

impl DesktopState {
    /// Creates desktop state around validated settings.
    #[must_use]
    pub fn new(settings: DesktopSettings, settings_dir: PathBuf) -> Self {
        Self {
            sessions: AppState::local(),
            pty: PtyRegistry::default(),
            settings: Mutex::new(settings),
            settings_dir,
            recent_commands: Mutex::new(VecDeque::new()),
            started_at: Instant::now(),
        }
    }

    /// Working directory used for new sessions.
    ///
    /// Empty settings fall back to the user's home directory, then the process
    /// working directory. No path is hardcoded to a particular machine.
    #[must_use]
    pub fn default_working_directory(&self) -> PathBuf {
        let configured = self
            .settings
            .lock()
            .map(|settings| settings.default_working_directory.clone())
            .unwrap_or_default();

        if !configured.is_empty() {
            return PathBuf::from(configured);
        }

        home_directory()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// Records a recently executed engine command.
    pub fn record_recent_command(&self, command: &str, exit_code: Option<i32>) {
        if command.trim().is_empty() {
            return;
        }

        let entry = RecentCommand {
            command: command.to_owned(),
            exit_code,
            timestamp: Local::now().to_rfc3339(),
        };

        if let Ok(mut commands) = self.recent_commands.lock() {
            commands.push_front(entry);

            while commands.len() > RECENT_COMMAND_LIMIT {
                commands.pop_back();
            }
        }
    }

    /// Returns the recent command list ordered newest first.
    #[must_use]
    pub fn recent_commands(&self) -> Vec<RecentCommand> {
        self.recent_commands
            .lock()
            .map(|commands| commands.clone().into_iter().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    /// Builds the dashboard snapshot.
    #[must_use]
    pub fn dashboard(&self) -> DesktopDashboard {
        let mut sessions = self
            .sessions
            .list()
            .into_iter()
            .map(|info| DesktopSessionSummary::from_engine(&info))
            .collect::<Vec<_>>();

        sessions.extend(self.pty.list());
        sessions.sort_by_key(|summary| (summary.kind.label().to_owned(), summary.id));

        DesktopDashboard {
            version: VERSION.to_owned(),
            platform: platform_label().to_owned(),
            working_directory: self
                .default_working_directory()
                .to_string_lossy()
                .into_owned(),
            sessions,
            recent_commands: self.recent_commands(),
            pty_available: true,
            uptime_seconds: self.started_at.elapsed().as_secs(),
        }
    }
}

/// Best-effort user home directory without third-party crates.
fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
}

/// Stable `os-arch` label used across the desktop UI.
#[must_use]
pub fn platform_label() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x86_64",
        ("windows", "x86") => "windows-x86",
        ("windows", "aarch64") => "windows-aarch64",
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "x86_64") => "macos-x86_64",
        ("macos", "aarch64") => "macos-aarch64",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_label_is_stable() {
        assert!(platform_label().contains(std::env::consts::OS));
    }

    #[test]
    fn recent_commands_keep_newest_first() {
        let state = DesktopState::new(
            DesktopSettings::default(),
            std::env::temp_dir().join("kernext-desktop-state-test"),
        );

        state.record_recent_command("echo first", Some(0));
        state.record_recent_command("echo second", Some(0));

        let commands = state.recent_commands();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command, "echo second");
        assert_eq!(commands[1].command, "echo first");
    }
}
