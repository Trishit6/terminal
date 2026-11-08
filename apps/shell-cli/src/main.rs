use shell_cli::cli;
use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::run() {
        Ok(status) => ExitCode::from(status.rem_euclid(256) as u8),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
