//! Filesystem operations for the desktop file explorer.
//!
//! Every operation resolves the requested path through the operating system
//! and reports the real result. Nothing here bypasses OS permissions; the
//! desktop application runs with the current user's normal rights.

use crate::state::{DesktopState, SessionKind};
use chrono::{DateTime, Local};
use serde::Serialize;
use shell_api::SessionId;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

/// One entry in a directory listing.
#[derive(Debug, Clone, Serialize)]
pub struct DirectoryEntry {
    /// File or directory name.
    pub name: String,
    /// Absolute path of the entry.
    pub path: String,
    /// Whether the entry is a directory.
    pub is_dir: bool,
    /// Size in bytes for files.
    pub size: Option<u64>,
    /// Last modification time in RFC 3339 format.
    pub modified: Option<String>,
}

/// Result of listing a directory.
#[derive(Debug, Clone, Serialize)]
pub struct DirectoryListing {
    /// Canonical directory path.
    pub path: String,
    /// Entries sorted by name.
    pub entries: Vec<DirectoryEntry>,
    /// Number of entries returned.
    pub total: usize,
}

/// Lists a directory.
///
/// `path` defaults to the configured working directory.
///
/// # Errors
///
/// Returns a human-readable error string when the path does not exist, is not
/// a directory, or cannot be read.
pub fn list_directory(
    state: &DesktopState,
    path: Option<String>,
) -> Result<DirectoryListing, String> {
    let base = match path {
        Some(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => state.default_working_directory(),
    };

    let canonical = canonical_directory(&base)?;
    let entries = read_entries(&canonical)?;
    let total = entries.len();

    Ok(DirectoryListing {
        path: canonical.to_string_lossy().into_owned(),
        entries,
        total,
    })
}

/// Creates a directory, including any missing parents.
///
/// # Errors
///
/// Returns a human-readable error string when the path is invalid or creation
/// fails.
pub fn create_directory(path: String) -> Result<(), String> {
    let path = validate_path(&path)?;

    fs::create_dir_all(&path)
        .map_err(|error| format!("could not create directory {path:?}: {error}"))
}

/// Renames or moves a file or directory.
///
/// # Errors
///
/// Returns a human-readable error string when either path is invalid or the
/// rename fails.
pub fn rename_entry(from: String, to: String) -> Result<(), String> {
    let from = validate_path(&from)?;
    let to = validate_path(&to)?;

    if !from.exists() {
        return Err(format!("cannot rename {from:?} because it does not exist"));
    }

    fs::rename(&from, &to).map_err(|error| format!("could not rename {from:?} to {to:?}: {error}"))
}

/// Deletes a file or directory.
///
/// # Errors
///
/// Returns a human-readable error string when the path is invalid or deletion
/// fails.
pub fn delete_entry(path: String, recursive: bool) -> Result<(), String> {
    let path = validate_path(&path)?;

    if path.is_file() {
        fs::remove_file(&path).map_err(|error| format!("could not delete file {path:?}: {error}"))
    } else if path.is_dir() {
        if recursive {
            fs::remove_dir_all(&path)
                .map_err(|error| format!("could not delete directory {path:?}: {error}"))
        } else {
            fs::remove_dir(&path).map_err(|error| {
                format!("could not delete directory {path:?}: {error}. Use the recursive option for non-empty directories.")
            })
        }
    } else {
        Err(format!("cannot delete {path:?} because it does not exist"))
    }
}

/// Opens a path with the operating system's default application.
///
/// # Errors
///
/// Returns a human-readable error string when the path does not exist or the
/// OS refuses to open it.
pub fn open_path(app: &AppHandle, path: String) -> Result<(), String> {
    let path = validate_path(&path)?;

    if !path.exists() {
        return Err(format!("cannot open {path:?} because it does not exist"));
    }

    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|error| format!("could not open {path:?} with the default application: {error}"))
}

/// Returns the current working directory of a terminal session.
///
/// Engine sessions report their live working directory. PTY sessions report
/// the directory the worker was started in, because the worker owns its own
/// process state.
///
/// # Errors
///
/// Returns a human-readable error string when the session does not exist.
pub fn session_working_directory(
    state: &DesktopState,
    id: u32,
    kind: SessionKind,
) -> Result<String, String> {
    match kind {
        SessionKind::Engine => {
            let session = state
                .sessions
                .get(SessionId(id as usize))
                .ok_or_else(|| format!("engine session {id} does not exist"))?;

            let info = session
                .lock()
                .map_err(|_| "engine session lock was poisoned".to_owned())?
                .info();

            Ok(info.working_directory)
        }
        SessionKind::Pty => {
            let session = state
                .pty
                .get(id)
                .ok_or_else(|| format!("pseudoterminal session {id} does not exist"))?;

            Ok(session.working_directory.to_string_lossy().into_owned())
        }
    }
}

/// Resolves and validates a user-provided path.
///
/// # Errors
///
/// Returns a human-readable error string when the path is empty or contains
/// NUL bytes.
fn validate_path(path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("path cannot be empty".to_owned());
    }

    if path.contains('\0') {
        return Err("path cannot contain NUL bytes".to_owned());
    }

    Ok(PathBuf::from(path))
}

/// Canonicalizes a directory and verifies that it is a directory.
///
/// # Errors
///
/// Returns a human-readable error string when the path cannot be resolved or
/// is not a directory.
fn canonical_directory(path: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("could not resolve directory {path:?}: {error}"))?;

    if !canonical.is_dir() {
        return Err(format!("{path:?} is not a directory"));
    }

    Ok(canonical)
}

/// Reads the entries of a directory sorted by name.
///
/// # Errors
///
/// Returns a human-readable error string when the directory cannot be read.
fn read_entries(directory: &Path) -> Result<Vec<DirectoryEntry>, String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("could not read directory {directory:?}: {error}"))?;

    let mut result = Vec::new();

    for entry in entries {
        let entry =
            entry.map_err(|error| format!("could not read entry in {directory:?}: {error}"))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|error| format!("could not read metadata for {path:?}: {error}"))?;

        let modified = metadata
            .modified()
            .ok()
            .map(format_timestamp)
            .unwrap_or_default();

        result.push(DirectoryEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: path.to_string_lossy().into_owned(),
            is_dir: metadata.is_dir(),
            size: if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            },
            modified: if modified.is_empty() {
                None
            } else {
                Some(modified)
            },
        });
    }

    result.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(result)
}

/// Formats a filesystem timestamp as RFC 3339 local time.
fn format_timestamp(timestamp: SystemTime) -> String {
    DateTime::<Local>::from(timestamp).to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::DesktopSettings;
    use std::fs;

    fn temp_state() -> (DesktopState, PathBuf) {
        let directory = std::env::temp_dir().join("kernext-desktop-fs-test");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("test directory should be created");

        let settings = DesktopSettings {
            default_working_directory: directory.to_string_lossy().into_owned(),
            ..DesktopSettings::default()
        };

        (
            DesktopState::new(settings, directory.join("settings")),
            directory,
        )
    }

    #[test]
    fn directories_list_their_entries() {
        let (state, directory) = temp_state();
        fs::write(directory.join("alpha.txt"), b"alpha").expect("file should be written");
        fs::create_dir(directory.join("nested")).expect("directory should be created");

        let listing = list_directory(&state, None).expect("listing should succeed");

        assert_eq!(listing.total, 2);
        assert!(
            listing
                .entries
                .iter()
                .any(|entry| entry.name == "alpha.txt")
        );
        assert!(
            listing
                .entries
                .iter()
                .any(|entry| entry.name == "nested" && entry.is_dir)
        );
    }

    #[test]
    fn empty_paths_are_rejected() {
        assert!(validate_path("   ").is_err());
        assert!(validate_path("with\0null").is_err());
    }

    #[test]
    fn directories_can_be_created_and_deleted() {
        let directory = std::env::temp_dir().join("kernext-desktop-fs-create-test");
        let _ = fs::remove_dir_all(&directory);

        create_directory(directory.to_string_lossy().into_owned())
            .expect("directory should be created");
        assert!(directory.is_dir());

        delete_entry(directory.to_string_lossy().into_owned(), false)
            .expect("empty directory should be deleted");
        assert!(!directory.exists());
    }
}
