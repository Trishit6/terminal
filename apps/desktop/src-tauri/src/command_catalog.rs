//! Command catalog for the desktop command explorer.
//!
//! Built-in commands come from the real registry. External commands are
//! resolved through the same `PATH` logic the shell uses, so availability is
//! checked against the current machine rather than hardcoded.

use crate::state::DesktopState;
use serde::Serialize;
use shell_builtins::{DEFAULT_BUILTINS, resolve_executable};

/// Command category shown in the explorer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandCategory {
    /// Shell syntax, scripting helpers, and lifecycle commands.
    CoreShell,
    /// Filesystem navigation and manipulation.
    Filesystem,
    /// Text processing and inspection.
    TextProcessing,
    /// System information and process control.
    System,
    /// Development and package tooling.
    Development,
    /// Networking and remote access.
    Networking,
}

impl CommandCategory {
    /// Stable machine-readable label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::CoreShell => "core_shell",
            Self::Filesystem => "filesystem",
            Self::TextProcessing => "text_processing",
            Self::System => "system",
            Self::Development => "development",
            Self::Networking => "networking",
        }
    }
}

/// Whether a command is native to KERNEXT or resolved from `PATH`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandKind {
    /// Native KERNEXT built-in.
    Builtin,
    /// External operating-system program.
    External,
}

/// One catalog entry shown in the command explorer.
#[derive(Debug, Clone, Serialize)]
pub struct CommandCatalogEntry {
    /// Command name typed at the prompt.
    pub name: String,
    /// One-line description.
    pub description: String,
    /// Explorer category.
    pub category: CommandCategory,
    /// `builtin` or `external`.
    pub kind: CommandKind,
    /// Whether the command can run on this host right now.
    pub available: bool,
    /// Example invocations.
    pub usage: Vec<String>,
    /// Platforms where the command is known to work: `windows`, `linux`,
    /// `macos`, or `all`.
    pub platforms: Vec<String>,
}

/// Static metadata for an external command.
struct ExternalCommandSpec {
    name: &'static str,
    description: &'static str,
    category: CommandCategory,
    usage: &'static [&'static str],
    platforms: &'static [&'static str],
}

const EXTERNAL_COMMANDS: &[ExternalCommandSpec] = &[
    ExternalCommandSpec {
        name: "git",
        description: "Distributed version control",
        category: CommandCategory::Development,
        usage: &["git status", "git log --oneline"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "node",
        description: "Run JavaScript outside the browser",
        category: CommandCategory::Development,
        usage: &["node --version", "node script.js"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "python",
        description: "Run the Python interpreter",
        category: CommandCategory::Development,
        usage: &["python --version", "python script.py"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "cargo",
        description: "Build and manage Rust projects",
        category: CommandCategory::Development,
        usage: &["cargo --version", "cargo build"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "rustc",
        description: "Compile Rust source files",
        category: CommandCategory::Development,
        usage: &["rustc --version"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "go",
        description: "Build and manage Go projects",
        category: CommandCategory::Development,
        usage: &["go version", "go build"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "make",
        description: "Run Makefile targets",
        category: CommandCategory::Development,
        usage: &["make", "make clean"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "docker",
        description: "Build and run containers",
        category: CommandCategory::Development,
        usage: &["docker --version", "docker ps"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "kubectl",
        description: "Control Kubernetes clusters",
        category: CommandCategory::Development,
        usage: &["kubectl version", "kubectl get pods"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "curl",
        description: "Transfer data from URLs",
        category: CommandCategory::Networking,
        usage: &["curl --version", "curl https://example.com"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "wget",
        description: "Download files from the network",
        category: CommandCategory::Networking,
        usage: &["wget https://example.com/file"],
        platforms: &["linux", "macos"],
    },
    ExternalCommandSpec {
        name: "ssh",
        description: "Open a secure remote shell",
        category: CommandCategory::Networking,
        usage: &["ssh user@host"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "ping",
        description: "Measure network reachability",
        category: CommandCategory::Networking,
        usage: &["ping example.com"],
        platforms: &["all"],
    },
    ExternalCommandSpec {
        name: "ipconfig",
        description: "Show Windows network configuration",
        category: CommandCategory::Networking,
        usage: &["ipconfig /all"],
        platforms: &["windows"],
    },
    ExternalCommandSpec {
        name: "ifconfig",
        description: "Show Unix network interfaces",
        category: CommandCategory::Networking,
        usage: &["ifconfig"],
        platforms: &["linux", "macos"],
    },
    ExternalCommandSpec {
        name: "ps",
        description: "List running processes",
        category: CommandCategory::System,
        usage: &["ps", "ps aux"],
        platforms: &["linux", "macos"],
    },
    ExternalCommandSpec {
        name: "systemctl",
        description: "Control systemd services",
        category: CommandCategory::System,
        usage: &["systemctl status ssh"],
        platforms: &["linux"],
    },
    ExternalCommandSpec {
        name: "uname",
        description: "Print operating system information",
        category: CommandCategory::System,
        usage: &["uname -a"],
        platforms: &["linux", "macos"],
    },
];

/// Builds the command catalog for the current host.
///
/// Built-ins are always available. External entries are resolved with the
/// shell's real `PATH` logic, so the explorer never reports a command as
/// available when the operating system cannot find it.
#[must_use]
pub fn command_catalog(state: &DesktopState) -> Vec<CommandCatalogEntry> {
    let working_directory = state.default_working_directory();
    let mut entries = Vec::new();

    for builtin in DEFAULT_BUILTINS.iter() {
        entries.push(CommandCatalogEntry {
            name: builtin.name().to_owned(),
            description: builtin.summary().to_owned(),
            category: builtin_category(builtin.name()),
            kind: CommandKind::Builtin,
            available: true,
            usage: builtin_usage(builtin.name()),
            platforms: vec!["all".to_owned()],
        });
    }

    for command in EXTERNAL_COMMANDS {
        if !command_supports_platform(command.platforms) {
            continue;
        }

        let available = resolve_executable(command.name, &working_directory).is_ok();

        entries.push(CommandCatalogEntry {
            name: command.name.to_owned(),
            description: command.description.to_owned(),
            category: command.category,
            kind: CommandKind::External,
            available,
            usage: command
                .usage
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
            platforms: command
                .platforms
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        });
    }

    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

/// Maps a built-in name onto its explorer category.
fn builtin_category(name: &str) -> CommandCategory {
    match name {
        "pwd" | "cd" | "ls" | "tree" | "find" | "stat" | "mkdir" | "touch" | "cp" | "mv" | "rm"
        | "which" | "basename" | "dirname" | "realpath" => CommandCategory::Filesystem,
        "cat" | "head" | "tail" | "wc" | "grep" | "sort" | "uniq" => {
            CommandCategory::TextProcessing
        }
        "whoami" | "date" | "jobs" | "wait" | "kill" | "fg" | "bg" | "sleep" | "host"
        | "sysinfo" => CommandCategory::System,
        _ => CommandCategory::CoreShell,
    }
}

/// Usage examples for built-in commands.
fn builtin_usage(name: &str) -> Vec<String> {
    let examples: &[&str] = match name {
        "help" => &["help"],
        "pwd" => &["pwd"],
        "cd" => &["cd ..", "cd ~/Projects"],
        "ls" => &["ls", "ls -a"],
        "tree" => &["tree"],
        "find" => &["find . -name \"*.rs\""],
        "stat" => &["stat file.txt"],
        "mkdir" => &["mkdir new-folder"],
        "touch" => &["touch file.txt"],
        "cat" => &["cat file.txt"],
        "head" => &["head -n 10 file.txt"],
        "tail" => &["tail -n 20 file.txt"],
        "wc" => &["wc file.txt"],
        "grep" => &["grep pattern file.txt"],
        "sort" => &["sort file.txt"],
        "uniq" => &["uniq file.txt"],
        "echo" => &["echo hello world"],
        "print" => &["print value"],
        "input" => &["input name"],
        "len" => &["len text"],
        "type" => &["type value"],
        "str" => &["str value"],
        "int" => &["int value"],
        "float" => &["float value"],
        "bool" => &["bool value"],
        "cp" => &["cp source destination"],
        "mv" => &["mv source destination"],
        "rm" => &["rm file.txt"],
        "which" => &["which command"],
        "basename" => &["basename path"],
        "dirname" => &["dirname path"],
        "realpath" => &["realpath path"],
        "clear" => &["clear"],
        "history" => &["history"],
        "jobs" => &["jobs"],
        "wait" => &["wait"],
        "kill" => &["kill job"],
        "fg" => &["fg job"],
        "bg" => &["bg job"],
        "run" => &["run script"],
        "test" => &["test -f file"],
        "[" => &["[ -f file ]"],
        "expr" => &["expr 1 + 2"],
        "sleep" => &["sleep 1"],
        "true" => &["true"],
        "false" => &["false"],
        "whoami" => &["whoami"],
        "date" => &["date"],
        "host" => &["host example.com"],
        "sysinfo" => &["sysinfo"],
        "version" => &["version"],
        "exit" => &["exit", "exit 0"],
        "export" => &["export NAME=value"],
        "unset" => &["unset NAME"],
        "env" => &["env"],
        "set" => &["set"],
        _ => &[],
    };

    examples
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
}

/// Whether the current OS is covered by a command's platform list.
fn command_supports_platform(platforms: &[&str]) -> bool {
    let current = std::env::consts::OS;

    platforms
        .iter()
        .any(|platform| *platform == "all" || *platform == current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::DesktopSettings;

    #[test]
    fn catalog_contains_all_builtins() {
        let state = DesktopState::new(
            DesktopSettings::default(),
            std::env::temp_dir().join("kernext-desktop-catalog-test"),
        );
        let catalog = command_catalog(&state);

        let builtin_count = catalog
            .iter()
            .filter(|entry| entry.kind == CommandKind::Builtin)
            .count();

        assert_eq!(builtin_count, DEFAULT_BUILTINS.len());
    }

    #[test]
    fn external_commands_are_resolved_on_this_host() {
        let state = DesktopState::new(
            DesktopSettings::default(),
            std::env::temp_dir().join("kernext-desktop-catalog-resolve-test"),
        );
        let catalog = command_catalog(&state);

        let external = catalog
            .iter()
            .filter(|entry| entry.kind == CommandKind::External)
            .count();

        assert!(external > 0, "the catalog should include external commands");
    }

    #[test]
    fn platform_lists_match_the_current_os() {
        assert!(command_supports_platform(&["all"]));
        assert!(command_supports_platform(&[std::env::consts::OS]));
        assert!(!command_supports_platform(&["not-a-platform"]));
    }
}
