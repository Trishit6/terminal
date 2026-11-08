//! Configuration model and persistence for the custom shell.
//!
//! The CLI loads the user configuration from [`Config::default_path`] during
//! startup and falls back to built-in defaults when the file is missing or
//! invalid.

use serde::{Deserialize, Serialize};
use shell_core::SHELL_NAME;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Default prompt shown before an interactive command.
pub const DEFAULT_PROMPT: &str = "kernext ~ $ ";

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
    /// Whether command history persistence is enabled.
    pub history_enabled: bool,
    /// Path to the history file (relative to config dir if not absolute).
    pub history_file: String,
    /// Whether to filter sensitive-looking commands from history.
    pub history_filter_secrets: bool,
    /// Whether inline command suggestions are enabled.
    pub suggestions_enabled: bool,
    /// Whether to show hidden files in completion.
    pub completion_hidden_files: bool,
    /// Whether the interactive startup sequence may animate.
    pub startup_animation: bool,
    /// Whether to suppress the startup banner and checklist entirely.
    pub quiet_startup: bool,
    /// Startup animation pace: `slow`, `normal`, or `fast`.
    pub animation_speed: String,
    /// Primary brand color (logo, prompt accent). Named ANSI, `colorN`, or hex.
    pub accent_color: String,
    /// Secondary gradient color used by the animated wordmark.
    pub secondary_color: String,
    /// Highlight gradient color used by the animated wordmark.
    pub highlight_color: String,
    /// Color used for successful checklist items and status text.
    pub success_color: String,
    /// Color used for failures and error messages.
    pub error_color: String,
    /// Color applied to the interactive prompt.
    pub prompt_color: String,
    /// Terminal font family. Only used when `apply_terminal_font` is enabled.
    pub terminal_font: String,
    /// Opt in to emitting OSC 50 font changes. Ignored by many terminals.
    pub apply_terminal_font: bool,
}

/// Default history file name.
pub const DEFAULT_HISTORY_FILE: &str = "history.txt";

impl Default for Config {
    fn default() -> Self {
        Self {
            prompt: DEFAULT_PROMPT.to_owned(),
            history_size: DEFAULT_HISTORY_SIZE,
            color: true,
            history_enabled: true,
            history_file: DEFAULT_HISTORY_FILE.to_owned(),
            history_filter_secrets: true,
            suggestions_enabled: true,
            completion_hidden_files: false,
            startup_animation: true,
            quiet_startup: false,
            animation_speed: "normal".to_owned(),
            accent_color: "bright-cyan".to_owned(),
            secondary_color: "bright-blue".to_owned(),
            highlight_color: "bright-magenta".to_owned(),
            success_color: "bright-green".to_owned(),
            error_color: "bright-red".to_owned(),
            prompt_color: "bright-cyan".to_owned(),
            terminal_font: String::new(),
            apply_terminal_font: false,
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

    /// Resolves the conventional per-user history file path on this platform.
    ///
    /// Uses the same base directory as `default_path()` but places the history
    /// file alongside the config file. Returns `None` if the base directory
    /// cannot be determined.
    #[must_use]
    pub fn default_history_path() -> Option<PathBuf> {
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

        Some(base.join(SHELL_NAME).join(DEFAULT_HISTORY_FILE))
    }

    /// Returns the effective history file path for this configuration.
    ///
    /// If `history_file` is an absolute path, it is used directly.
    /// Otherwise, it is resolved relative to the config directory (same as
    /// `default_history_path()`'s parent).
    #[must_use]
    pub fn history_path(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.history_file);
        if path.is_absolute() {
            Some(path)
        } else {
            Self::default_history_path().map(|p| p.parent().unwrap().join(&self.history_file))
        }
    }

    /// Checks values that would be invalid at runtime.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.history_size == 0 {
            return Err(ConfigError::Invalid {
                message: "history_size must be greater than zero".to_owned(),
            });
        }

        if self.history_file.trim().is_empty() {
            return Err(ConfigError::Invalid {
                message: "history_file must not be empty".to_owned(),
            });
        }

        for (name, value) in [
            ("accent_color", &self.accent_color),
            ("secondary_color", &self.secondary_color),
            ("highlight_color", &self.highlight_color),
            ("success_color", &self.success_color),
            ("error_color", &self.error_color),
            ("prompt_color", &self.prompt_color),
        ] {
            parse_color(value).map_err(|reason| ConfigError::Invalid {
                message: format!("{name}: {reason}"),
            })?;
        }

        if self
            .terminal_font
            .chars()
            .any(|character| character.is_control())
        {
            return Err(ConfigError::Invalid {
                message: "terminal_font must not contain control characters".to_owned(),
            });
        }

        if !matches!(
            self.animation_speed.trim().to_ascii_lowercase().as_str(),
            "slow" | "normal" | "fast"
        ) {
            return Err(ConfigError::Invalid {
                message: format!(
                    "animation_speed must be slow, normal, or fast, not '{}'",
                    self.animation_speed
                ),
            });
        }

        if self.apply_terminal_font && self.terminal_font.trim().is_empty() {
            return Err(ConfigError::Invalid {
                message: "terminal_font must be set when apply_terminal_font is true".to_owned(),
            });
        }

        Ok(())
    }

    /// Returns the parsed theme, falling back to the defaults for any value
    /// that was changed after validation.
    #[must_use]
    pub fn theme(&self) -> Theme {
        Theme::from_config(self)
    }
}

/// A terminal color parsed from configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiColor {
    /// The terminal's default foreground color.
    Default,
    /// One of the eight standard ANSI colors.
    Basic(u8),
    /// One of the eight bright ANSI colors.
    Bright(u8),
    /// An xterm 256-color palette entry.
    Fixed(u8),
    /// A 24-bit color.
    Rgb(u8, u8, u8),
}

impl AnsiColor {
    /// ANSI foreground sequence for this color.
    #[must_use]
    pub fn foreground(self) -> String {
        match self {
            Self::Default => "\x1b[39m".to_owned(),
            Self::Basic(index) => format!("\x1b[{}m", 30 + index),
            Self::Bright(index) => format!("\x1b[{}m", 90 + index),
            Self::Fixed(index) => format!("\x1b[38;5;{index}m"),
            Self::Rgb(red, green, blue) => format!("\x1b[38;2;{red};{green};{blue}m"),
        }
    }

    /// Approximate RGB representation used for smooth gradients.
    #[must_use]
    pub fn to_rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Default => (204, 204, 204),
            Self::Basic(index) => basic_rgb(index, false),
            Self::Bright(index) => basic_rgb(index, true),
            Self::Fixed(index) => fixed_rgb(index),
            Self::Rgb(red, green, blue) => (red, green, blue),
        }
    }
}

fn basic_rgb(index: u8, bright: bool) -> (u8, u8, u8) {
    const NORMAL: [(u8, u8, u8); 8] = [
        (0, 0, 0),
        (205, 49, 49),
        (13, 188, 121),
        (229, 229, 16),
        (36, 114, 200),
        (188, 63, 188),
        (17, 168, 205),
        (229, 229, 229),
    ];
    const BRIGHT: [(u8, u8, u8); 8] = [
        (102, 102, 102),
        (241, 76, 76),
        (35, 209, 139),
        (245, 245, 67),
        (59, 142, 234),
        (214, 112, 214),
        (41, 184, 219),
        (255, 255, 255),
    ];

    let table = if bright { &BRIGHT } else { &NORMAL };
    table[usize::from(index.min(7))]
}

fn fixed_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        0..=15 => basic_rgb(index % 8, index >= 8),
        16..=231 => {
            let value = index - 16;
            let red = value / 36;
            let green = (value % 36) / 6;
            let blue = value % 6;
            let component = |level: u8| if level == 0 { 0 } else { 55 + level * 40 };
            (component(red), component(green), component(blue))
        }
        _ => {
            let level = 8 + (index - 232) * 10;
            (level, level, level)
        }
    }
}

/// Parsed visual theme used by startup and prompt rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Primary brand color.
    pub accent: AnsiColor,
    /// Secondary gradient color.
    pub secondary: AnsiColor,
    /// Highlight gradient color.
    pub highlight: AnsiColor,
    /// Success indicator color.
    pub success: AnsiColor,
    /// Failure indicator color.
    pub error: AnsiColor,
    /// Interactive prompt color.
    pub prompt: AnsiColor,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: AnsiColor::Bright(6),
            secondary: AnsiColor::Bright(4),
            highlight: AnsiColor::Bright(5),
            success: AnsiColor::Bright(2),
            error: AnsiColor::Bright(1),
            prompt: AnsiColor::Bright(6),
        }
    }
}

impl Theme {
    /// Builds a theme from configuration, using defaults for invalid values.
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        let defaults = Self::default();

        Self {
            accent: parse_color(&config.accent_color).unwrap_or(defaults.accent),
            secondary: parse_color(&config.secondary_color).unwrap_or(defaults.secondary),
            highlight: parse_color(&config.highlight_color).unwrap_or(defaults.highlight),
            success: parse_color(&config.success_color).unwrap_or(defaults.success),
            error: parse_color(&config.error_color).unwrap_or(defaults.error),
            prompt: parse_color(&config.prompt_color).unwrap_or(defaults.prompt),
        }
    }

    /// Smoothly interpolates around the accent → secondary → highlight loop.
    ///
    /// `position` is measured in gradient segments; callers usually pass a
    /// time-derived value plus a row offset.
    #[must_use]
    pub fn gradient_rgb(self, position: f64) -> (u8, u8, u8) {
        let colors = [
            self.accent.to_rgb(),
            self.secondary.to_rgb(),
            self.highlight.to_rgb(),
        ];
        let scaled = position.rem_euclid(3.0);
        let index = scaled.floor() as usize;
        let fraction = scaled.fract();
        let start = colors[index % 3];
        let end = colors[(index + 1) % 3];

        (
            lerp(start.0, end.0, fraction),
            lerp(start.1, end.1, fraction),
            lerp(start.2, end.2, fraction),
        )
    }
}

fn lerp(start: u8, end: u8, fraction: f64) -> u8 {
    let value = f64::from(start) + (f64::from(end) - f64::from(start)) * fraction;
    value.round().clamp(0.0, 255.0) as u8
}

/// Parses a color from configuration.
///
/// Accepted forms are ANSI names (`cyan`, `bright-cyan`), `colorN` or a bare
/// `0`–`255` palette index, and `#RGB`/`#RRGGBB` hex values. `default` and an
/// empty string mean the terminal's default foreground.
pub fn parse_color(value: &str) -> Result<AnsiColor, String> {
    let value = value.trim();

    if value.is_empty()
        || value.eq_ignore_ascii_case("default")
        || value.eq_ignore_ascii_case("auto")
    {
        return Ok(AnsiColor::Default);
    }

    let lower = value.to_ascii_lowercase();

    if let Some(hex) = lower.strip_prefix('#') {
        return parse_hex(hex);
    }

    if let Some(index) = lower.strip_prefix("color") {
        return parse_index(index);
    }

    if lower.chars().all(|character| character.is_ascii_digit()) {
        return parse_index(&lower);
    }

    let (bright, name) = match lower
        .strip_prefix("bright-")
        .or_else(|| lower.strip_prefix("light-"))
    {
        Some(name) => (true, name),
        None => (false, lower.as_str()),
    };

    let index = match name {
        "black" => 0,
        "red" => 1,
        "green" => 2,
        "yellow" => 3,
        "blue" => 4,
        "magenta" | "violet" | "purple" => 5,
        "cyan" | "teal" => 6,
        "white" => 7,
        "gray" | "grey" => {
            return Ok(AnsiColor::Bright(0));
        }
        _ => {
            return Err(format!(
                "unknown color '{value}' (use an ANSI name, colorN, or #RRGGBB)"
            ));
        }
    };

    Ok(if bright {
        AnsiColor::Bright(index)
    } else {
        AnsiColor::Basic(index)
    })
}

fn parse_index(value: &str) -> Result<AnsiColor, String> {
    let index = value
        .parse::<u8>()
        .map_err(|_| format!("invalid palette index '{value}'"))?;
    Ok(AnsiColor::Fixed(index))
}

fn parse_hex(value: &str) -> Result<AnsiColor, String> {
    let expanded = match value.len() {
        3 => value
            .chars()
            .flat_map(|character| [character, character])
            .collect::<String>(),
        6 => value.to_owned(),
        _ => {
            return Err(format!(
                "invalid hex color '#{value}' (expected #RGB or #RRGGBB)"
            ));
        }
    };

    if !expanded
        .chars()
        .all(|character| character.is_ascii_hexdigit())
    {
        return Err(format!("invalid hex color '#{value}'"));
    }

    let red =
        u8::from_str_radix(&expanded[0..2], 16).map_err(|_| "invalid hex color".to_owned())?;
    let green =
        u8::from_str_radix(&expanded[2..4], 16).map_err(|_| "invalid hex color".to_owned())?;
    let blue =
        u8::from_str_radix(&expanded[4..6], 16).map_err(|_| "invalid hex color".to_owned())?;

    Ok(AnsiColor::Rgb(red, green, blue))
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
        assert!(config.color);
        assert!(config.history_enabled);
        assert_eq!(config.history_file, DEFAULT_HISTORY_FILE);
        assert!(config.history_filter_secrets);
        assert!(config.suggestions_enabled);
        assert!(!config.completion_hidden_files);
        assert!(config.startup_animation);
        assert!(!config.quiet_startup);
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing_fields() {
        let config = Config::from_toml("color = false\n").expect("partial config should parse");

        assert_eq!(config.prompt, DEFAULT_PROMPT);
        assert_eq!(config.history_size, DEFAULT_HISTORY_SIZE);
        assert!(!config.color);
        assert!(config.history_enabled);
        assert_eq!(config.history_file, DEFAULT_HISTORY_FILE);
        assert!(config.history_filter_secrets);
        assert!(config.suggestions_enabled);
        assert!(!config.completion_hidden_files);
        assert!(config.startup_animation);
        assert!(!config.quiet_startup);
    }

    #[test]
    fn configuration_round_trips_through_toml() {
        let original = Config {
            prompt: "dev> ".to_owned(),
            history_size: 42,
            color: false,
            history_enabled: false,
            history_file: "custom_history.txt".to_owned(),
            history_filter_secrets: false,
            suggestions_enabled: false,
            completion_hidden_files: true,
            startup_animation: false,
            quiet_startup: true,
            ..Config::default()
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
    fn empty_history_file_is_rejected() {
        let error = Config::from_toml("history_file = \"\"\n")
            .expect_err("empty history_file should be invalid");

        assert!(matches!(error, ConfigError::Invalid { .. }));
    }

    #[test]
    fn history_path_resolves_relative_to_config_dir() {
        let config = Config::default();
        let path = config.history_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.ends_with("kernext/history.txt"));
    }

    #[test]
    fn save_and_load_round_trip_on_disk() {
        let directory = temporary_directory();
        let path = directory.join("nested").join("config.toml");
        let original = Config {
            prompt: "test> ".to_owned(),
            history_size: 7,
            color: false,
            history_enabled: false,
            history_file: "custom_history.txt".to_owned(),
            history_filter_secrets: false,
            suggestions_enabled: false,
            completion_hidden_files: true,
            startup_animation: false,
            quiet_startup: true,
            ..Config::default()
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

    #[test]
    fn named_hex_and_palette_colors_parse() {
        assert_eq!(parse_color("cyan"), Ok(AnsiColor::Basic(6)));
        assert_eq!(parse_color("bright-magenta"), Ok(AnsiColor::Bright(5)));
        assert_eq!(parse_color("color75"), Ok(AnsiColor::Fixed(75)));
        assert_eq!(parse_color("#0af"), Ok(AnsiColor::Rgb(0, 170, 255)));
        assert_eq!(parse_color("#00aaff"), Ok(AnsiColor::Rgb(0, 170, 255)));
        assert_eq!(parse_color(""), Ok(AnsiColor::Default));
    }

    #[test]
    fn invalid_colors_are_rejected() {
        assert!(Config::from_toml("accent_color = \"not-a-color\"\n").is_err());
        assert!(Config::from_toml("accent_color = \"#12\"\n").is_err());
    }

    #[test]
    fn applying_a_terminal_font_requires_a_font() {
        assert!(Config::from_toml("apply_terminal_font = true\n").is_err());
        assert!(
            Config::from_toml("apply_terminal_font = true\nterminal_font = \"JetBrains Mono\"\n")
                .is_ok()
        );
    }

    #[test]
    fn animation_speed_accepts_known_paces_and_rejects_others() {
        assert_eq!(Config::default().animation_speed, "normal");

        for speed in ["slow", "normal", "fast", "FAST"] {
            let source = format!("animation_speed = \"{speed}\"\n");
            assert!(
                Config::from_toml(&source).is_ok(),
                "{speed} should be accepted"
            );
        }

        let error = Config::from_toml("animation_speed = \"turbo\"\n")
            .expect_err("unknown animation speed should be rejected");

        assert!(matches!(error, ConfigError::Invalid { .. }));
    }

    #[test]
    fn theme_uses_the_configured_palette() {
        let config = Config::from_toml(
            "accent_color = \"#00aaff\"\n\
             secondary_color = \"magenta\"\n\
             highlight_color = \"bright-green\"\n\
             prompt_color = \"color45\"\n",
        )
        .expect("theme config should parse");

        let theme = config.theme();

        assert_eq!(theme.accent, AnsiColor::Rgb(0, 170, 255));
        assert_eq!(theme.secondary, AnsiColor::Basic(5));
        assert_eq!(theme.highlight, AnsiColor::Bright(2));
        assert_eq!(theme.prompt, AnsiColor::Fixed(45));
    }
}
