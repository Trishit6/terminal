//! Configuration model and persistence for the custom shell.
//!
//! The CLI does not load a configuration file during Phase 1. The persistence
//! API is complete and tested so later phases can opt into it without changing
//! the configuration format.

use serde::{Deserialize, Serialize};
use shell_core::SHELL_NAME;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Default prompt shown before an interactive command.
pub const DEFAULT_PROMPT: &str = "custom-shell > ";

/// Default number of entries retained in command history.
pub const DEFAULT_HISTORY_SIZE: usize = 1_000;

/// Persistent user configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Text displayed before interactive input.
    pub prompt: String,
    /// Maximum number of command-history entries to retain.
    pub history_size: usize,
    /// Whether interactive output may use ANSI colors.
    pub color: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prompt: DEFAULT_PROMPT.to_owned(),
            history_size: DEFAULT_HISTORY_SIZE,
            color: false,
        }
    }
}

impl Config {
    /// Parses and validates configuration from TOML.
    pub fn from_toml(source: &str) -> Result<Self, ConfigError> {
        let config: Self =
            toml::from_str(source).map_err(|source| ConfigError::Parse { path: None, source })?;
        config.validate()?;
        Ok(config)
    }

    /// Serializes this configuration as TOML.
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        self.validate()?;
        toml::to_string_pretty(self).map_err(ConfigError::Serialize)
    }

    /// Loads and validates a configuration file.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let source = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        Self::from_toml(&source).map_err(|error| error.with_path(path))
    }

    /// Loads a configuration file, or returns defaults when it does not exist.
    pub fn load_or_default(path: &Path) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(source) => Self::from_toml(&source).map_err(|error| error.with_path(path)),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// Writes this configuration as TOML, creating parent directories.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let encoded = self.to_toml()?;

        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|source| ConfigError::CreateDirectory {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        fs::write(path, encoded).map_err(|source| ConfigError::Write {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Resolves the conventional per-user configuration path on this platform.
    ///
    /// Returns `None` when the platform's base directory environment variables
    /// are unavailable, rather than inventing a fallback location.
    #[must_use]
    pub fn default_path() -> Option<PathBuf> {
        let base = if cfg!(target_os = "windows") {
            env::var_os("APPDATA").map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            env::var_os("HOME").map(|home| {
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
            })
        } else {
            env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        }?;

        Some(base.join(SHELL_NAME).join("config.toml"))
    }

    /// Checks values that would be invalid at runtime.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.history_size == 0 {
            return Err(ConfigError::Invalid {
                message: "history_size must be greater than zero".to_owned(),
            });
        }

        Ok(())
    }
}

/// Errors produced while loading, validating, or saving configuration.
#[derive(Debug)]
pub enum ConfigError {
    /// A configuration file could not be read.
    Read {
        /// File that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// A parent directory could not be created.
    CreateDirectory {
        /// Directory that could not be created.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// A configuration file could not be written.
    Write {
        /// File that could not be written.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// TOML was syntactically valid but did not match the configuration schema.
    Parse {
        /// File being parsed, when the input came from a file.
        path: Option<PathBuf>,
        /// Underlying TOML error.
        source: toml::de::Error,
    },
    /// The configuration could not be serialized.
    Serialize(toml::ser::Error),
    /// A configuration value is not valid at runtime.
    Invalid {
        /// Explanation of the invalid value.
        message: String,
    },
}

impl ConfigError {
    fn with_path(self, path: &Path) -> Self {
        match self {
            Self::Parse { source, .. } => Self::Parse {
                path: Some(path.to_path_buf()),
                source,
            },
            other => other,
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "could not read {}: {source}", path.display())
            }
            Self::CreateDirectory { path, source } => {
                write!(formatter, "could not create {}: {source}", path.display())
            }
            Self::Write { path, source } => {
                write!(formatter, "could not write {}: {source}", path.display())
            }
            Self::Parse {
                path: Some(path),
                source,
            } => write!(formatter, "could not parse {}: {source}", path.display()),
            Self::Parse { path: None, source } => {
                write!(formatter, "could not parse configuration: {source}")
            }
            Self::Serialize(source) => {
                write!(formatter, "could not serialize configuration: {source}")
            }
            Self::Invalid { message } => write!(formatter, "invalid configuration: {message}"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. }
            | Self::CreateDirectory { source, .. }
            | Self::Write { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::Serialize(source) => Some(source),
            Self::Invalid { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        env::temp_dir().join(format!("shell-config-test-{}-{unique}", std::process::id()))
    }

    #[test]
    fn defaults_are_stable() {
        let config = Config::default();

        assert_eq!(config.prompt, DEFAULT_PROMPT);
        assert_eq!(config.history_size, DEFAULT_HISTORY_SIZE);
        assert!(!config.color);
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing_fields() {
        let config = Config::from_toml("color = false\n").expect("partial config should parse");

        assert_eq!(config.prompt, DEFAULT_PROMPT);
        assert_eq!(config.history_size, DEFAULT_HISTORY_SIZE);
        assert!(!config.color);
    }

    #[test]
    fn configuration_round_trips_through_toml() {
        let original = Config {
            prompt: "dev> ".to_owned(),
            history_size: 42,
            color: false,
        };

        let encoded = original.to_toml().expect("config should serialize");
        let decoded = Config::from_toml(&encoded).expect("config should parse");

        assert_eq!(decoded, original);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let error = Config::from_toml("unknown = true\n").expect_err("unknown field should fail");

        assert!(matches!(error, ConfigError::Parse { .. }));
    }

    #[test]
    fn zero_history_size_is_rejected() {
        let error = Config::from_toml("history_size = 0\n")
            .expect_err("zero history size should be invalid");

        assert!(matches!(error, ConfigError::Invalid { .. }));
    }

    #[test]
    fn save_and_load_round_trip_on_disk() {
        let directory = temporary_directory();
        let path = directory.join("nested").join("config.toml");
        let original = Config {
            prompt: "test> ".to_owned(),
            history_size: 7,
            color: false,
        };

        original.save(&path).expect("config should save");
        let loaded = Config::load(&path).expect("config should load");

        assert_eq!(loaded, original);
        fs::remove_dir_all(&directory).expect("temporary directory should be removable");
    }

    #[test]
    fn missing_file_uses_defaults() {
        let path = temporary_directory().join("does-not-exist.toml");

        let config = Config::load_or_default(&path).expect("missing config should use defaults");

        assert_eq!(config, Config::default());
    }
}
