//! Comprehensive lexer tests.

use super::{LexerError, LexerErrorKind, Token, lex};

fn tokens(input: &str) -> Vec<Token> {
    lex(input).expect("input should lex")
}

fn word(value: &str) -> Token {
    Token::Word(value.to_owned())
}

fn string(value: &str) -> Token {
    Token::String(value.to_owned())
}

fn variable(value: &str) -> Token {
    Token::Variable(value.to_owned())
}

fn error(input: &str) -> LexerError {
    lex(input).expect_err("input should fail to lex")
}

fn assert_error(input: &str, kind: LexerErrorKind, line: usize, column: usize) {
    let error = error(input);

    assert_eq!(error.kind(), &kind);
    assert_eq!(error.line(), line);
    assert_eq!(error.column(), column);
}

#[test]
fn empty_input_contains_only_end_of_input() {
    assert_eq!(tokens(""), [Token::EndOfInput]);
}

#[test]
fn whitespace_only_input_is_skipped() {
    assert_eq!(tokens("   \t \u{00A0}"), [Token::EndOfInput]);
}

#[test]
fn basic_words_are_separated_by_whitespace() {
    assert_eq!(
        tokens("echo Hello pwd ls"),
        [
            word("echo"),
            word("Hello"),
            word("pwd"),
            word("ls"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn leading_trailing_and_repeated_whitespace_is_ignored() {
    assert_eq!(
        tokens("  \t echo   Hello \t "),
        [word("echo"), word("Hello"), Token::EndOfInput]
    );
}

#[test]
fn single_and_double_quoted_strings_are_decoded() {
    assert_eq!(
        tokens(r#"echo "Hello World" 'It is working'"#),
        [
            word("echo"),
            string("Hello World"),
            string("It is working"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn escaped_double_quotes_inside_double_quotes_are_decoded() {
    assert_eq!(
        tokens(r#"echo "Hello \"World\"""#),
        [word("echo"), string("Hello \"World\""), Token::EndOfInput]
    );
}

#[test]
fn escaped_single_quote_inside_single_quotes_is_decoded() {
    assert_eq!(
        tokens(r#"echo 'It\'s working'"#),
        [word("echo"), string("It's working"), Token::EndOfInput]
    );
}

#[test]
fn escaped_dollar_inside_strings_is_literal() {
    assert_eq!(
        tokens(r#"echo "\$HOME""#),
        [word("echo"), string("$HOME"), Token::EndOfInput]
    );
}

#[test]
fn escapes_outside_quotes_are_decoded() {
    assert_eq!(
        tokens(r"echo Hello\ World \$HOME \| \> \(x\)"),
        [
            word("echo"),
            word("Hello World"),
            word("$HOME"),
            word("|"),
            word(">"),
            word("(x)"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn escape_sequences_can_insert_control_characters() {
    assert_eq!(
        tokens(r"echo one\ntwo\tthree\r"),
        [word("echo"), word("one\ntwo\tthree\r"), Token::EndOfInput,]
    );
}

#[test]
fn escaped_quotes_outside_quotes_are_literal_word_characters() {
    assert_eq!(
        tokens(r#"echo \"Hello\""#),
        [word("echo"), word("\"Hello\""), Token::EndOfInput]
    );
}

#[test]
fn escaped_operator_characters_are_literal_word_characters() {
    assert_eq!(
        tokens(r"echo \|\|\&\&"),
        [word("echo"), word("||&&"), Token::EndOfInput]
    );
}

#[test]
fn pipes_and_logical_or_are_recognized() {
    assert_eq!(
        tokens("echo Hello | grep Hello || echo none"),
        [
            word("echo"),
            word("Hello"),
            Token::Pipe,
            word("grep"),
            word("Hello"),
            Token::Or,
            word("echo"),
            word("none"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn redirection_operators_are_recognized() {
    assert_eq!(
        tokens("echo Hello > output.txt >> log.txt < input.txt"),
        [
            word("echo"),
            word("Hello"),
            Token::OutputRedirect,
            word("output.txt"),
            Token::AppendRedirect,
            word("log.txt"),
            Token::InputRedirect,
            word("input.txt"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn command_chaining_operators_are_recognized() {
    assert_eq!(
        tokens("mkdir projects && cd projects ; echo Hello"),
        [
            word("mkdir"),
            word("projects"),
            Token::And,
            word("cd"),
            word("projects"),
            Token::Semicolon,
            word("echo"),
            word("Hello"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn operators_do_not_require_whitespace() {
    assert_eq!(
        tokens("a|b&&c||d>e>>f<g;h(i)"),
        [
            word("a"),
            Token::Pipe,
            word("b"),
            Token::And,
            word("c"),
            Token::Or,
            word("d"),
            Token::OutputRedirect,
            word("e"),
            Token::AppendRedirect,
            word("f"),
            Token::InputRedirect,
            word("g"),
            Token::Semicolon,
            word("h"),
            Token::LeftParenthesis,
            word("i"),
            Token::RightParenthesis,
            Token::EndOfInput,
        ]
    );
}

#[test]
fn parentheses_are_recognized() {
    assert_eq!(
        tokens("(echo Hello)"),
        [
            Token::LeftParenthesis,
            word("echo"),
            word("Hello"),
            Token::RightParenthesis,
            Token::EndOfInput,
        ]
    );
}

#[test]
fn variables_are_recognized_without_the_dollar_sign() {
    assert_eq!(
        tokens("echo $username $USER_NAME $naïve"),
        [
            word("echo"),
            variable("username"),
            variable("USER_NAME"),
            variable("naïve"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn variables_can_be_adjacent_to_words() {
    assert_eq!(
        tokens("foo$bar-baz"),
        [
            word("foo"),
            variable("bar"),
            word("-baz"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn variables_inside_quotes_are_literal_text() {
    assert_eq!(
        tokens(r#"echo "$username" '$username'"#),
        [
            word("echo"),
            string("$username"),
            string("$username"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn assignment_syntax_uses_words_and_a_string() {
    assert_eq!(
        tokens(r#"set username = "Trishit""#),
        [
            word("set"),
            word("username"),
            word("="),
            string("Trishit"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn empty_quoted_strings_are_tokens() {
    assert_eq!(
        tokens(r#"echo "" ''"#),
        [word("echo"), string(""), string(""), Token::EndOfInput,]
    );
}

#[test]
fn adjacent_word_and_string_segments_are_separate_tokens() {
    assert_eq!(
        tokens(r#"echo foo"bar"baz"#),
        [
            word("echo"),
            word("foo"),
            string("bar"),
            word("baz"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn unicode_words_and_strings_are_preserved() {
    assert_eq!(
        tokens(r#"echo café 'こんにちは世界' "Привет""#),
        [
            word("echo"),
            word("café"),
            string("こんにちは世界"),
            string("Привет"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn tabs_and_unicode_whitespace_separate_words() {
    assert_eq!(
        tokens("echo\tHello\u{00A0}World"),
        [
            word("echo"),
            word("Hello"),
            word("World"),
            Token::EndOfInput
        ]
    );
}

#[test]
fn strings_can_span_physical_lines() {
    assert_eq!(
        tokens("echo \"hello\nworld\""),
        [word("echo"), string("hello\nworld"), Token::EndOfInput]
    );
}

#[test]
fn newline_characters_become_newline_tokens() {
    assert_eq!(
        tokens("echo a\necho b\r\n"),
        [
            word("echo"),
            word("a"),
            Token::Newline,
            word("echo"),
            word("b"),
            Token::Newline,
            Token::EndOfInput,
        ]
    );
}

#[test]
fn a_lone_carriage_return_is_a_newline() {
    assert_eq!(
        tokens("echo a\recho b"),
        [
            word("echo"),
            word("a"),
            Token::Newline,
            word("echo"),
            word("b"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn consecutive_newlines_produce_consecutive_tokens() {
    assert_eq!(
        tokens("\n\n"),
        [Token::Newline, Token::Newline, Token::EndOfInput,]
    );
}

#[test]
fn hash_glob_and_question_mark_are_ordinary_word_characters() {
    assert_eq!(
        tokens("echo #comment *.txt ?"),
        [
            word("echo"),
            word("#comment"),
            word("*.txt"),
            word("?"),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn unterminated_single_quote_reports_the_opening_position() {
    assert_error(
        "echo 'unterminated",
        LexerErrorKind::UnterminatedSingleQuote,
        1,
        6,
    );
}

#[test]
fn unterminated_double_quote_reports_the_opening_position() {
    assert_error(
        "echo \"unterminated",
        LexerErrorKind::UnterminatedDoubleQuote,
        1,
        6,
    );
}

#[test]
fn unterminated_quote_reports_a_later_line() {
    assert_error(
        "echo ok\necho \"bad",
        LexerErrorKind::UnterminatedDoubleQuote,
        2,
        6,
    );
}

#[test]
fn invalid_escape_sequences_are_rejected() {
    assert_error(
        r"echo \q",
        LexerErrorKind::InvalidEscape { character: 'q' },
        1,
        6,
    );
}

#[test]
fn invalid_escape_inside_strings_is_rejected() {
    assert_error(
        r#"echo "bad \q""#,
        LexerErrorKind::InvalidEscape { character: 'q' },
        1,
        11,
    );
}

#[test]
fn dangling_escape_is_rejected() {
    assert_error("echo \\", LexerErrorKind::DanglingEscape, 1, 6);
}

#[test]
fn dangling_escape_inside_strings_is_rejected() {
    assert_error("echo \"bad \\", LexerErrorKind::DanglingEscape, 1, 11);
}

#[test]
fn lone_ampersand_is_rejected() {
    assert_error(
        "echo Hello & echo World",
        LexerErrorKind::UnexpectedCharacter { character: '&' },
        1,
        12,
    );
}

#[test]
fn control_characters_are_rejected() {
    assert_error(
        "echo \u{0001}",
        LexerErrorKind::UnexpectedCharacter {
            character: '\u{0001}',
        },
        1,
        6,
    );
}

#[test]
fn variable_without_a_name_is_rejected() {
    assert_error("echo $", LexerErrorKind::InvalidVariable, 1, 6);
}

#[test]
fn variable_with_a_digit_is_rejected() {
    assert_error("echo $1", LexerErrorKind::InvalidVariable, 1, 6);
}

#[test]
fn braced_variables_are_not_supported() {
    assert_error("echo ${name}", LexerErrorKind::InvalidVariable, 1, 6);
}

#[test]
fn error_columns_count_unicode_scalars() {
    assert_error("echo café $1", LexerErrorKind::InvalidVariable, 1, 11);
    assert_error(
        "echo café 'bad",
        LexerErrorKind::UnterminatedSingleQuote,
        1,
        11,
    );
    assert_error(
        "echo café \\q",
        LexerErrorKind::InvalidEscape { character: 'q' },
        1,
        11,
    );
}

#[test]
fn error_display_includes_line_column_and_message() {
    let error = error("echo 'bad");

    assert_eq!(
        error.to_string(),
        "lex error at line 1, column 6: unterminated single-quoted string"
    );
}

#[test]
fn invalid_escape_display_is_readable() {
    let error = error(r"echo \q");

    assert_eq!(
        error.to_string(),
        "lex error at line 1, column 6: invalid escape sequence `\\q`"
    );
}

#[test]
fn unexpected_character_display_is_readable() {
    let error = error("echo &");

    assert_eq!(
        error.to_string(),
        "lex error at line 1, column 6: unexpected character `&`"
    );
}
