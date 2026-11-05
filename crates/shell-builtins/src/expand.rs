//! Variable expansion for shell words and redirection targets.
//!
//! Expansion happens at evaluation time, after parsing. Each word in the AST
//! is expanded into a single string argument, preserving argument boundaries.

use shell_core::{Assignment, Redirection, Word, WordPart};
use shell_runtime::Session;

/// Expands a [`Word`] into its final string value.
///
/// Variable references ($name, ${name}, $?, $$) are replaced with their values.
/// Undefined variables expand to an empty string.
/// Single-quoted segments are never expanded.
pub fn expand_word(word: &Word, session: &Session) -> String {
    let mut result = String::new();

    for part in &word.parts {
        match part {
            WordPart::Unquoted { text, .. } => result.push_str(text),
            WordPart::SingleQuoted { text, .. } => result.push_str(text),
            WordPart::DoubleQuoted { text, .. } => result.push_str(text),
            WordPart::Variable { name, .. } => {
                if let Some(value) = session.get_variable(name) {
                    result.push_str(&value);
                }
            }
            WordPart::Special { kind, .. } => {
                let value = match kind {
                    shell_core::lexer::SpecialVariable::LastStatus => {
                        session.last_status().to_string()
                    }
                    shell_core::lexer::SpecialVariable::ProcessId => {
                        session.process_id().to_string()
                    }
                };
                result.push_str(&value);
            }
        }
    }

    result
}

/// Expands a redirection target word.
pub fn expand_redirection_target(redir: &Redirection, session: &Session) -> String {
    expand_word(&redir.target, session)
}

/// Expands an assignment value.
pub fn expand_assignment_value(assignment: &Assignment, session: &Session) -> String {
    expand_word(&assignment.value, session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_core::lexer::lex_spanned;
    use shell_core::parser::parse;
    use shell_runtime::{Config, Session};

    fn test_session() -> Session {
        let mut session = Session::with_environment(
            Config::default(),
            vec![
                ("PATH".to_owned(), "/usr/bin".to_owned()),
                ("HOME".to_owned(), "/home/user".to_owned()),
            ],
        )
        .expect("session");
        session.set_variable("TEST_VAR", "test_value".to_owned(), false);
        session.set_variable("EMPTY_VAR", "".to_owned(), false);
        session.set_last_status(42);
        session
    }

    fn parse_word(input: &str) -> Word {
        // Parse as "echo <input>" to get the argument word
        let tokens = lex_spanned(&format!("echo {input}")).expect("lex");
        // Debug: print tokens
        // eprintln!("Tokens for 'echo {}': {:?}", input, tokens);
        let ast = parse(&tokens).expect("parse");
        // Debug: print AST
        // eprintln!("AST: {:?}", ast);
        match &ast.list.items[0].expression {
            shell_core::Expression::Command(cmd) => {
                // eprintln!("Command args: {:?}", cmd.arguments);
                cmd.arguments[0].clone()
            }
            _ => panic!("expected command"),
        }
    }

    #[test]
    fn expands_simple_variable() {
        let session = test_session();
        let word = parse_word("$TEST_VAR");
        assert_eq!(expand_word(&word, &session), "test_value");
    }

    #[test]
    fn expands_braced_variable() {
        let session = test_session();
        let word = parse_word("${TEST_VAR}");
        assert_eq!(expand_word(&word, &session), "test_value");
    }

    #[test]
    fn expands_undefined_variable_to_empty() {
        let session = test_session();
        let word = parse_word("$UNDEFINED_VAR");
        assert_eq!(expand_word(&word, &session), "");
    }

    #[test]
    fn expands_special_last_status() {
        let session = test_session();
        let word = parse_word("$?");
        assert_eq!(expand_word(&word, &session), "42");
    }

    #[test]
    fn expands_special_process_id() {
        let session = test_session();
        let word = parse_word("$$");
        assert_eq!(
            expand_word(&word, &session),
            session.process_id().to_string()
        );
    }

    #[test]
    fn preserves_single_quoted_literal() {
        let session = test_session();
        let word = parse_word("'$TEST_VAR'");
        assert_eq!(expand_word(&word, &session), "$TEST_VAR");
    }

    #[test]
    fn expands_double_quoted_variable() {
        let session = test_session();
        // Use braces to delimit the variable name
        let word = parse_word("\"prefix_${TEST_VAR}_suffix\"");
        assert_eq!(expand_word(&word, &session), "prefix_test_value_suffix");
    }

    #[test]
    fn expands_multiple_variables_in_one_word() {
        let session = test_session();
        let word = parse_word("$TEST_VAR$EMPTY_VAR");
        assert_eq!(expand_word(&word, &session), "test_value");
    }

    #[test]
    fn expands_variable_adjacent_to_text() {
        let session = test_session();
        // Use braces to delimit the variable name
        let word = parse_word("prefix_${TEST_VAR}_suffix");
        eprintln!("Word parts: {:?}", word.parts);
        assert_eq!(expand_word(&word, &session), "prefix_test_value_suffix");
    }

    #[test]
    fn expands_inherited_variable() {
        let session = test_session();
        let word = parse_word("$HOME");
        assert_eq!(expand_word(&word, &session), "/home/user");
    }
}
