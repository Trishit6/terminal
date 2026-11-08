//! KERNEXT startup presentation.
//!
//! The startup sequence is deliberately separate from the execution engine.
//! [`Startup`] only renders text and terminal-control sequences; it never
//! parses, evaluates, or launches commands. Every checklist item is emitted by
//! `cli::run` at the point where the corresponding subsystem has actually
//! finished initializing.
//!
//! Output modes:
//!
//! - [`StartupMode::Animated`] is used for an interactive terminal with
//!   animation enabled. It hides the cursor briefly and redraws the wordmark in
//!   place while a palette-driven gradient sweeps across it.
//! - [`StartupMode::Plain`] prints the same information without animation and
//!   without ANSI styling. It is used for piped/redirected output and when
//!   `NO_COLOR`, `TERM=dumb`, or the animation configuration is active.
//! - [`StartupMode::Quiet`] prints nothing and goes straight to the prompt.

use shell_config::{Config, Theme};
use shell_core::{MOTTO, PRODUCT_NAME, TAGLINE, VALUES, VERSION};
use shell_runtime::Session;
use std::io::{self, Write};
use std::time::{Duration, Instant};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const HIDE_CURSOR: &str = "\x1b[?25l";
const SHOW_CURSOR: &str = "\x1b[?25h";

/// Delay between in-place wordmark frames. Kept intentionally short: the
/// animation is a polish layer, not an artificial boot delay.
const ANIMATION_FRAME: Duration = Duration::from_millis(18);

/// Delay between emblem reveal rows.
const EMBLEM_FRAME: Duration = Duration::from_millis(24);

/// Number of in-place gradient frames drawn while the wordmark is revealed.
const ANIMATION_FRAMES: usize = 16;

/// Cinematic K emblem shown above the wordmark on wide terminals.
///
/// The metallic K monogram sits inside a luminous orbital ring, drawn entirely
/// with box-drawing and block glyphs so the mark stays text-only, inherits the
/// terminal's font, and keeps a fixed display width on every platform. Every
/// row is exactly 58 columns; a unit test enforces that so the ring can never
/// drift out of alignment.
const EMBLEM_FULL: [&str; 9] = [
    r"          ╭────────────────────────────────────╮          ",
    r"      ╭───╯                                    ╰───╮      ",
    r"  ╱          ◇           ██╗  ██╗           ◇          ╲  ",
    r"│                        ██║ ██╔╝                        │",
    r"│                        █████╔╝                         │",
    r"│                        ██╔═██╗                         │",
    r"  ╲          ◇           ██║  ██╗           ◇          ╱  ",
    r"      ╰───╮                                    ╭───╯      ",
    r"          ╰────────────────────────────────────╯          ",
];

/// Compact three-row core badge for mid-width terminals. The full ring does not
/// fit below 76 columns, so the badge carries the product name instead.
const EMBLEM_COMPACT: [&str; 3] = [
    r"  ╭────────────────────────────────╮  ",
    r"  │        K E R N E X T           │  ",
    r"  ╰────────────────────────────────╯  ",
];

/// Full six-row block wordmark, used when the terminal is wide enough.
const LOGO_FULL: [&str; 6] = [
    r"██╗  ██╗ ███████╗ ██████╗  ███╗   ██╗ ███████╗ ██╗  ██╗ ████████╗",
    r"██║ ██╔╝ ██╔════╝ ██╔══██╗ ████╗  ██║ ██╔════╝ ╚██╗██╔╝ ╚══██╔══╝",
    r"█████╔╝  █████╗   ██████╔╝ ██╔██╗ ██║ █████╗    ╚███╔╝     ██║",
    r"██╔═██╗  ██╔══╝   ██╔══██╗ ██║╚██╗██║ ██╔══╝    ██╔██╗     ██║",
    r"██║  ██╗ ███████╗ ██║  ██║ ██║ ╚████║ ███████╗ ██╔╝ ██╗    ██║",
    r"╚═╝  ╚═╝ ╚══════╝ ╚═╝  ╚═╝ ╚═╝  ╚═══╝ ╚══════╝ ╚═╝  ╚═╝    ╚═╝",
];

/// Compact three-row wordmark for narrow terminals.
const LOGO_COMPACT: [&str; 3] = [
    "╦╔═╔═╗╦═╗╔╗╔╔═╗═╗╔╦",
    "╠╩╗║╣ ╠╦╝║║║║╣  ╠╩╗║",
    "╩ ╩╚═╝╩╚═╝╚╝╚═╝═╝ ╩",
];

/// Columns of gradient travel per rendered character. Small enough to read as
/// a smooth sweep, large enough to stay visible on wide wordmarks.
const GRADIENT_COLUMN_STEP: f64 = 0.045;

/// Extra gradient travel between rows so the wordmark reads as one diagonal
/// wave rather than identical stripes.
const GRADIENT_ROW_STEP: f64 = 0.28;

/// Returns the logo rows for a terminal of `width` columns.
///
/// An empty slice means the terminal is too narrow for a block wordmark and the
/// caller should fall back to plain `KERNEXT` text.
fn logo_rows(width: usize) -> &'static [&'static str] {
    if width >= 80 {
        &LOGO_FULL
    } else if width >= 24 {
        &LOGO_COMPACT
    } else {
        &[]
    }
}

/// Returns the emblem rows for a terminal of `width` columns.
///
/// The full orbital ring needs 58 columns, so it only appears at 76 or wider;
/// narrower terminals get the compact badge, and very narrow terminals get the
/// wordmark alone.
fn emblem_rows(width: usize) -> &'static [&'static str] {
    if width >= 76 {
        &EMBLEM_FULL
    } else if width >= 44 {
        &EMBLEM_COMPACT
    } else {
        &[]
    }
}

/// Maps the configured `animation_speed` to the delay between reveal steps.
///
/// Unknown values fall back to `normal`; configuration validation rejects them
/// first, so this only protects against a value changed after loading.
fn frame_delay(speed: &str) -> Duration {
    match speed.to_ascii_lowercase().as_str() {
        "slow" => ANIMATION_FRAME.mul_f64(1.6),
        "fast" => ANIMATION_FRAME.mul_f64(0.5),
        _ => ANIMATION_FRAME,
    }
}

/// Maps the configured `animation_speed` to the emblem reveal delay.
fn emblem_frame_delay(speed: &str) -> Duration {
    match speed.to_ascii_lowercase().as_str() {
        "slow" => EMBLEM_FRAME.mul_f64(1.6),
        "fast" => EMBLEM_FRAME.mul_f64(0.5),
        _ => EMBLEM_FRAME,
    }
}

/// How the startup sequence should present itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupMode {
    /// Interactive reveal with ANSI styling.
    Animated,
    /// Static, uncolored output.
    Plain,
    /// No startup output at all.
    Quiet,
}

/// Resolves the startup mode from configuration and terminal state.
#[must_use]
pub fn resolve_mode(
    config: &Config,
    quiet_flag: bool,
    no_animation_flag: bool,
    interactive: bool,
) -> StartupMode {
    if config.quiet_startup || quiet_flag {
        StartupMode::Quiet
    } else if !interactive || !config.startup_animation || no_animation_flag {
        StartupMode::Plain
    } else {
        StartupMode::Animated
    }
}

/// Returns `true` when the environment permits ANSI styling.
///
/// `NO_COLOR` always wins. On Windows, ANSI is only enabled for terminals that
/// advertise virtual-terminal support so legacy consoles do not receive escape
/// bytes they cannot render.
#[must_use]
pub fn supports_color() -> bool {
    let term = std::env::var("TERM").ok();

    color_allowed(
        std::env::var_os("NO_COLOR").is_some(),
        term.as_deref(),
        platform_supports_ansi(term.as_deref()),
    )
}

#[cfg(windows)]
fn platform_supports_ansi(term: Option<&str>) -> bool {
    // crossterm enables virtual-terminal processing when the console supports
    // it. This covers Windows Terminal, ConHost on modern Windows, and
    // ANSI-capable third-party terminals.
    crossterm::ansi_support::supports_ansi()
        || std::env::var_os("WT_SESSION").is_some()
        || std::env::var_os("ANSICON").is_some()
        || term.is_some()
}

#[cfg(not(windows))]
fn platform_supports_ansi(_term: Option<&str>) -> bool {
    true
}

/// Pure color-gating policy used by [`supports_color`].
///
/// Keeping the policy separate from environment access makes `NO_COLOR` and
/// `TERM=dumb` behavior testable without mutating process-wide state.
fn color_allowed(no_color: bool, term: Option<&str>, windows_terminal: bool) -> bool {
    if no_color {
        return false;
    }

    if term.is_some_and(|term| term.eq_ignore_ascii_case("dumb")) {
        return false;
    }

    if cfg!(windows) {
        windows_terminal
    } else {
        true
    }
}

/// Returns the terminal width, defaulting to 80 columns when it is unavailable
/// or reported as zero by the terminal driver.
#[must_use]
pub fn terminal_width() -> usize {
    crossterm::terminal::size()
        .ok()
        .map(|(width, _)| usize::from(width))
        .filter(|width| *width > 0)
        .unwrap_or(80)
}

/// Renders the cinematic emblem for a terminal of `width` columns.
#[must_use]
pub fn render_emblem(width: usize, color: bool) -> String {
    render_emblem_themed(width, color, &Theme::default(), 0.0)
}

/// Renders the emblem with a time-derived gradient.
#[must_use]
pub fn render_emblem_themed(width: usize, color: bool, theme: &Theme, phase: f64) -> String {
    emblem_rows(width)
        .iter()
        .enumerate()
        .map(|(row, line)| {
            gradient_line(line, theme, color, phase + row as f64 * GRADIENT_ROW_STEP)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Renders the KERNEXT logo for a terminal of `width` columns using the default
/// theme and no gradient offset.
#[must_use]
pub fn render_logo(width: usize, color: bool) -> String {
    render_logo_themed(width, color, &Theme::default(), 0.0)
}

/// Renders the logo with a time-derived gradient.
///
/// Every character receives its own 24-bit color, so `phase` sweeps a smooth
/// highlight across the wordmark. Passing a phase derived from
/// [`Instant::elapsed`] produces the continuous color-changing animation.
#[must_use]
pub fn render_logo_themed(width: usize, color: bool, theme: &Theme, phase: f64) -> String {
    let rows = logo_rows(width);

    if rows.is_empty() {
        return gradient_line(PRODUCT_NAME, theme, color, phase);
    }

    rows.iter()
        .enumerate()
        .map(|(row, line)| {
            gradient_line(line, theme, color, phase + row as f64 * GRADIENT_ROW_STEP)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Renders the logo, wordmark, tagline, and values block.
#[must_use]
pub fn render_intro(width: usize, color: bool) -> String {
    render_intro_themed(width, color, &Theme::default())
}

/// Renders the branded introduction with an explicit theme.
#[must_use]
pub fn render_intro_themed(width: usize, color: bool, theme: &Theme) -> String {
    let emblem = render_emblem_themed(width, color, theme, 0.0);
    let logo = render_logo_themed(width, color, theme, 0.0);
    let emblem = if emblem.is_empty() {
        String::new()
    } else {
        format!("{emblem}\n\n")
    };

    format!(
        "\n{emblem}{logo}\n\n{name}  {version}\n{tagline}\n{values}\n",
        name = style(
            PRODUCT_NAME,
            &format!("{BOLD}{}", theme.accent.foreground()),
            color
        ),
        version = style(&format!("[v{VERSION}]"), DIM, color),
        tagline = gradient_line(&TAGLINE.to_uppercase(), theme, color, 0.4),
        values = gradient_line(VALUES, theme, color, 1.1),
    )
}

/// Renders the one-line runtime summary shown after the checklist.
#[must_use]
pub fn render_session_summary(session: &Session, color: bool) -> String {
    render_session_summary_themed(session, color, &Theme::default())
}

/// Renders the runtime summary with an explicit theme.
#[must_use]
pub fn render_session_summary_themed(session: &Session, color: bool, theme: &Theme) -> String {
    let summary = format!(
        "{PRODUCT_NAME} v{}  ·  {} {}  ·  {}",
        session.version(),
        session.platform().display_name(),
        session.platform().arch(),
        session.working_directory().display(),
    );

    style(
        &summary,
        &format!("{DIM}{}", theme.secondary.foreground()),
        color,
    )
}

/// Renders a completed checklist item.
#[must_use]
pub fn render_stage_ok(label: &str, color: bool) -> String {
    render_stage_ok_themed(label, color, &Theme::default())
}

/// Renders a completed checklist item with an explicit theme.
#[must_use]
pub fn render_stage_ok_themed(label: &str, color: bool, theme: &Theme) -> String {
    format!(
        "{} {label}",
        style("[OK]", &theme.success.foreground(), color)
    )
}

/// Renders a failed checklist item.
#[must_use]
pub fn render_stage_failed(label: &str, error: &str, color: bool) -> String {
    render_stage_failed_themed(label, error, color, &Theme::default())
}

/// Renders a failed checklist item with an explicit theme.
#[must_use]
pub fn render_stage_failed_themed(label: &str, error: &str, color: bool, theme: &Theme) -> String {
    format!(
        "{} {label}: {error}",
        style("[FAIL]", &theme.error.foreground(), color)
    )
}

/// Renders the welcome message shown immediately before the first prompt.
#[must_use]
pub fn render_welcome(color: bool) -> String {
    render_welcome_themed(color, &Theme::default())
}

/// Renders the welcome message with an explicit theme.
#[must_use]
pub fn render_welcome_themed(color: bool, theme: &Theme) -> String {
    format!(
        "Welcome to {name}.\n{motto}\n\nType 'help' to explore commands.\n",
        name = style(
            PRODUCT_NAME,
            &format!("{BOLD}{}", theme.accent.foreground()),
            color
        ),
        motto = style(
            MOTTO,
            &format!("{DIM}{}", theme.highlight.foreground()),
            color
        ),
    )
}

/// Returns the OSC 50 sequence that asks the terminal to switch font families.
///
/// OSC 50 is a presentation hint: many terminals ignore it, and the shell
/// never depends on it. The caller must opt in through configuration.
#[must_use]
pub fn terminal_font_sequence(font: &str) -> String {
    format!("\x1b]50;{font}\x1b\\")
}

fn style(text: &str, code: &str, enabled: bool) -> String {
    if enabled {
        format!("{code}{text}{RESET}")
    } else {
        text.to_owned()
    }
}

/// Reveals a line from left to right without ANSI color.
///
/// This keeps the startup animation visible in terminals where color is
/// disabled or unavailable; the final frame always contains the complete line.
fn reveal_line(text: &str, progress: f64) -> String {
    let characters: Vec<char> = text.chars().collect();
    let visible = ((characters.len() as f64) * progress.clamp(0.0, 1.0)).ceil() as usize;

    characters
        .into_iter()
        .enumerate()
        .map(|(index, character)| if index < visible { character } else { ' ' })
        .collect()
}

fn gradient_line(text: &str, theme: &Theme, color: bool, phase: f64) -> String {
    if !color {
        return text.to_owned();
    }

    let mut output = String::with_capacity(text.len().saturating_mul(20));
    for (index, character) in text.chars().enumerate() {
        let (red, green, blue) = theme.gradient_rgb(phase + index as f64 * GRADIENT_COLUMN_STEP);
        output.push_str(&format!("\x1b[38;2;{red};{green};{blue}m"));
        output.push(character);
    }
    output.push_str(RESET);
    output
}

impl<W: Write> Drop for Startup<W> {
    fn drop(&mut self) {
        // A dropped startup renderer must never leave the terminal with a
        // hidden cursor or an active color attribute, even after an early
        // initialization error.
        if self.mode == StartupMode::Animated {
            let _ = self.writer.write_all(SHOW_CURSOR.as_bytes());
        }
        if self.color || self.mode == StartupMode::Animated {
            let _ = self.writer.write_all(RESET.as_bytes());
        }
        let _ = self.writer.flush();
    }
}

/// Writes the startup sequence to a generic writer.
///
/// The writer is usually `stdout`, but keeping it generic makes the animation
/// and its terminal-control behaviour testable without a real terminal.
pub struct Startup<W: Write> {
    writer: W,
    mode: StartupMode,
    color: bool,
    width: usize,
    theme: Theme,
    frame: Duration,
    emblem_frame: Duration,
}

impl<W: Write> Startup<W> {
    /// Creates a startup renderer with the default theme and `normal` pace.
    pub fn new(writer: W, mode: StartupMode, color: bool, width: usize) -> Self {
        Self {
            writer,
            mode,
            color,
            width,
            theme: Theme::default(),
            frame: frame_delay("normal"),
            emblem_frame: emblem_frame_delay("normal"),
        }
    }

    /// Replaces the default theme with the user's configured palette.
    #[must_use]
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Applies the configured `slow`, `normal`, or `fast` animation pace.
    #[must_use]
    pub fn with_animation_speed(mut self, speed: &str) -> Self {
        self.frame = frame_delay(speed);
        self.emblem_frame = emblem_frame_delay(speed);
        self
    }

    /// Writes the logo and tagline block, then reports an actual runtime summary.
    pub fn intro(&mut self) -> io::Result<()> {
        if self.mode == StartupMode::Quiet {
            return Ok(());
        }

        if self.mode == StartupMode::Animated {
            self.writer.write_all(HIDE_CURSOR.as_bytes())?;
        }

        if self.mode == StartupMode::Animated {
            self.animated_emblem()?;
            self.animated_logo()?;
        } else {
            let emblem = render_emblem_themed(self.width, self.color, &self.theme, 0.0);
            if !emblem.is_empty() {
                for line in emblem.lines() {
                    writeln!(self.writer, "{line}")?;
                }
                writeln!(self.writer)?;
            }

            for line in render_logo_themed(self.width, self.color, &self.theme, 0.0).lines() {
                writeln!(self.writer, "{line}")?;
            }
        }

        writeln!(self.writer)?;
        writeln!(
            self.writer,
            "{}  {}",
            style(
                PRODUCT_NAME,
                &format!("{BOLD}{}", self.theme.accent.foreground()),
                self.color
            ),
            style(&format!("[v{VERSION}]"), DIM, self.color)
        )?;
        writeln!(
            self.writer,
            "{}",
            gradient_line(&TAGLINE.to_uppercase(), &self.theme, self.color, 0.4)
        )?;
        writeln!(
            self.writer,
            "{}",
            gradient_line(VALUES, &self.theme, self.color, 1.1)
        )?;
        writeln!(self.writer)?;
        self.writer.flush()
    }

    /// Reveals the emblem from top to bottom.
    ///
    /// Each row is a real line of the final artwork; nothing is invented and no
    /// initialization work is simulated. The reveal is bounded to a handful of
    /// short frames so the shell becomes usable immediately afterwards.
    fn animated_emblem(&mut self) -> io::Result<()> {
        let emblem = render_emblem_themed(self.width, self.color, &self.theme, 0.0);

        if emblem.is_empty() {
            return Ok(());
        }

        let rows: Vec<&str> = emblem.lines().collect();

        for (index, line) in rows.iter().enumerate() {
            writeln!(self.writer, "{line}")?;
            self.writer.flush()?;

            if index + 1 < rows.len() {
                std::thread::sleep(self.emblem_frame);
            }
        }

        writeln!(self.writer)?;
        Ok(())
    }

    /// Draws the wordmark in place while the gradient sweeps across it.
    ///
    /// This is a presentation-only animation. It does not represent progress or
    /// delay initialization; it simply makes the first impression feel alive.
    fn animated_logo(&mut self) -> io::Result<()> {
        let rows = logo_rows(self.width);

        if rows.is_empty() {
            writeln!(
                self.writer,
                "{}",
                gradient_line(PRODUCT_NAME, &self.theme, self.color, 0.0)
            )?;
            return Ok(());
        }

        let started = Instant::now();

        for frame in 0..ANIMATION_FRAMES {
            let phase = started.elapsed().as_secs_f64() * 2.2;
            let progress = (frame + 1) as f64 / ANIMATION_FRAMES as f64;

            for (row, line) in rows.iter().enumerate() {
                let line = if self.color {
                    gradient_line(
                        line,
                        &self.theme,
                        self.color,
                        phase + row as f64 * GRADIENT_ROW_STEP,
                    )
                } else {
                    reveal_line(line, progress)
                };
                writeln!(self.writer, "{line}")?;
            }

            self.writer.flush()?;

            if frame + 1 < ANIMATION_FRAMES {
                write!(self.writer, "\x1b[{}A", rows.len())?;
                self.writer.flush()?;
                std::thread::sleep(self.frame);
            }
        }

        Ok(())
    }

    /// Applies the configured terminal font with OSC 50, when requested.
    pub fn apply_terminal_font(&mut self, font: &str) -> io::Result<()> {
        if self.mode == StartupMode::Quiet || font.trim().is_empty() {
            return Ok(());
        }

        self.writer
            .write_all(terminal_font_sequence(font).as_bytes())?;
        self.writer.flush()
    }

    /// Reports the version, platform, and working directory after the session
    /// has been created.
    pub fn session_summary(&mut self, session: &Session) -> io::Result<()> {
        if self.mode == StartupMode::Quiet {
            return Ok(());
        }

        writeln!(
            self.writer,
            "  {}",
            render_session_summary_themed(session, self.color, &self.theme)
        )?;
        writeln!(self.writer)?;
        self.writer.flush()
    }

    /// Signals that a real initialization stage has started.
    pub fn stage_begin(&mut self, label: &str) -> io::Result<()> {
        if self.mode != StartupMode::Animated {
            return Ok(());
        }

        write!(
            self.writer,
            "\r{} {label}",
            style(
                "[..]",
                &format!("{DIM}{}", self.theme.accent.foreground()),
                self.color
            )
        )?;
        self.writer.flush()
    }

    /// Reports that the current stage completed successfully.
    pub fn stage_ok(&mut self, label: &str) -> io::Result<()> {
        if self.mode == StartupMode::Quiet {
            return Ok(());
        }

        if self.mode == StartupMode::Animated {
            write!(self.writer, "\r")?;
        }

        writeln!(
            self.writer,
            "{}",
            render_stage_ok_themed(label, self.color, &self.theme)
        )?;
        self.writer.flush()
    }

    /// Reports that the current stage failed.
    pub fn stage_failed(&mut self, label: &str, error: &str) -> io::Result<()> {
        if self.mode == StartupMode::Quiet {
            return Ok(());
        }

        if self.mode == StartupMode::Animated {
            write!(self.writer, "\r")?;
        }

        writeln!(
            self.writer,
            "{}",
            render_stage_failed_themed(label, error, self.color, &self.theme)
        )?;
        self.writer.flush()
    }

    /// Writes the welcome message and restores the cursor.
    pub fn welcome(&mut self) -> io::Result<()> {
        if self.mode == StartupMode::Quiet {
            return Ok(());
        }

        writeln!(self.writer)?;
        writeln!(
            self.writer,
            "{}",
            render_welcome_themed(self.color, &self.theme)
        )?;
        self.finish()
    }

    /// Restores the cursor and flushes after a successful startup.
    pub fn finish(&mut self) -> io::Result<()> {
        if self.mode == StartupMode::Animated {
            self.writer.write_all(SHOW_CURSOR.as_bytes())?;
        }
        self.writer.flush()
    }

    /// Restores terminal state after an initialization error.
    pub fn abort(&mut self) -> io::Result<()> {
        if self.mode != StartupMode::Quiet {
            self.writer.write_all(RESET.as_bytes())?;
        }
        if self.mode == StartupMode::Animated {
            self.writer.write_all(SHOW_CURSOR.as_bytes())?;
        }
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_config::Config;

    fn plain_startup() -> Startup<Vec<u8>> {
        Startup::new(Vec::new(), StartupMode::Plain, false, 100)
    }

    fn rendered(startup: &mut Startup<Vec<u8>>) -> String {
        String::from_utf8(std::mem::take(&mut startup.writer))
            .expect("startup output should be UTF-8")
    }

    #[test]
    fn intro_contains_brand_tagline_values_and_version() {
        let intro = render_intro(100, false);

        assert!(intro.contains("KERNEXT"));
        assert!(intro.contains("THE NEXT CORE OF COMPUTING"));
        assert!(intro.contains("OPEN • POWERFUL • INDEPENDENT"));
        assert!(intro.contains(&format!("v{VERSION}")));
    }

    #[test]
    fn wide_terminal_uses_the_full_logo() {
        let logo = render_logo(100, false);

        assert!(logo.contains('█'));
        assert_eq!(logo.lines().count(), LOGO_FULL.len());
    }

    #[test]
    fn narrow_terminal_uses_the_compact_logo() {
        let logo = render_logo(40, false);

        assert_eq!(logo.lines().count(), LOGO_COMPACT.len());
        assert!(logo.contains('╦'));
    }

    #[test]
    fn extremely_narrow_terminal_uses_the_plain_wordmark() {
        assert_eq!(render_logo(20, false), "KERNEXT");
    }

    #[test]
    fn colors_are_only_emitted_when_enabled() {
        let plain = render_stage_ok("Initializing KERNEXT Core", false);
        let colored = render_stage_ok("Initializing KERNEXT Core", true);

        assert_eq!(plain, "[OK] Initializing KERNEXT Core");
        assert!(colored.contains("\x1b["));
        assert!(colored.contains("Initializing KERNEXT Core"));
    }

    #[test]
    fn color_policy_respects_no_color_and_dumb_terminals() {
        assert!(!color_allowed(true, Some("xterm-256color"), true));
        assert!(!color_allowed(false, Some("dumb"), true));
        assert!(!color_allowed(false, Some("DUMB"), true));

        #[cfg(not(windows))]
        assert!(color_allowed(false, Some("xterm-256color"), false));

        #[cfg(windows)]
        {
            assert!(color_allowed(false, Some("xterm-256color"), true));
            assert!(!color_allowed(false, Some("xterm-256color"), false));
        }
    }

    #[test]
    fn plain_mode_writes_intro_stages_and_welcome() {
        let mut startup = plain_startup();
        startup.intro().expect("intro should write");
        startup
            .stage_ok("Initializing KERNEXT Core")
            .expect("stage should write");
        startup.welcome().expect("welcome should write");

        let output = rendered(&mut startup);

        assert!(output.contains("KERNEXT"));
        assert!(output.contains("[OK] Initializing KERNEXT Core"));
        assert!(output.contains("Welcome to KERNEXT."));
        assert!(output.contains("Build. Execute. Innovate."));
        assert!(!output.contains("\x1b["));
    }

    #[test]
    fn quiet_mode_writes_nothing() {
        let mut startup = Startup::new(Vec::new(), StartupMode::Quiet, true, 100);
        startup.intro().expect("intro should be a no-op");
        startup
            .stage_begin("Initializing KERNEXT Core")
            .expect("stage begin should be a no-op");
        startup
            .stage_ok("Initializing KERNEXT Core")
            .expect("stage should be a no-op");
        startup.welcome().expect("welcome should be a no-op");

        assert!(rendered(&mut startup).is_empty());
    }

    #[test]
    fn mode_resolution_respects_configuration_and_terminal_state() {
        let mut config = Config::default();

        assert_eq!(
            resolve_mode(&config, false, false, true),
            StartupMode::Animated
        );
        assert_eq!(
            resolve_mode(&config, false, false, false),
            StartupMode::Plain
        );

        config.startup_animation = false;
        assert_eq!(
            resolve_mode(&config, false, false, true),
            StartupMode::Plain
        );

        config.quiet_startup = true;
        assert_eq!(
            resolve_mode(&config, false, false, true),
            StartupMode::Quiet
        );

        let config = Config::default();
        assert_eq!(resolve_mode(&config, true, false, true), StartupMode::Quiet);
        assert_eq!(resolve_mode(&config, false, true, true), StartupMode::Plain);
    }

    #[test]
    fn failed_stage_is_reported_without_claiming_success() {
        let mut startup = plain_startup();
        startup
            .stage_failed("Initializing KERNEXT Core", "disk unavailable")
            .expect("failure should write");

        let output = rendered(&mut startup);

        assert!(output.contains("[FAIL] Initializing KERNEXT Core: disk unavailable"));
        assert!(!output.contains("[OK]"));
    }

    #[test]
    fn animated_startup_restores_the_cursor_and_colors_on_drop() {
        use std::cell::RefCell;
        use std::rc::Rc;

        #[derive(Clone)]
        struct SharedBuffer(Rc<RefCell<Vec<u8>>>);

        impl Write for SharedBuffer {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.borrow_mut().extend_from_slice(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let buffer = Rc::new(RefCell::new(Vec::new()));
        {
            let mut startup = Startup::new(
                SharedBuffer(Rc::clone(&buffer)),
                StartupMode::Animated,
                true,
                100,
            );
            startup.intro().expect("animated intro should write");
        }

        let output =
            String::from_utf8(buffer.borrow().clone()).expect("startup output should be UTF-8");

        assert!(
            output.contains(HIDE_CURSOR),
            "cursor should be hidden during startup"
        );
        assert!(
            output.contains(SHOW_CURSOR),
            "dropping startup must restore the cursor"
        );
        assert!(output.contains(RESET), "dropping startup must reset colors");
    }

    #[test]
    fn abort_restores_the_terminal_without_claiming_success() {
        let mut startup = Startup::new(Vec::new(), StartupMode::Animated, true, 100);
        startup.intro().expect("animated intro should write");
        startup.abort().expect("abort should write");

        let output = rendered(&mut startup);

        assert!(output.contains(SHOW_CURSOR));
        assert!(output.contains(RESET));
        assert!(!output.contains("[OK]"));
    }

    #[test]
    fn the_gradient_moves_between_frames() {
        let theme = Theme::default();
        let first = render_logo_themed(100, true, &theme, 0.0);
        let second = render_logo_themed(100, true, &theme, 1.0);

        assert!(first.contains("\x1b[38;2;"));
        assert_ne!(first, second, "the gradient must change over time");
        assert!(!render_logo_themed(100, false, &theme, 1.0).contains('\x1b'));
    }

    #[test]
    fn emblem_rows_keep_a_constant_display_width() {
        for rows in [&EMBLEM_FULL[..], &EMBLEM_COMPACT[..]] {
            let widths: Vec<usize> = rows.iter().map(|row| row.chars().count()).collect();
            let first = widths[0];

            assert!(
                widths.iter().all(|width| *width == first),
                "every emblem row must be {first} columns, got {widths:?}"
            );
        }
    }

    #[test]
    fn full_emblem_fits_its_width_threshold() {
        let widest = EMBLEM_FULL
            .iter()
            .map(|row| row.chars().count())
            .max()
            .expect("the emblem has rows");

        assert!(widest <= 76, "the full emblem must fit at 76 columns");
    }

    #[test]
    fn emblem_scales_with_the_terminal_width() {
        assert_eq!(emblem_rows(120), &EMBLEM_FULL);
        assert_eq!(emblem_rows(76), &EMBLEM_FULL);
        assert_eq!(emblem_rows(60), &EMBLEM_COMPACT);
        assert_eq!(emblem_rows(44), &EMBLEM_COMPACT);
        assert!(emblem_rows(43).is_empty());
        assert_eq!(render_emblem(40, false), "");
    }

    #[test]
    fn animation_speed_maps_to_bounded_frame_delays() {
        assert_eq!(frame_delay("normal"), ANIMATION_FRAME);
        assert_eq!(frame_delay("unknown"), ANIMATION_FRAME);
        assert!(frame_delay("fast") < frame_delay("normal"));
        assert!(frame_delay("slow") > frame_delay("normal"));
        assert!(emblem_frame_delay("slow") > EMBLEM_FRAME);
        assert!(emblem_frame_delay("fast") < EMBLEM_FRAME);
    }

    #[test]
    fn animated_emblem_reveal_is_bounded_and_complete() {
        // Colorless output keeps the assertions readable: every emblem row is
        // written verbatim, without per-character gradient escapes.
        let mut startup = Startup::new(Vec::new(), StartupMode::Animated, false, 120)
            .with_animation_speed("fast");
        startup.intro().expect("animated intro should write");

        let output = rendered(&mut startup);

        for row in EMBLEM_FULL {
            assert!(
                output.contains(row.trim_end()),
                "every emblem row should appear in the reveal"
            );
        }
    }

    #[test]
    fn terminal_font_hint_uses_osc_50() {
        assert_eq!(
            terminal_font_sequence("JetBrains Mono"),
            "\x1b]50;JetBrains Mono\x1b\\"
        );
    }

    #[test]
    fn animated_intro_redraws_the_animated_wordmark() {
        let mut startup = Startup::new(Vec::new(), StartupMode::Animated, true, 100);
        startup.intro().expect("animated intro should write");

        let output = rendered(&mut startup);

        assert!(output.contains(HIDE_CURSOR));
        assert!(
            output.contains("\x1b[6A"),
            "animated frames should move the cursor up over the full logo"
        );
        assert!(output.matches("\x1b[38;2;").count() > LOGO_FULL.len());
    }
}
