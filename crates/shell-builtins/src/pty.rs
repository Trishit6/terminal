//! Genuine interactive terminal sessions.
//!
//! A pseudoterminal makes a child believe it is talking to a terminal: `isatty`
//! succeeds, line editing and full-screen programs work, Ctrl+C is delivered as
//! a foreground interrupt, and the window size can change while it runs.
//!
//! Platform backends:
//!
//! - Windows uses **ConPTY** (`CreatePseudoConsole`), the same mechanism Windows
//!   Terminal and VS Code use. `CreateProcessW` is called directly because the
//!   pseudoterminal is attached through a `STARTUPINFOEX` attribute, which
//!   [`std::process::Command`] cannot express.
//! - Linux and macOS use `posix_openpt` plus a slave opened with a controlling
//!   terminal (`setsid` + `TIOCSCTTY`).
//!
//! This is separate from [`crate::process::StdioMode::Inherit`], which simply
//! hands the shell's own console to the child. Sharing the console is the best
//! option when the shell already owns a terminal; a pseudoterminal is used when
//! KERNEXT must own the connection (relaying input and output itself), which is
//! what lets it observe the session, resize it, and keep running after the child
//! exits.
//!
//! # Limitations
//!
//! - Only a single external command can own a pseudoterminal. Pipeline stages
//!   always use pipes so the pipeline stays byte-exact.
//! - Ctrl+C forwarding depends on the terminal line discipline inside the
//!   pseudoterminal. A program that switches the child terminal out of `ISIG`
//!   or consumes raw `0x03` itself keeps full control, which is correct for
//!   programs such as editors and TUI clients.
//! - The shell's own terminal mode is the caller's responsibility. Interactive
//!   shells put the terminal in raw mode while a session runs so keystrokes are
//!   forwarded unchanged instead of being line-buffered by the console.

use crate::process::{ExternalCommand, ProcessError, ProcessReport};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

#[cfg(windows)]
#[path = "pty/windows.rs"]
mod backend;

#[cfg(unix)]
#[path = "pty/unix.rs"]
mod backend;

pub use backend::{PtyProcess, PtyResizer};

/// Default terminal size used when the shell itself has no console.
pub const DEFAULT_COLUMNS: u16 = 80;

/// Default terminal size used when the shell itself has no console.
pub const DEFAULT_ROWS: u16 = 24;

/// How often the shell looks for a window-size change while a session runs.
const RESIZE_POLL_INTERVAL: Duration = Duration::from_millis(150);

/// Rows and columns of a pseudoterminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtySize {
    rows: u16,
    columns: u16,
}

impl PtySize {
    /// The conventional 80x24 terminal.
    pub const DEFAULT: Self = Self {
        rows: DEFAULT_ROWS,
        columns: DEFAULT_COLUMNS,
    };

    /// Creates a size, clamping both dimensions to at least one cell.
    ///
    /// Zero-sized terminals confuse `CreatePseudoConsole` and `TIOCSWINSZ`, so
    /// they are never forwarded.
    #[must_use]
    pub fn new(rows: u16, columns: u16) -> Self {
        Self {
            rows: rows.max(1),
            columns: columns.max(1),
        }
    }

    /// Number of rows.
    #[must_use]
    pub const fn rows(self) -> u16 {
        self.rows
    }

    /// Number of columns.
    #[must_use]
    pub const fn columns(self) -> u16 {
        self.columns
    }
}

impl Default for PtySize {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Size of the shell's own terminal, when it has one.
#[must_use]
pub fn terminal_size() -> Option<PtySize> {
    backend::terminal_size()
}

/// Runs `command` on a pseudoterminal, relaying it to the shell's own streams.
///
/// Output is streamed as the child produces it. Input is forwarded until the
/// child exits, at which point the relay stops so it cannot consume keystrokes
/// meant for the prompt. Window-size changes are picked up while the session
/// runs, and Ctrl+C is forwarded into the pseudoterminal instead of terminating
/// the shell.
///
/// # Errors
///
/// Returns [`ProcessError`] when the program cannot be resolved, the
/// pseudoterminal cannot be created, or the child cannot be started or waited
/// for.
pub fn run_interactive(command: &ExternalCommand) -> Result<ProcessReport, ProcessError> {
    let size = terminal_size().unwrap_or(PtySize::DEFAULT);
    let mut process = PtyProcess::spawn(command, size)?;

    let reader = process.take_reader();
    let writer = process.take_writer();
    let stop = Arc::new(AtomicBool::new(false));

    let output_thread = reader.map(|mut reader| {
        thread::spawn(move || {
            let mut stdout = std::io::stdout();
            let mut buffer = [0u8; 8192];

            loop {
                match std::io::Read::read(&mut reader, &mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if stdout.write_all(&buffer[..count]).is_err() {
                            break;
                        }

                        let _ = stdout.flush();
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        })
    });

    let input_thread = writer.map(|writer| {
        let stop = Arc::clone(&stop);

        thread::spawn(move || backend::relay_stdin(writer, &stop))
    });

    let resizer = process.resizer();
    let monitor = {
        let stop = Arc::clone(&stop);

        thread::spawn(move || {
            let mut current = size;

            while !stop.load(Ordering::Relaxed) {
                thread::sleep(RESIZE_POLL_INTERVAL);

                if stop.load(Ordering::Relaxed) {
                    break;
                }

                if let Some(next) = terminal_size()
                    && next != current
                {
                    let _ = resizer.resize(next);
                    current = next;
                }
            }
        })
    };

    let forwarder = process.forward_interrupts();
    let status = process.wait();

    // Stop the relays before returning so no background thread can read the
    // user's next command line.
    drop(forwarder);
    stop.store(true, Ordering::Relaxed);
    drop(process);

    if let Some(thread) = output_thread {
        let _ = thread.join();
    }

    if let Some(thread) = input_thread {
        let _ = thread.join();
    }

    let _ = monitor.join();

    status.map(ProcessReport::from_status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::{ExternalCommand, StdioMode};
    use std::io::Read;
    use std::thread;

    #[test]
    fn sizes_are_clamped_to_at_least_one_cell() {
        let size = PtySize::new(0, 0);

        assert_eq!(size.rows(), 1);
        assert_eq!(size.columns(), 1);
    }

    #[test]
    fn the_default_size_is_eighty_by_twenty_four() {
        assert_eq!(PtySize::DEFAULT.rows(), 24);
        assert_eq!(PtySize::DEFAULT.columns(), 80);
        assert_eq!(PtySize::default(), PtySize::DEFAULT);
    }

    /// Helper process that reports whether it sees a terminal.
    ///
    /// Run by the tests below with `KERNEXT_PTY_PROBE` set; a no-op otherwise.
    #[test]
    fn pty_probe_helper() {
        use std::io::IsTerminal;

        if std::env::var("KERNEXT_PTY_PROBE").is_err() {
            return;
        }

        println!("stdout-terminal={}", std::io::stdout().is_terminal());
        println!("stdin-terminal={}", std::io::stdin().is_terminal());
        println!("stderr-terminal={}", std::io::stderr().is_terminal());

        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Storage::FileSystem::GetFileType;
            use windows_sys::Win32::System::Console::{
                GetConsoleWindow, GetStdHandle, STD_OUTPUT_HANDLE,
            };

            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            println!("output-file-type={}", GetFileType(handle));
            println!("console-window={}", !GetConsoleWindow().is_null());
        }
    }

    #[cfg(windows)]
    fn shell_command(script: &str) -> ExternalCommand {
        // For cmd /c, the script must be split into separate arguments so that
        // built-in commands like 'exit' work correctly. The first word is the
        // command, the rest are its arguments.
        let parts: Vec<&str> = script.split_whitespace().collect();
        let mut args = vec!["/d", "/c"];
        args.extend(parts);
        ExternalCommand::new("cmd").args(args)
    }

    #[cfg(unix)]
    fn shell_command(script: &str) -> ExternalCommand {
        ExternalCommand::new("sh").args(["-c", script])
    }

    #[test]
    fn a_child_sees_a_terminal_and_its_exit_code_is_reported() {
        let probe = std::env::current_exe().expect("test executable should be known");
        let command = ExternalCommand::new(probe.to_string_lossy().into_owned())
            .args(["--exact", "pty::tests::pty_probe_helper", "--nocapture"])
            .env("KERNEXT_PTY_PROBE", "1")
            .env("PATH", std::env::var_os("PATH").unwrap_or_default());

        let mut process =
            PtyProcess::spawn(&command, PtySize::new(24, 80)).expect("pseudoterminal should start");

        let reader = process.take_reader().expect("reader should be available");
        let output = thread::spawn(move || {
            let mut reader = reader;
            let mut output = Vec::new();
            let _ = reader.read_to_end(&mut output);
            output
        });

        let status = process.wait().expect("child should be waited for");
        drop(process);

        let output = output.join().expect("reader thread should finish");
        let text = String::from_utf8_lossy(&output);

        assert!(status.success(), "{status:?}: {text}");
        assert_eq!(status.code(), Some(0));
        assert!(
            text.contains("stdout-terminal=true"),
            "stdout is not a terminal: {text:?}"
        );
        assert!(
            text.contains("stdin-terminal=true"),
            "stdin is not a terminal: {text:?}"
        );
    }

    #[test]
    fn a_failing_child_reports_its_exit_code() {
        let command = shell_command("exit 42");

        let mut process =
            PtyProcess::spawn(&command, PtySize::DEFAULT).expect("pseudoterminal should start");
        let status = process.wait().expect("child should be waited for");

        assert!(!status.success());
        assert_eq!(status.code(), Some(42));
    }

    #[test]
    fn the_terminal_can_be_resized_while_the_child_runs() {
        let command = shell_command("echo resized");
        let mut process =
            PtyProcess::spawn(&command, PtySize::new(24, 80)).expect("pseudoterminal should start");

        process
            .resizer()
            .resize(PtySize::new(50, 132))
            .expect("resize should succeed on a live pseudoterminal");

        let status = process.wait().expect("child should be waited for");
        assert!(status.success(), "{status:?}");
    }

    #[test]
    fn keystrokes_reach_the_child_through_the_terminal() {
        let directory = std::env::temp_dir().join(format!(
            "kernext-pty-input-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        std::fs::create_dir_all(&directory).expect("temporary directory should be created");

        #[cfg(windows)]
        let command = {
            let script = directory.join("read.cmd");
            std::fs::write(&script, "@echo off\r\nset /p line=\r\necho got=%line%\r\n")
                .expect("script should be written");
            ExternalCommand::new(script.to_string_lossy().into_owned())
        };

        #[cfg(unix)]
        let command = {
            let _ = &directory;
            ExternalCommand::new("sh").args(["-c", "read line; echo got=$line"])
        };

        let mut process =
            PtyProcess::spawn(&command, PtySize::DEFAULT).expect("pseudoterminal should start");
        let mut writer = process.take_writer().expect("writer should be available");
        writer
            .write_all(b"typed-line\r\n")
            .expect("input should be accepted");
        writer.flush().expect("input should be flushed");

        let reader = process.take_reader().expect("reader should be available");
        let output = thread::spawn(move || {
            let mut reader = reader;
            let mut output = Vec::new();
            let _ = reader.read_to_end(&mut output);
            output
        });

        let status = process.wait().expect("child should be waited for");
        drop(process);
        drop(writer);

        let output = output.join().expect("reader thread should finish");
        let text = String::from_utf8_lossy(&output);
        let _ = std::fs::remove_dir_all(&directory);

        assert!(status.success(), "{status:?}: {text}");
        assert!(text.contains("got=typed-line"), "missing output: {text:?}");
    }

    #[test]
    fn an_unknown_program_is_reported_as_not_found() {
        let command = ExternalCommand::new("kernext-definitely-missing-pty").stdio(StdioMode::Pty);

        let error = PtyProcess::spawn(&command, PtySize::DEFAULT)
            .err()
            .expect("missing program should fail");

        assert!(matches!(error, ProcessError::NotFound { .. }), "{error:?}");
    }

    fn unique_suffix() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT: AtomicU64 = AtomicU64::new(0);

        NEXT.fetch_add(1, Ordering::Relaxed)
    }
}
