//! Public-API integration tests for the lexer.

use shell_core::{LexerErrorKind, Token, lex};

#[test]
fn public_api_lexes_a_compound_command() {
    let tokens = lex(r#"echo "Hello World" | grep Hello > output.txt"#).expect("valid input");

    assert_eq!(
        tokens,
        [
            Token::Word("echo".to_owned()),
            Token::String("Hello World".to_owned()),
            Token::Pipe,
            Token::Word("grep".to_owned()),
            Token::Word("Hello".to_owned()),
            Token::OutputRedirect,
            Token::Word("output.txt".to_owned()),
            Token::EndOfInput,
        ]
    );
}

#[test]
fn public_api_exposes_error_locations() {
    let error = lex("echo 'unterminated").expect_err("input should fail");

    assert_eq!(error.kind(), &LexerErrorKind::UnterminatedSingleQuote);
    assert_eq!(error.line(), 1);
    assert_eq!(error.column(), 6);
}
