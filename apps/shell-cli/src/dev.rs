//! KERNEXT project developer commands.
//!
//! `kernext build`, `kernext test`, and the rest are a user-facing abstraction
//! over the Rust toolchain and the JavaScript tooling. Cargo remains the
//! underlying build tool: this module only decides which invocation to run, and
//! it never assembles a shell command line. Every argument is passed to the
//! child process as its own `OsString`, so paths with spaces and quoted values
//! keep their boundaries.
//!
//! The commands run before the interactive shell starts and do not require a
//! terminal on standard input.

use clap::{Args, Subcommand};
use shell_core::{BRAND_STATEMENT, PRODUCT_NAME, TAGLINE, VERSION};
use std::ffi::OsString;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

/// Environment override for the project root, used by tests and by editors that
/// run commands from outside the checkout.
pub const PROJECT_ROOT_ENV: &str = "KERNEXT_PROJECT_ROOT";

/// Minimum Node.js major version assumed by the front-end toolchain.
const MIN_NODE_MAJOR: u64 = 22;

/// Minimum npm major version that supports the workspace scripts used here.
const MIN_NPM_MAJOR: u64 = 10;

/// Developer subcommands exposed by the `kernext` executable.
#[derive(Debug, Subcommand)]
pub enum DevCommand {
    /// Build the workspace with Cargo.
    Build(BuildArgs),
    /// Type-check the workspace without producing binaries.
    Check(CheckArgs),
    /// Run the workspace test suite.
    Test(TestArgs),
    /// Format the workspace with rustfmt.
    Fmt(FmtArgs),
    /// Run Clippy with warnings denied.
    Lint(LintArgs),
    /// Run the KERNEXT shell from the workspace.
    Run(RunArgs),
    /// Remove Cargo build artifacts.
    Clean(CleanArgs),
    /// Inspect the development environment.
    Doctor,
    /// Display version information.
    Version,
    /// Show the developer command reference.
    Help,
}

/// Options for `kernext build`.
#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Build optimized artifacts.
    #[arg(long)]
    pub release: bool,
    /// Build only this Cargo package.
    #[arg(short = 'p', long, value_name = "PACKAGE")]
    pub package: Option<String>,
    /// Build all targets, including tests, examples, and benches.
    #[arg(long)]
    pub all_targets: bool,
    /// Print the Cargo invocation without running it.
    #[arg(long)]
    pub dry_run: bool,
}

/// Options for `kernext check`.
#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Check only this Cargo package.
    #[arg(short = 'p', long, value_name = "PACKAGE")]
    pub package: Option<String>,
    /// Check optimized artifacts.
    #[arg(long)]
    pub release: bool,
    /// Print the Cargo invocation without running it.
    #[arg(long)]
    pub dry_run: bool,
}

/// Options for `kernext test`.
#[derive(Debug, Args)]
pub struct TestArgs {
    /// Test only this Cargo package.
    #[arg(short = 'p', long, value_name = "PACKAGE")]
    pub package: Option<String>,
    /// Only run tests whose names contain this string.
    #[arg(value_name = "FILTER")]
    pub filter: Option<String>,
    /// Arguments passed to the test harness after `--`.
    #[arg(last = true, value_name = "ARGS")]
    pub extra: Vec<String>,
    /// Build the test binaries without running them.
    #[arg(long)]
    pub no_run: bool,
    /// Print the Cargo invocation without running it.
    #[arg(long)]
    pub dry_run: bool,
}

/// Options for `kernext fmt`.
#[derive(Debug, Args)]
pub struct FmtArgs {
    /// Check formatting instead of rewriting files.
    #[arg(long)]
    pub check: bool,
    /// Print the Cargo invocation without running it.
    #[arg(long)]
    pub dry_run: bool,
}

/// Options for `kernext lint`.
#[derive(Debug, Args)]
pub struct LintArgs {
    /// Lint only this Cargo package.
    #[arg(short = 'p', long, value_name = "PACKAGE")]
    pub package: Option<String>,
    /// Apply Clippy's suggested fixes where it can.
    #[arg(long)]
    pub fix: bool,
    /// Print the Cargo invocation without running it.
    #[arg(long)]
    pub dry_run: bool,
}

/// Options for `kernext run`.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// Run an optimized shell binary.
    #[arg(long)]
    pub release: bool,
    /// Arguments passed to the shell after `--`.
    #[arg(last = true, value_name = "ARGS")]
    pub args: Vec<String>,
    /// Print the Cargo invocation without running it.
    #[arg(long)]
    pub dry_run: bool,
}

/// Options for `kernext clean`.
#[derive(Debug, Args)]
pub struct CleanArgs {
    /// Remove artifacts for only this Cargo package.
    #[arg(short = 'p', long, value_name = "PACKAGE")]
    pub package: Option<String>,
    /// Remove artifacts for all targets (tests, examples, benches) in addition to
    /// library/binaries.
    #[arg(long)]
    pub all_targets: bool,
    /// Print the Cargo invocation without running it.
    #[arg(long)]
    pub dry_run: bool,
}

/// Runs a developer command and returns the process exit code.
///
/// The code is the child tool's own exit code when it ran, `127` when the tool
/// is not installed, and `1` when the project root cannot be determined.
#[must_use]
pub fn execute(command: DevCommand) -> i32 {
    match command {
        DevCommand::Version => version(),
        DevCommand::Help => {
            help();
            0
        }
        DevCommand::Doctor => doctor(),
        DevCommand::Build(args) => build(args),
        DevCommand::Check(args) => check(args),
        DevCommand::Test(args) => test(args),
        DevCommand::Fmt(args) => fmt(args),
        DevCommand::Lint(args) => lint(args),
        DevCommand::Run(args) => run(args),
        DevCommand::Clean(args) => clean(args),
    }
}

/// Builds the Cargo argument list for `kernext build`.
#[must_use]
pub fn build_arguments(args: &BuildArgs) -> Vec<OsString> {
    let mut cargo = Vec::new();
    cargo.push(OsString::from("build"));
    if args.release {
        cargo.push(OsString::from("--release"));
    }
    if let Some(package) = &args.package {
        cargo.push(OsString::from("-p"));
        cargo.push(OsString::from(package));
    } else {
        cargo.push(OsString::from("--workspace"));
    }
    if args.all_targets {
        cargo.push(OsString::from("--all-targets"));
    }
    cargo
}

/// Builds the Cargo argument list for `kernext check`.
#[must_use]
pub fn check_arguments(args: &CheckArgs) -> Vec<OsString> {
    let mut cargo = Vec::new();
    cargo.push(OsString::from("check"));
    if args.release {
        cargo.push(OsString::from("--release"));
    }
    if let Some(package) = &args.package {
        cargo.push(OsString::from("-p"));
        cargo.push(OsString::from(package));
    } else {
        cargo.push(OsString::from("--workspace"));
    }
    cargo.push(OsString::from("--all-targets"));
    cargo
}

/// Builds the Cargo argument list for `kernext test`.
#[must_use]
pub fn test_arguments(args: &TestArgs) -> Vec<OsString> {
    let mut cargo = Vec::new();
    cargo.push(OsString::from("test"));
    if let Some(package) = &args.package {
        cargo.push(OsString::from("-p"));
        cargo.push(OsString::from(package));
    } else {
        cargo.push(OsString::from("--workspace"));
    }
    if args.no_run {
        cargo.push(OsString::from("--no-run"));
    }
    if let Some(filter) = &args.filter {
        cargo.push(OsString::from(filter));
    }
    if !args.extra.is_empty() {
        cargo.push(OsString::from("--"));
        cargo.extend(args.extra.iter().map(OsString::from));
    }
    cargo
}

/// Builds the Cargo argument list for `kernext fmt`.
#[must_use]
pub fn fmt_arguments(args: &FmtArgs) -> Vec<OsString> {
    let mut cargo = vec![OsString::from("fmt"), OsString::from("--all")];
    if args.check {
        cargo.push(OsString::from("--"));
        cargo.push(OsString::from("--check"));
    }
    cargo
}

/// Builds the Cargo argument list for `kernext lint`.
#[must_use]
pub fn lint_arguments(args: &LintArgs) -> Vec<OsString> {
    let mut cargo = vec![OsString::from("clippy")];
    if let Some(package) = &args.package {
        cargo.push(OsString::from("-p"));
        cargo.push(OsString::from(package));
        cargo.push(OsString::from("--all-targets"));
    } else {
        cargo.push(OsString::from("--workspace"));
        cargo.push(OsString::from("--all-targets"));
    }
    if args.fix {
        cargo.push(OsString::from("--fix"));
    }
    cargo.push(OsString::from("--"));
    cargo.push(OsString::from("-D"));
    cargo.push(OsString::from("warnings"));
    cargo
}

/// Builds the Cargo argument list for `kernext clean`.
#[must_use]
pub fn clean_arguments(args: &CleanArgs) -> Vec<OsString> {
    let mut cargo = vec![OsString::from("clean")];
    if let Some(package) = &args.package {
        cargo.push(OsString::from("-p"));
        cargo.push(OsString::from(package));
    } else {
        cargo.push(OsString::from("--workspace"));
    }
    if args.all_targets {
        cargo.push(OsString::from("--all-targets"));
    }
    cargo
}

/// Builds the Cargo argument list for `kernext run`.
#[must_use]
pub fn run_arguments(args: &RunArgs) -> Vec<OsString> {
    let mut cargo = vec![OsString::from("run"), OsString::from("-p")];
    cargo.push(OsString::from("shell-cli"));
    if args.release {
        cargo.push(OsString::from("--release"));
    }
    if !args.args.is_empty() {
        cargo.push(OsString::from("--"));
        cargo.extend(args.args.iter().map(OsString::from));
    }
    cargo
}

fn build(args: BuildArgs) -> i32 {
    let cargo = build_arguments(&args);
    execute_cargo("build", "Build", cargo, args.dry_run)
}

fn check(args: CheckArgs) -> i32 {
    let cargo = check_arguments(&args);
    execute_cargo("check", "Check", cargo, args.dry_run)
}

fn test(args: TestArgs) -> i32 {
    let cargo = test_arguments(&args);
    execute_cargo("test", "Test", cargo, args.dry_run)
}

fn fmt(args: FmtArgs) -> i32 {
    let cargo = fmt_arguments(&args);
    execute_cargo("fmt", "Format", cargo, args.dry_run)
}

fn lint(args: LintArgs) -> i32 {
    let cargo = lint_arguments(&args);
    execute_cargo("clippy", "Lint", cargo, args.dry_run)
}

fn run(args: RunArgs) -> i32 {
    let cargo = run_arguments(&args);
    execute_cargo("run", "Run", cargo, args.dry_run)
}

fn clean(args: CleanArgs) -> i32 {
    let cargo = clean_arguments(&args);
    execute_cargo("clean", "Clean", cargo, args.dry_run)
}

/// The Cargo executable. `CARGO` is set when the shell itself was started by
/// Cargo, which keeps the same toolchain in use.
fn cargo_program() -> OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

/// Resolves the workspace root by walking up from `start` to the nearest
/// directory whose `Cargo.toml` declares a `[workspace]`.
#[must_use]
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut candidate = Some(start);
    while let Some(directory) = candidate {
        if is_workspace_root(directory) {
            return Some(directory.to_path_buf());
        }
        candidate = directory.parent();
    }
    None
}

fn is_workspace_root(directory: &Path) -> bool {
    let Ok(manifest) = std::fs::read_to_string(directory.join("Cargo.toml")) else {
        return false;
    };
    manifest
        .lines()
        .any(|line| line.trim().trim_end_matches('\r') == "[workspace]")
}

/// Resolves the project root for a developer command.
pub fn project_root() -> Result<PathBuf, String> {
    if let Some(explicit) = std::env::var_os(PROJECT_ROOT_ENV) {
        let path = PathBuf::from(explicit);
        return if is_workspace_root(&path) {
            Ok(path)
        } else {
            Err(format!(
                "{PROJECT_ROOT_ENV} points at `{}`, which is not a KERNEXT workspace root",
                path.display()
            ))
        };
    }

    let start = std::env::current_dir()
        .map_err(|error| format!("could not read the working directory: {error}"))?;

    find_project_root(&start).ok_or_else(|| {
        format!(
            "not inside a KERNEXT checkout: no Cargo.toml with a [workspace] table was found in \
             `{}` or any parent directory",
            start.display()
        )
    })
}

/// Executes a Cargo invocation and translates its result into an exit code.
fn execute_cargo(subcommand: &str, title: &str, arguments: Vec<OsString>, dry_run: bool) -> i32 {
    let palette = Palette::detect();
    let root = match project_root() {
        Ok(root) => root,
        Err(message) => {
            eprintln!("{} {message}", palette.paint(RED, "error:"));
            eprintln!(
                "{}",
                palette.paint(
                    DIM,
                    "  Run this command from inside the KERNEXT repository, or set \
                     KERNEXT_PROJECT_ROOT."
                )
            );
            return 1;
        }
    };

    let program = cargo_program();
    let invocation = Invocation {
        program: program.clone(),
        arguments,
    };

    println!(
        "{} {} {}",
        palette.paint(BOLD_CYAN, PRODUCT_NAME),
        palette.paint(DIM, &format!("v{VERSION} · {title}")),
        palette.paint(DIM, root.display().to_string().as_str())
    );
    println!("  {} {}", palette.paint(DIM, "$"), invocation.display());

    if dry_run {
        println!();
        println!(
            "{}",
            palette.paint(DIM, "dry run: the command above was not executed.")
        );
        return 0;
    }

    let started = Instant::now();
    let status = Command::new(&invocation.program)
        .args(&invocation.arguments)
        .current_dir(&root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    match status {
        Ok(status) if status.success() => {
            println!(
                "{} {subcommand} completed in {:.1}s",
                palette.paint(GREEN, "✓"),
                started.elapsed().as_secs_f64()
            );
            0
        }
        Ok(status) => {
            let code = status.code().unwrap_or(1);
            println!();
            println!(
                "{} {subcommand} failed with exit code {code}",
                palette.paint(RED, "✗")
            );
            code
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            eprintln!();
            eprintln!(
                "{} `{}` was not found on PATH.",
                palette.paint(RED, "error:"),
                program.to_string_lossy()
            );
            eprintln!(
                "{}",
                palette.paint(
                    DIM,
                    "  Install the Rust toolchain from https://rustup.rs and run the command again."
                )
            );
            127
        }
        Err(error) => {
            eprintln!();
            eprintln!(
                "{} could not start `{}`: {error}",
                palette.paint(RED, "error:"),
                program.to_string_lossy()
            );
            1
        }
    }
}

struct Invocation {
    program: OsString,
    arguments: Vec<OsString>,
}

impl Invocation {
    fn display(&self) -> String {
        let mut text = self.program.to_string_lossy().into_owned();
        for argument in &self.arguments {
            text.push(' ');
            let argument = argument.to_string_lossy();
            if argument.is_empty() || argument.contains(' ') {
                text.push('"');
                text.push_str(&argument);
                text.push('"');
            } else {
                text.push_str(&argument);
            }
        }
        text
    }
}

fn version() -> i32 {
    let palette = Palette::detect();
    println!(
        "{} {}",
        palette.paint(BOLD_CYAN, PRODUCT_NAME),
        palette.paint(BOLD, &format!("v{VERSION}"))
    );
    println!("{TAGLINE}");
    println!("{BRAND_STATEMENT}");
    println!();

    let rows = [
        ("Rust toolchain", probe("rustc", &["--version"])),
        (
            "Cargo",
            probe(cargo_program().to_string_lossy().as_ref(), &["--version"]),
        ),
        ("Node.js", probe("node", &["--version"])),
        ("npm", probe("npm", &["--version"])),
    ];

    for (label, value) in rows {
        if let Some(value) = value {
            let label = format!("{label:<16}");
            println!("  {}{value}", palette.paint(DIM, &label));
        }
    }

    let label = format!("{:<16}", "Project root");
    match project_root() {
        Ok(root) => println!("\n  {}{}", palette.paint(DIM, &label), root.display()),
        Err(_) => println!(
            "\n  {}{}",
            palette.paint(DIM, &label),
            palette.paint(DIM, "not inside a KERNEXT checkout")
        ),
    }

    0
}

fn help() -> i32 {
    let palette = Palette::detect();
    println!("{}", palette.paint(BOLD_CYAN, PRODUCT_NAME));
    println!("{TAGLINE}");
    println!("{BRAND_STATEMENT}");
    println!();
    println!("{}", palette.paint(BOLD, "Project tools"));
    println!();
    for (name, about) in [
        ("build", "Build the workspace"),
        ("check", "Type-check the project"),
        ("test", "Run the workspace tests"),
        ("fmt", "Format source code"),
        ("lint", "Run static analysis"),
        ("run", "Run the shell from source"),
        ("clean", "Remove build artifacts"),
        ("doctor", "Inspect the environment"),
        ("version", "Display version information"),
        ("help", "Show available commands"),
    ] {
        let command = format!("{:<18}", format!("kernext {name}"));
        println!("  {}{about}", palette.paint(CYAN, &command));
    }
    println!();
    println!("{}", palette.paint(BOLD, "Shell"));
    println!();
    for (command, about) in [
        ("kernext", "Start the interactive shell"),
        ("kernext --quiet", "Start without the startup banner"),
    ] {
        let command = format!("{command:<18}");
        println!("  {}{about}", palette.paint(CYAN, &command));
    }
    println!();
    println!(
        "{}",
        palette.paint(
            DIM,
            "Project commands call Cargo and rustfmt under the hood. Pass `--dry-run` to see the \
             exact invocation."
        )
    );
    0
}

fn doctor() -> i32 {
    let palette = Palette::detect();
    println!("{}", palette.paint(BOLD_CYAN, PRODUCT_NAME));
    println!("{TAGLINE}");
    println!("{BRAND_STATEMENT}");

    let mut report = Report::default();

    report.section("Environment");
    report.finding(Status::Available, "KERNEXT", format!("v{VERSION}"));
    report.finding(
        Status::Available,
        "Platform",
        format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
    );

    report.section("Rust tooling");
    let rustc = probe("rustc", &["--version"]);
    if let Some(version) = &rustc {
        report.finding(Status::Available, "rustc", version.clone());
    } else {
        report.finding(Status::Missing, "rustc", "not found on PATH".into());
        report.suggest("Install the Rust toolchain from https://rustup.rs.");
    }

    let cargo = cargo_program();
    let cargo_label = cargo.to_string_lossy().into_owned();
    match probe(&cargo_label, &["--version"]) {
        Some(version) => report.finding(Status::Available, "cargo", version),
        None => {
            report.finding(
                Status::Missing,
                "cargo",
                format!("`{cargo_label}` not found on PATH"),
            );
            report.suggest("Install the Rust toolchain from https://rustup.rs.");
        }
    }

    match probe("cargo", &["clippy", "--version"]) {
        Some(version) => report.finding(Status::Available, "clippy", version),
        None => {
            report.finding(Status::Missing, "clippy", "component not installed".into());
            report.suggest("Run `rustup component add clippy`.");
        }
    }

    match probe("cargo", &["fmt", "--version"]) {
        Some(version) => report.finding(Status::Available, "rustfmt", version),
        None => {
            report.finding(Status::Missing, "rustfmt", "component not installed".into());
            report.suggest("Run `rustup component add rustfmt`.");
        }
    }

    match probe("rustup", &["--version"]) {
        Some(version) => report.finding(Status::Available, "rustup", version),
        None => report.finding(
            Status::NotApplicable,
            "rustup",
            "not installed; using the toolchain on PATH".into(),
        ),
    }

    report.section("Frontend tooling");
    match probe("node", &["--version"]) {
        Some(version) => {
            let status = match major_version(&version) {
                Some(major) if major >= MIN_NODE_MAJOR => Status::Available,
                Some(_) => Status::Warning,
                None => Status::Warning,
            };
            report.finding(status, "Node.js", version.clone());
            if status == Status::Warning {
                report.suggest(format!(
                    "Use Node.js {MIN_NODE_MAJOR} or newer for the web and desktop front ends."
                ));
            }
        }
        None => {
            report.finding(Status::Missing, "Node.js", "not found on PATH".into());
            report.suggest(
                "Install Node.js 22 or newer from https://nodejs.org to build the front end.",
            );
        }
    }

    match probe("npm", &["--version"]) {
        Some(version) => {
            let status = match major_version(&version) {
                Some(major) if major >= MIN_NPM_MAJOR => Status::Available,
                Some(_) => Status::Warning,
                None => Status::Warning,
            };
            report.finding(status, "npm", version);
            if status == Status::Warning {
                report.suggest(format!(
                    "Update npm to {MIN_NPM_MAJOR} or newer with `npm install -g npm@latest`."
                ));
            }
        }
        None => {
            report.finding(Status::Missing, "npm", "not found on PATH".into());
            report.suggest("Install Node.js, which ships npm, from https://nodejs.org.");
        }
    }

    match project_root() {
        Ok(root) => {
            report.finding(
                Status::Available,
                "Lockfile",
                relative_check(&root, "package-lock.json"),
            );
            let installed = root.join("node_modules").is_dir();
            report.finding(
                if installed {
                    Status::Available
                } else {
                    Status::Missing
                },
                "Frontend dependencies",
                if installed {
                    "installed".into()
                } else {
                    "missing".into()
                },
            );
            if !installed {
                report.suggest("Run `npm install` to install the front-end dependencies.");
            }
        }
        Err(_) => {
            report.finding(
                Status::Missing,
                "Frontend dependencies",
                "unknown (no project root)".into(),
            );
        }
    }

    report.section("Project");
    match project_root() {
        Ok(root) => {
            report.finding(Status::Available, "Workspace", root.display().to_string());
            report.finding(
                Status::Available,
                "Lockfile (Rust)",
                relative_check(&root, "Cargo.lock"),
            );
            report.finding(
                Status::Available,
                "Shell config",
                match shell_config::Config::default_path() {
                    Some(path) if path.is_file() => format!("{} (found)", path.display()),
                    Some(path) => format!("{} (defaults)", path.display()),
                    None => "unavailable on this platform".into(),
                },
            );
            if let Some(path) = shell_config::Config::default_path() {
                if path.is_file() && shell_config::Config::load_or_default(&path).is_err() {
                    report.finding(
                        Status::Warning,
                        "Shell config file",
                        "could not be parsed; defaults are in use".into(),
                    );
                    report.suggest(format!(
                        "Fix or remove `{}`; KERNEXT falls back to defaults.",
                        path.display()
                    ));
                }
            }
        }
        Err(message) => {
            report.finding(Status::Missing, "Workspace", message);
        }
    }

    report.section("Git");
    if probe("git", &["--version"]).is_none() {
        report.finding(Status::NotApplicable, "Git", "not installed".into());
    } else {
        match probe("git", &["rev-parse", "--abbrev-ref", "HEAD"]) {
            Some(branch) => {
                let changes = count_changes();
                report.finding(
                    Status::Available,
                    "Branch",
                    match changes {
                        Some(0) => format!("{branch} (clean)"),
                        Some(count) => format!("{branch} ({count} uncommitted)"),
                        None => branch,
                    },
                );
            }
            None => report.finding(
                Status::NotApplicable,
                "Repository",
                "not a Git work tree".into(),
            ),
        }
    }

    report.section("Build environment");
    match cargo_target_directory() {
        Some(path) => {
            let exists = path.is_dir();
            report.finding(
                if exists {
                    Status::Available
                } else {
                    Status::Warning
                },
                "Cargo target dir",
                if exists {
                    path.display().to_string()
                } else {
                    format!("{} (created on first build)", path.display())
                },
            );
        }
        None => report.finding(
            Status::NotApplicable,
            "Cargo target dir",
            "Cargo default (`target/`)".into(),
        ),
    }
    let session = shell_runtime::Session::new(shell_config::Config::default());
    match session {
        Ok(session) => report.finding(
            Status::Available,
            "Shell backend",
            format!("session in `{}`", session.working_directory().display()),
        ),
        Err(error) => {
            report.finding(Status::Warning, "Shell backend", error.to_string());
            report.suggest("Fix the shell configuration; the session could not be created.");
        }
    }

    report.section("Platform capabilities");
    let interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
    report.finding(
        if interactive {
            Status::Available
        } else {
            Status::Warning
        },
        "Interactive terminal",
        if interactive {
            "stdin and stdout are terminals".into()
        } else {
            "not a terminal; the shell needs one".into()
        },
    );
    match shell_builtins::terminal_size() {
        Some(size) => report.finding(
            Status::Available,
            "Pseudoterminal",
            format!("{} × {}", size.columns(), size.rows()),
        ),
        None => report.finding(
            Status::NotApplicable,
            "Pseudoterminal",
            "no size reported for this terminal".into(),
        ),
    }
    #[cfg(windows)]
    report.finding(
        Status::Available,
        "Command scripts",
        "`.cmd`/`.bat` launch through the system command interpreter".into(),
    );
    #[cfg(not(windows))]
    report.finding(
        Status::NotApplicable,
        "Command scripts",
        "Windows command scripts do not apply on this platform".into(),
    );
    report.print(&palette);

    // Warnings are informational; a missing requirement makes doctor fail so
    // scripts can gate on it.
    if report.missing > 0 { 1 } else { 0 }
}

fn count_changes() -> Option<usize> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(text.lines().filter(|line| !line.trim().is_empty()).count())
}

fn relative_check(root: &Path, name: &str) -> String {
    if root.join(name).exists() {
        format!("{name} present")
    } else {
        format!("{name} missing")
    }
}

/// Reads the Cargo target directory from `CARGO_TARGET_DIR` or the tracked
/// `.cargo/config.toml`.
fn cargo_target_directory() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("CARGO_TARGET_DIR") {
        return Some(PathBuf::from(value));
    }

    let root = project_root().ok()?;
    let config = std::fs::read_to_string(root.join(".cargo/config.toml")).ok()?;
    for line in config.lines() {
        let line = line.trim();
        if !line.starts_with('#')
            && let Some((key, value)) = line.split_once('=')
            && key.trim() == "target-dir"
        {
            let value = value.trim().trim_matches('"');
            if !value.is_empty() {
                return Some(PathBuf::from(value));
            }
        }
    }
    None
}

/// Runs a program and returns the first line of its version output.
///
/// Probing goes through the shell's own external executor so Windows command
/// scripts such as `npm.cmd` resolve the same way they do inside KERNEXT.
fn probe(program: &str, arguments: &[&str]) -> Option<String> {
    let report = shell_builtins::ExternalCommand::new(program)
        .args(arguments.iter().copied())
        .stdio(shell_builtins::StdioMode::Capture)
        .run()
        .ok()?;

    if !report.success() {
        return None;
    }

    let text = if report.stdout().trim().is_empty() {
        report.stderr()
    } else {
        report.stdout()
    };

    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn major_version(version: &str) -> Option<u64> {
    version
        .trim_start_matches('v')
        .split(['.', '-', '+'])
        .next()
        .and_then(|value| value.parse().ok())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Available,
    Missing,
    Warning,
    NotApplicable,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Available => "Available",
            Status::Missing => "Missing",
            Status::Warning => "Warning",
            Status::NotApplicable => "Not applicable",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Status::Available => GREEN,
            Status::Missing => RED,
            Status::Warning => YELLOW,
            Status::NotApplicable => DIM,
        }
    }
}

#[derive(Default)]
struct Report {
    findings: Vec<(Option<&'static str>, Status, &'static str, String)>,
    suggestions: Vec<String>,
    missing: usize,
    attention: usize,
}

impl Report {
    fn section(&mut self, name: &'static str) {
        self.findings
            .push((Some(name), Status::Available, "", String::new()));
    }

    fn finding(&mut self, status: Status, label: &'static str, detail: String) {
        match status {
            Status::Missing => self.missing += 1,
            Status::Warning => self.attention += 1,
            Status::Available | Status::NotApplicable => {}
        }
        self.findings.push((None, status, label, detail));
    }

    fn suggest(&mut self, suggestion: impl Into<String>) {
        let suggestion = suggestion.into();
        if !self.suggestions.contains(&suggestion) {
            self.suggestions.push(suggestion);
        }
    }

    fn print(&self, palette: &Palette) {
        for (section, status, label, detail) in &self.findings {
            if let Some(section) = section {
                println!();
                println!("{}", palette.paint(BOLD, section));
                continue;
            }
            let status_text = format!("{:<16}", status.label());
            let label_text = format!("{label:<24}");
            println!(
                "  {}{}{}",
                palette.paint(status.color(), &status_text),
                label_text,
                palette.paint(DIM, detail)
            );
        }

        println!();
        if !self.suggestions.is_empty() {
            println!("{}", palette.paint(BOLD, "Suggestions"));
            for suggestion in &self.suggestions {
                println!("  {} {suggestion}", palette.paint(YELLOW, "•"));
            }
        }
        if self.missing == 0 && self.attention == 0 {
            println!("{}", palette.paint(GREEN, "No problems detected."));
        } else if self.suggestions.is_empty() {
            let count = self.missing + self.attention;
            println!(
                "{}",
                palette.paint(YELLOW, &format!("{count} item(s) need attention."))
            );
        }
    }
}

const RED: &str = "31";
const GREEN: &str = "32";
const YELLOW: &str = "33";
const CYAN: &str = "36";
const BOLD_CYAN: &str = "1;36";
const DIM: &str = "2";
const BOLD: &str = "1";

/// Whether ANSI styling is appropriate for the current output stream.
struct Palette {
    enabled: bool,
}

impl Palette {
    fn detect() -> Self {
        let term_is_dumb = std::env::var("TERM").is_ok_and(|term| term == "dumb");
        Self {
            enabled: io::stdout().is_terminal()
                && std::env::var_os("NO_COLOR").is_none()
                && !term_is_dumb,
        }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.enabled {
            format!("\u{1b}[{code}m{text}\u{1b}[0m")
        } else {
            text.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn build_defaults_to_the_whole_workspace() {
        let arguments = build_arguments(&BuildArgs {
            release: false,
            package: None,
            all_targets: false,
            dry_run: false,
        });
        assert_eq!(arguments, strings(&["build", "--workspace"]));
    }

    #[test]
    fn build_accepts_release_packages_and_all_targets() {
        let arguments = build_arguments(&BuildArgs {
            release: true,
            package: Some("shell-core".into()),
            all_targets: true,
            dry_run: false,
        });
        assert_eq!(
            arguments,
            strings(&["build", "--release", "-p", "shell-core", "--all-targets"])
        );
    }

    #[test]
    fn check_always_includes_all_targets() {
        let arguments = check_arguments(&CheckArgs {
            release: false,
            package: None,
            dry_run: false,
        });
        assert_eq!(
            arguments,
            strings(&["check", "--workspace", "--all-targets"])
        );
    }

    #[test]
    fn test_passes_harness_arguments_after_a_separator() {
        let arguments = test_arguments(&TestArgs {
            package: Some("shell-core".into()),
            filter: Some("parser".into()),
            extra: vec!["--nocapture".into()],
            no_run: false,
            dry_run: false,
        });
        assert_eq!(
            arguments,
            strings(&["test", "-p", "shell-core", "parser", "--", "--nocapture"])
        );
    }

    #[test]
    fn fmt_check_and_lint_use_the_expected_cargo_forms() {
        let format = fmt_arguments(&FmtArgs {
            check: true,
            dry_run: false,
        });
        assert_eq!(format, strings(&["fmt", "--all", "--", "--check"]));

        let lint = lint_arguments(&LintArgs {
            package: None,
            fix: false,
            dry_run: false,
        });
        assert_eq!(
            lint,
            strings(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings"
            ])
        );
    }

    #[test]
    fn run_targets_the_shell_cli_package_and_keeps_shell_arguments_separate() {
        let arguments = run_arguments(&RunArgs {
            release: false,
            args: vec!["--quiet".into()],
            dry_run: false,
        });
        assert_eq!(
            arguments,
            strings(&["run", "-p", "shell-cli", "--", "--quiet"])
        );
    }

    #[test]
    fn clean_defaults_to_the_whole_workspace() {
        let arguments = clean_arguments(&CleanArgs {
            package: None,
            all_targets: false,
            dry_run: false,
        });
        assert_eq!(arguments, strings(&["clean", "--workspace"]));
    }

    #[test]
    fn clean_accepts_package_and_all_targets() {
        let arguments = clean_arguments(&CleanArgs {
            package: Some("shell-core".into()),
            all_targets: true,
            dry_run: false,
        });
        assert_eq!(
            arguments,
            strings(&["clean", "-p", "shell-core", "--all-targets"])
        );
    }

    #[test]
    fn project_root_is_found_by_walking_up() {
        let directory = std::env::temp_dir().join(format!("kernext-root-{}", std::process::id()));
        let nested = directory.join("apps").join("web");
        std::fs::create_dir_all(&nested).expect("temporary directory");
        std::fs::write(directory.join("Cargo.toml"), "[workspace]\nmembers = []\n")
            .expect("workspace manifest");

        assert_eq!(find_project_root(&nested), Some(directory.clone()));

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_manifest_without_a_workspace_table_is_not_a_root() {
        let directory = std::env::temp_dir().join(format!("kernext-pkg-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("temporary directory");
        std::fs::write(directory.join("Cargo.toml"), "[package]\nname = \"demo\"\n")
            .expect("package manifest");

        assert_ne!(find_project_root(&directory), Some(directory.clone()));

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn versions_are_parsed_from_tool_output() {
        assert_eq!(major_version("v24.1.0"), Some(24));
        assert_eq!(major_version("11.2.0"), Some(11));
        assert_eq!(major_version("1.85.0-nightly"), Some(1));
        assert_eq!(major_version("unknown"), None);
    }
}
