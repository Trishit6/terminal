//! Command-history privacy helpers.
//!
//! The shell can keep password- and token-bearing commands out of both its
//! in-memory history and the persistent Reedline history file. The check is
//! deliberately conservative: it looks for well-known credential names used as
//! assignments or command-line options, and never tries to inspect the command's
//! output.

/// Variable/option names that almost always carry credentials.
const SENSITIVE_NAMES: &[&str] = &[
    "password",
    "passwd",
    "passphrase",
    "secret",
    "token",
    "api_key",
    "api_token",
    "apikey",
    "access_token",
    "refresh_token",
    "access_key",
    "secret_key",
    "private_key",
    "client_secret",
    "auth_token",
    "authorization",
    "credentials",
    "aws_secret_access_key",
    "github_token",
    "npm_token",
    "database_url",
];

/// Returns `true` when a command line looks like it contains a credential.
///
/// The matching covers:
///
/// - `NAME=value`, `NAME:value`
/// - `--name=value`, `--name value`, `-name value`
/// - `export NAME=value`
/// - PEM private-key blocks
///
/// It intentionally does not attempt to detect every possible secret. It is a
/// privacy guard, not a security boundary.
#[must_use]
pub fn is_sensitive_command(command: &str) -> bool {
    if command.contains("-----BEGIN") && command.contains("PRIVATE KEY") {
        return true;
    }

    let words: Vec<&str> = command.split_whitespace().map(trim_token).collect();
    if words.iter().copied().any(is_sensitive_word) {
        return true;
    }

    // Spaced assignments such as `PASSWORD = value` are common in scripting
    // shells and must be filtered just like `PASSWORD=value`.
    words.windows(2).any(|pair| {
        is_sensitive_name(pair[0]) && (pair[1].starts_with('=') || pair[1].starts_with(':'))
    })
}

fn trim_token(word: &str) -> &str {
    word.trim_matches(|character: char| {
        matches!(
            character,
            '"' | '\'' | '(' | ')' | ';' | '|' | '&' | ',' | '{' | '}'
        )
    })
}

fn is_sensitive_name(word: &str) -> bool {
    let normalized = word
        .trim_start_matches('-')
        .to_ascii_lowercase()
        .replace('-', "_");
    SENSITIVE_NAMES.contains(&normalized.as_str())
}

fn is_sensitive_word(word: &str) -> bool {
    let trimmed = trim_token(word);

    if trimmed.is_empty() {
        return false;
    }

    let (body, had_option_prefix) = match trimmed.strip_prefix('-') {
        Some(rest) => (rest.trim_start_matches('-'), true),
        None => (trimmed, false),
    };

    let normalized = body.to_ascii_lowercase().replace('-', "_");

    // `NAME=value` and `NAME:value`.
    if let Some((key, value)) = split_key_value(&normalized) {
        if SENSITIVE_NAMES.contains(&key) {
            // `client-secret: value` splits into two words, so a trailing
            // separator still marks the name as credential-bearing.
            return !value.is_empty() || body.ends_with(':') || body.ends_with('=');
        }
        return false;
    }

    // A bare `--password` is followed by its value in the next argument.
    if had_option_prefix && SENSITIVE_NAMES.contains(&normalized.as_str()) {
        return true;
    }

    false
}

fn split_key_value(word: &str) -> Option<(&str, &str)> {
    let (key, value) = word.split_once('=').or_else(|| word.split_once(':'))?;

    if key.is_empty() {
        return None;
    }

    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::is_sensitive_command;

    #[test]
    fn plain_commands_are_not_sensitive() {
        for command in [
            "ls -la",
            "echo hello",
            "git status",
            "cat secrets.txt",
            "export PATH=/usr/bin",
            "cargo test -p shell-core",
        ] {
            assert!(!is_sensitive_command(command), "{command}");
        }
    }

    #[test]
    fn assignments_with_credential_names_are_sensitive() {
        for command in [
            "export PASSWORD=hunter2",
            "TOKEN=abc123",
            "API_KEY='secret-value'",
            "DATABASE_URL=postgres://user:pass@host/db",
            "AWS_SECRET_ACCESS_KEY=abc",
            "client-secret: value",
            "export PASSWORD = hunter2",
            "export API_KEY : abc123",
            "tool --password = hunter2",
        ] {
            assert!(is_sensitive_command(command), "{command}");
        }
    }

    #[test]
    fn credential_options_are_sensitive() {
        for command in [
            "tool --password hunter2",
            "tool --password=hunter2",
            "tool --token abc",
            "curl --header Authorization:Bearer",
            "tool -api-key abc",
        ] {
            assert!(is_sensitive_command(command), "{command}");
        }
    }

    #[test]
    fn private_key_blocks_are_sensitive() {
        assert!(is_sensitive_command(
            "cat '-----BEGIN RSA PRIVATE KEY-----'"
        ));
    }
}
