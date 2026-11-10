//! Desktop application settings.
//!
//! Settings are persisted as TOML in the platform application-configuration
//! directory and validated before they are accepted. The desktop shell never
//! mutates the CLI's `shell-config` values; these values only drive the
//! desktop presentation layer.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Desktop settings file name inside the application configuration directory.
pub const SETTINGS_FILE: &str = "desktop-settings.toml";

/// Errors that can occur while loading, validating, or saving settings.
#[derive(Debug)]
pub enum SettingsError {
    /// A field value is not allowed.
    Invalid(String),
    /// The settings file could not be read or written.
    Io(io::Error),
    /// The settings file is not valid TOML.
    Toml(toml::de::Error),
    /// The settings could not be serialized.
    Serialize(toml::ser::Error),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid desktop settings: {message}"),
            Self::Io(error) => write!(formatter, "desktop settings I/O error: {error}"),
            Self::Toml(error) => write!(formatter, "desktop settings parse error: {error}"),
            Self::Serialize(error) => write!(formatter, "desktop settings write error: {error}"),
        }
    }
}

impl std::error::Error for SettingsError {}

impl From<io::Error> for SettingsError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<toml::de::Error> for SettingsError {
    fn from(error: toml::de::Error) -> Self {
        Self::Toml(error)
    }
}

impl From<toml::ser::Error> for SettingsError {
    fn from(error: toml::ser::Error) -> Self {
        Self::Serialize(error)
    }
}

/// Presentation settings for the KERNEXT desktop application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DesktopSettings {
    /// Visual theme: `midnight`, `light`, or `system`.
    pub theme: String,
    /// Terminal font size in points.
    pub terminal_font_size: u16,
    /// Terminal font family.
    pub terminal_font_family: String,
    /// Terminal cursor style: `block`, `bar`, or `underline`.
    pub terminal_cursor_style: String,
    /// Absolute working directory used for new terminal sessions.
    pub default_working_directory: String,
    /// Whether terminal history is enabled.
    pub history_enabled: bool,
    /// Maximum number of terminal history entries retained per session.
    pub history_size: u16,
    /// Whether the cinematic startup animation is shown.
    pub startup_animation: bool,
    /// Whether motion should be reduced throughout the interface.
    pub reduce_motion: bool,
    /// Accent color family: `cyan`, `blue`, or `violet`.
    pub accent_color: String,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            theme: "midnight".to_owned(),
            terminal_font_size: 14,
            terminal_font_family: "JetBrains Mono".to_owned(),
            terminal_cursor_style: "block".to_owned(),
            default_working_directory: String::new(),
            history_enabled: true,
            history_size: 500,
            startup_animation: true,
            reduce_motion: false,
            accent_color: "cyan".to_owned(),
        }
    }
}

impl DesktopSettings {
    /// Validates every field before the settings are applied or persisted.
    ///
    /// # Errors
    ///
    /// Returns a [`SettingsError::Invalid`] naming the first invalid field.
    pub fn validate(&self) -> Result<(), SettingsError> {
        if !matches!(self.theme.as_str(), "midnight" | "light" | "system") {
            return Err(SettingsError::Invalid(
                "theme must be one of midnight, light, or system".to_owned(),
            ));
        }

        if !(8..=32).contains(&self.terminal_font_size) {
            return Err(SettingsError::Invalid(
                "terminal_font_size must be between 8 and 32".to_owned(),
            ));
        }

        if self.terminal_font_family.trim().is_empty()
            || self.terminal_font_family.chars().any(char::is_control)
        {
            return Err(SettingsError::Invalid(
                "terminal_font_family must be non-empty and contain no control characters"
                    .to_owned(),
            ));
        }

        if !matches!(
            self.terminal_cursor_style.as_str(),
            "block" | "bar" | "underline"
        ) {
            return Err(SettingsError::Invalid(
                "terminal_cursor_style must be block, bar, or underline".to_owned(),
            ));
        }

        if !self.default_working_directory.is_empty()
            && !Path::new(&self.default_working_directory).is_dir()
        {
            return Err(SettingsError::Invalid(
                "default_working_directory must be an existing directory or empty".to_owned(),
            ));
        }

        if self.history_size == 0 || self.history_size > 10_000 {
            return Err(SettingsError::Invalid(
                "history_size must be between 1 and 10000".to_owned(),
            ));
        }

        if !matches!(self.accent_color.as_str(), "cyan" | "blue" | "violet") {
            return Err(SettingsError::Invalid(
                "accent_color must be cyan, blue, or violet".to_owned(),
            ));
        }

        Ok(())
    }

    /// Reads settings from `config_dir`, falling back to defaults when the file
    /// is missing. A malformed file is reported instead of being replaced.
    ///
    /// # Errors
    ///
    /// Returns a [`SettingsError`] when the file cannot be read or parsed.
    pub fn load(config_dir: &Path) -> Result<Self, SettingsError> {
        let path = Self::path(config_dir);

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)?;
        let settings: Self = toml::from_str(&contents)?;
        settings.validate()?;
        Ok(settings)
    }

    /// Writes validated settings to `config_dir`.
    ///
    /// # Errors
    ///
    /// Returns a [`SettingsError`] when validation, serialization, or writing
    /// fails.
    pub fn save(config_dir: &Path, settings: &Self) -> Result<(), SettingsError> {
        settings.validate()?;

        fs::create_dir_all(config_dir)?;
        let contents = toml::to_string(settings)?;
        let path = Self::path(config_dir);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, contents)?;
        Ok(())
    }

    /// Absolute path of the settings file inside `config_dir`.
    #[must_use]
    pub fn path(config_dir: &Path) -> PathBuf {
        config_dir.join(SETTINGS_FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        DesktopSettings::default()
            .validate()
            .expect("default settings should validate");
    }

    #[test]
    fn invalid_theme_is_rejected() {
        let settings = DesktopSettings {
            theme: "neon".to_owned(),
            ..DesktopSettings::default()
        };

        let error = settings.validate().expect_err("theme should be validated");
        assert!(error.to_string().contains("theme"), "{error}");
    }

    #[test]
    fn missing_history_size_is_rejected() {
        let settings = DesktopSettings {
            history_size: 0,
            ..DesktopSettings::default()
        };

        let error = settings
            .validate()
            .expect_err("history size should be validated");
        assert!(error.to_string().contains("history_size"), "{error}");
    }

    #[test]
    fn settings_round_trip_through_toml() {
        let directory = std::env::temp_dir().join("kernext-desktop-settings-test");
        let _ = std::fs::remove_dir_all(&directory);
        let original = DesktopSettings::default();

        DesktopSettings::save(&directory, &original).expect("settings should save");
        let loaded = DesktopSettings::load(&directory).expect("settings should load");

        assert_eq!(original, loaded);
    }
}
