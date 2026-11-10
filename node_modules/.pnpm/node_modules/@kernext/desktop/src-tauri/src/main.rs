//! KERNEXT desktop entry point.
//!
//! The same executable also serves as the pseudoterminal worker for the
//! desktop terminal. `KERNEXT_DESKTOP_WORKER=1` starts the embedded KERNEXT
//! REPL instead of the Tauri window, so a desktop build never needs a separate
//! shell sidecar.

use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::var_os("KERNEXT_DESKTOP_WORKER").is_some() {
        return match shell_cli::worker::run_embedded() {
            Ok(status) => ExitCode::from(status.rem_euclid(256) as u8),
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        };
    }

    if let Err(error) = shell_desktop::run() {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
