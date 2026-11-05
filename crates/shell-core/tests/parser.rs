//! Public-API integration tests for the parser and AST.

use shell_core::{
    Command, Expression, LogicalOperator, ParseErrorKind, PipelineStage, RedirectionKind,
    Separator, WordPart, lex_spanned, parse,
};

fn parse_input(input: &str) -> shell_core::Ast {
    let tokens = lex_spanned(input).expect("input should lex");
    parse(&tokens).expect("input should parse")
}

fn command_from(expression: &Expression) -> &Command {
    match expression {
        Expression::Command(command) => command,
        other => panic!("expected a command, found {other:?}"),
    }
}

#[test]
fn public_api_parses_a_command_with_quotes_and_redirection() {
    let ast = parse_input(r#"echo "Hello World" > output.txt"#);
    let command = command_from(&ast.list.items[0].expression);

    assert_eq!(command.name.literal_text().as_deref(), Some("echo"));
    assert_eq!(command.arguments.len(), 1);
    assert_eq!(
        command.arguments[0].literal_text().as_deref(),
        Some("Hello World")
    );
    assert!(matches!(
        command.arguments[0].parts[0],
        WordPart::DoubleQuoted { .. }
    ));
    assert_eq!(command.redirections.len(), 1);
    assert_eq!(command.redirections[0].kind, RedirectionKind::Output);
    assert_eq!(
        command.redirections[0].target.literal_text().as_deref(),
        Some("output.txt")
    );
}

#[test]
fn public_api_preserves_variable_metadata() {
    let ast = parse_input(r#"echo pre$USER"-suffix""#);
    let command = command_from(&ast.list.items[0].expression);
    let argument = &command.arguments[0];

    assert_eq!(argument.parts.len(), 3);
    assert!(argument.has_variable());
    assert_eq!(argument.literal_text(), None);
    assert!(matches!(
        &argument.parts[1],
        WordPart::Variable { name, .. } if name == "USER"
    ));
    assert!(matches!(
        &argument.parts[2],
        WordPart::DoubleQuoted { text, .. } if text == "-suffix"
    ));
}

#[test]
fn public_api_parses_pipelines_and_logical_chaining() {
    let ast = parse_input("cat file.txt | grep hello && echo found || echo missing");
    let Expression::AndOr(outer) = &ast.list.items[0].expression else {
        panic!("expected an and/or expression");
    };

    assert_eq!(outer.operator, LogicalOperator::Or);

    let Expression::AndOr(inner) = &outer.left else {
        panic!("expected an inner and/or expression");
    };
    assert_eq!(inner.operator, LogicalOperator::And);
    assert!(matches!(inner.left, Expression::Pipeline(_)));
}

#[test]
fn public_api_parses_sequential_commands() {
    let ast = parse_input("echo Hello ; echo World");

    assert_eq!(ast.list.len(), 2);
    assert_eq!(ast.list.items[0].separator, Some(Separator::Semicolon));
    assert_eq!(ast.list.items[1].separator, None);
}

#[test]
fn public_api_parses_parenthesized_groups() {
    let ast = parse_input("(echo Hello ; echo World)");
    let Expression::Group(group) = &ast.list.items[0].expression else {
        panic!("expected a group");
    };

    assert_eq!(group.list.len(), 2);
    assert_eq!(group.list.items[0].separator, Some(Separator::Semicolon));
}

#[test]
fn public_api_exposes_source_spans() {
    let ast = parse_input("echo hello");
    let command = command_from(&ast.list.items[0].expression);

    assert_eq!(ast.span.start().line(), 1);
    assert_eq!(ast.span.start().column(), 1);
    assert_eq!(ast.span.end().offset(), 10);
    assert_eq!(command.name.span.start().offset(), 0);
    assert_eq!(command.name.span.end().offset(), 4);
    assert_eq!(command.arguments[0].span.start().offset(), 5);
}

#[test]
fn public_api_reports_parse_errors_with_locations() {
    let tokens = lex_spanned("echo hello >").expect("input should lex");
    let error = parse(&tokens).expect_err("input should fail to parse");

    assert_eq!(error.kind(), &ParseErrorKind::MissingRedirectionTarget);
    assert_eq!(error.line(), 1);
    assert_eq!(error.column(), 12);
    assert_eq!(
        error.to_string(),
        "parse error at line 1, column 12: expected a redirection target"
    );
}

#[test]
fn public_api_reports_unclosed_parentheses() {
    let tokens = lex_spanned("(echo hello").expect("input should lex");
    let error = parse(&tokens).expect_err("input should fail to parse");

    assert_eq!(error.kind(), &ParseErrorKind::UnclosedParenthesis);
    assert_eq!(error.line(), 1);
    assert_eq!(error.column(), 1);
}

#[test]
fn public_api_empty_input_is_an_empty_program() {
    let ast = parse_input("   \n\t ");

    assert!(ast.is_empty());
    assert_eq!(ast.list.len(), 0);
}

#[test]
fn public_api_handles_unicode_input_and_error_columns() {
    let ast = parse_input(r#"echo café "Привет""#);
    let command = command_from(&ast.list.items[0].expression);

    assert_eq!(command.arguments[0].literal_text().as_deref(), Some("café"));
    assert_eq!(
        command.arguments[1].literal_text().as_deref(),
        Some("Привет")
    );

    let tokens = lex_spanned("echo café |").expect("input should lex");
    let error = parse(&tokens).expect_err("input should fail to parse");

    assert_eq!(error.kind(), &ParseErrorKind::MissingPipeOperand);
    assert_eq!(error.line(), 1);
    assert_eq!(error.column(), 11);
}

#[test]
fn public_api_never_panics_on_empty_token_slices() {
    let ast = parse(&[]).expect("empty token streams are accepted");

    assert!(ast.is_empty());
}

#[test]
fn public_api_parses_every_redirection_kind() {
    let ast = parse_input("cat < in.txt > out.txt >> append.txt 2> err.txt 2>> err2.txt");
    let command = command_from(&ast.list.items[0].expression);

    let kinds: Vec<RedirectionKind> = command
        .redirections
        .iter()
        .map(|redirection| redirection.kind)
        .collect();

    assert_eq!(
        kinds,
        [
            RedirectionKind::Input,
            RedirectionKind::Output,
            RedirectionKind::Append,
            RedirectionKind::Error,
            RedirectionKind::ErrorAppend,
        ]
    );
}

#[test]
fn public_api_parses_multiple_pipeline_stages_with_redirection() {
    let ast = parse_input("cat in.txt | sort | uniq > out.txt");
    let Expression::Pipeline(pipeline) = &ast.list.items[0].expression else {
        panic!("expected a pipeline");
    };

    assert_eq!(pipeline.stages.len(), 3);

    let PipelineStage::Command(last) = &pipeline.stages[2] else {
        panic!("expected a simple command");
    };
    assert_eq!(last.name.literal_text().as_deref(), Some("uniq"));
    assert_eq!(last.redirections.len(), 1);
    assert_eq!(last.redirections[0].kind, RedirectionKind::Output);
    assert_eq!(
        last.redirections[0].target.literal_text().as_deref(),
        Some("out.txt")
    );
}

#[test]
fn public_api_pipelines_bind_more_tightly_than_logical_operators() {
    let ast = parse_input("echo a | sort && echo b");
    let Expression::AndOr(and_or) = &ast.list.items[0].expression else {
        panic!("expected an and/or expression");
    };

    assert_eq!(and_or.operator, LogicalOperator::And);
    assert!(matches!(and_or.left, Expression::Pipeline(_)));
    assert!(matches!(and_or.right, Expression::Command(_)));
}

#[test]
fn public_api_quoted_operators_are_ordinary_text() {
    let ast = parse_input(r#"echo "a && b" 'c | d' "> file""#);
    let command = command_from(&ast.list.items[0].expression);

    assert_eq!(command.arguments.len(), 3);
    assert_eq!(
        command.arguments[0].literal_text().as_deref(),
        Some("a && b")
    );
    assert_eq!(
        command.arguments[1].literal_text().as_deref(),
        Some("c | d")
    );
    assert_eq!(
        command.arguments[2].literal_text().as_deref(),
        Some("> file")
    );
    assert!(command.redirections.is_empty());
}

#[test]
fn public_api_preserves_quoted_redirection_paths() {
    let ast = parse_input(r#"echo hello > "out file.txt""#);
    let command = command_from(&ast.list.items[0].expression);

    assert_eq!(command.redirections.len(), 1);
    assert_eq!(
        command.redirections[0].target.literal_text().as_deref(),
        Some("out file.txt")
    );
}

#[test]
fn public_api_reports_missing_error_redirection_targets() {
    let tokens = lex_spanned("cmd 2>").expect("input should lex");
    let error = parse(&tokens).expect_err("input should fail to parse");

    assert_eq!(error.kind(), &ParseErrorKind::MissingRedirectionTarget);
    assert_eq!(error.line(), 1);
    assert_eq!(error.column(), 5);
}
