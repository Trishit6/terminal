//! Comprehensive parser and AST tests.

use super::{
    Ast, Command, Expression, List, LogicalOperator, ParseError, ParseErrorKind, PipelineStage,
    RedirectionKind, Separator, Word, WordPart, parse,
};
use crate::lexer::{QuoteKind, lex_spanned};

fn parse_ok(input: &str) -> Ast {
    let tokens = lex_spanned(input).expect("input should lex");
    parse(&tokens).expect("input should parse")
}

fn parse_err(input: &str) -> ParseError {
    let tokens = lex_spanned(input).expect("input should lex");
    parse(&tokens).expect_err("input should fail to parse")
}

fn first_expression(ast: &Ast) -> &Expression {
    &ast.list.items[0].expression
}

fn first_command(ast: &Ast) -> &Command {
    match first_expression(ast) {
        Expression::Command(command) => command,
        other => panic!("expected a command, found {other:?}"),
    }
}

fn text(word: &Word) -> String {
    word.literal_text().expect("word should be literal")
}

fn assert_error(input: &str, kind: ParseErrorKind, line: usize, column: usize) {
    let error = parse_err(input);

    assert_eq!(error.kind(), &kind);
    assert_eq!(error.line(), line);
    assert_eq!(error.column(), column);
}

#[test]
fn parses_a_simple_command() {
    let ast = parse_ok(r#"echo "Hello World""#);
    let command = first_command(&ast);

    assert_eq!(ast.list.len(), 1);
    assert_eq!(text(&command.name), "echo");
    assert_eq!(command.arguments.len(), 1);
    assert!(command.redirections.is_empty());

    let argument = &command.arguments[0];
    assert_eq!(argument.parts.len(), 1);
    assert_eq!(text(argument), "Hello World");
    assert!(matches!(argument.parts[0], WordPart::DoubleQuoted { .. }));
}

#[test]
fn command_spans_cover_the_whole_command() {
    let ast = parse_ok(r#"echo "Hello World""#);
    let command = first_command(&ast);
    let argument = &command.arguments[0];

    assert_eq!(command.name.span.start().offset(), 0);
    assert_eq!(command.name.span.end().offset(), 4);
    assert_eq!(argument.span.start().offset(), 5);
    assert_eq!(argument.span.end().offset(), 18);
    assert_eq!(command.span.start().offset(), 0);
    assert_eq!(command.span.end().offset(), 18);
    assert_eq!(ast.span.start().offset(), 0);
    assert_eq!(ast.span.end().offset(), 18);
    assert_eq!(command.span.start().line(), 1);
    assert_eq!(command.span.start().column(), 1);
}

#[test]
fn parses_multiple_arguments_in_source_order() {
    let ast = parse_ok("cp source.txt target.txt backup.txt");
    let command = first_command(&ast);

    assert_eq!(text(&command.name), "cp");
    assert_eq!(command.arguments.len(), 3);
    assert_eq!(text(&command.arguments[0]), "source.txt");
    assert_eq!(text(&command.arguments[1]), "target.txt");
    assert_eq!(text(&command.arguments[2]), "backup.txt");
}

#[test]
fn preserves_single_and_double_quote_styles() {
    let ast = parse_ok(r#"echo 'single quoted' "double quoted""#);
    let command = first_command(&ast);

    assert!(matches!(
        command.arguments[0].parts[0],
        WordPart::SingleQuoted { .. }
    ));
    assert!(matches!(
        command.arguments[1].parts[0],
        WordPart::DoubleQuoted { .. }
    ));
    assert_eq!(text(&command.arguments[0]), "single quoted");
    assert_eq!(text(&command.arguments[1]), "double quoted");
}

#[test]
fn preserves_empty_quoted_arguments() {
    let ast = parse_ok(r#"echo "" ''"#);
    let command = first_command(&ast);

    assert_eq!(command.arguments.len(), 2);
    assert!(matches!(
        command.arguments[0].parts[0],
        WordPart::DoubleQuoted { .. }
    ));
    assert!(matches!(
        command.arguments[1].parts[0],
        WordPart::SingleQuoted { .. }
    ));
    assert_eq!(text(&command.arguments[0]), "");
    assert_eq!(text(&command.arguments[1]), "");
}

#[test]
fn parses_variable_references_without_expanding_them() {
    let ast = parse_ok("echo $HOME");
    let command = first_command(&ast);
    let argument = &command.arguments[0];

    assert_eq!(argument.parts.len(), 1);
    assert!(argument.has_variable());
    assert_eq!(argument.literal_text(), None);
    assert!(matches!(
        &argument.parts[0],
        WordPart::Variable { name, .. } if name == "HOME"
    ));
    assert_eq!(argument.parts[0].span().start().offset(), 5);
    assert_eq!(argument.parts[0].span().end().offset(), 10);
}

#[test]
fn concatenates_adjacent_word_and_variable_segments() {
    let ast = parse_ok(r#"echo pre$name"post""#);
    let command = first_command(&ast);

    assert_eq!(command.arguments.len(), 1);

    let argument = &command.arguments[0];
    assert_eq!(argument.parts.len(), 3);
    assert!(matches!(
        argument.parts[0],
        WordPart::Unquoted { ref text, .. } if text == "pre"
    ));
    assert!(matches!(
        argument.parts[1],
        WordPart::Variable { ref name, .. } if name == "name"
    ));
    assert!(matches!(
        argument.parts[2],
        WordPart::DoubleQuoted { ref text, .. } if text == "post"
    ));
    assert_eq!(argument.span.start().offset(), 5);
    assert_eq!(argument.span.end().offset(), 19);
}

#[test]
fn adjacent_literal_segments_form_one_word() {
    let ast = parse_ok(r#"echo pre"mid"'post'"#);
    let command = first_command(&ast);

    assert_eq!(command.arguments.len(), 1);
    assert_eq!(text(&command.arguments[0]), "premidpost");
}

#[test]
fn whitespace_separates_word_segments() {
    let ast = parse_ok(r#"echo foo "bar""#);
    let command = first_command(&ast);

    assert_eq!(command.arguments.len(), 2);
    assert_eq!(text(&command.arguments[0]), "foo");
    assert_eq!(text(&command.arguments[1]), "bar");
}

#[test]
fn parses_a_single_pipe() {
    let ast = parse_ok("cat file.txt | sort");

    let Expression::Pipeline(pipeline) = first_expression(&ast) else {
        panic!("expected a pipeline");
    };

    assert_eq!(pipeline.stages.len(), 2);
    assert!(matches!(pipeline.stages[0], PipelineStage::Command(_)));
    assert!(matches!(pipeline.stages[1], PipelineStage::Command(_)));

    let PipelineStage::Command(left) = &pipeline.stages[0] else {
        unreachable!();
    };
    let PipelineStage::Command(right) = &pipeline.stages[1] else {
        unreachable!();
    };

    assert_eq!(text(&left.name), "cat");
    assert_eq!(text(&right.name), "sort");
    assert_eq!(pipeline.span.start().offset(), 0);
    assert_eq!(pipeline.span.end().offset(), 19);
}

#[test]
fn parses_multiple_pipes_in_source_order() {
    let ast = parse_ok("cat file.txt | grep hello | sort");
    let Expression::Pipeline(pipeline) = first_expression(&ast) else {
        panic!("expected a pipeline");
    };

    assert_eq!(pipeline.stages.len(), 3);

    let names: Vec<String> = pipeline
        .stages
        .iter()
        .map(|stage| match stage {
            PipelineStage::Command(command) => text(&command.name),
            PipelineStage::Group(_) => panic!("unexpected group"),
        })
        .collect();

    assert_eq!(names, ["cat", "grep", "sort"]);
}

#[test]
fn pipeline_allows_a_newline_after_the_pipe() {
    let ast = parse_ok("cat file.txt |\ngrep hello");
    let Expression::Pipeline(pipeline) = first_expression(&ast) else {
        panic!("expected a pipeline");
    };

    assert_eq!(pipeline.stages.len(), 2);
}

#[test]
fn parses_input_redirection() {
    let ast = parse_ok("cat < input.txt");
    let command = first_command(&ast);

    assert_eq!(text(&command.name), "cat");
    assert!(command.arguments.is_empty());
    assert_eq!(command.redirections.len(), 1);

    let redirection = &command.redirections[0];
    assert_eq!(redirection.kind, RedirectionKind::Input);
    assert_eq!(text(&redirection.target), "input.txt");
    assert_eq!(redirection.operator_span.start().offset(), 4);
    assert_eq!(redirection.operator_span.end().offset(), 5);
    assert_eq!(redirection.span.start().offset(), 4);
    assert_eq!(redirection.span.end().offset(), 15);
}

#[test]
fn parses_output_redirection() {
    let ast = parse_ok("echo Hello > output.txt");
    let command = first_command(&ast);

    assert_eq!(command.redirections.len(), 1);
    assert_eq!(command.redirections[0].kind, RedirectionKind::Output);
    assert_eq!(text(&command.redirections[0].target), "output.txt");
}

#[test]
fn parses_append_redirection() {
    let ast = parse_ok("echo Hello >> output.txt");
    let command = first_command(&ast);

    assert_eq!(command.redirections.len(), 1);
    assert_eq!(command.redirections[0].kind, RedirectionKind::Append);
    assert_eq!(text(&command.redirections[0].target), "output.txt");
}

#[test]
fn parses_multiple_redirections_in_source_order() {
    let ast = parse_ok("cat < in.txt > out.txt >> log.txt");
    let command = first_command(&ast);

    assert_eq!(command.redirections.len(), 3);
    assert_eq!(command.redirections[0].kind, RedirectionKind::Input);
    assert_eq!(command.redirections[1].kind, RedirectionKind::Output);
    assert_eq!(command.redirections[2].kind, RedirectionKind::Append);
    assert_eq!(text(&command.redirections[0].target), "in.txt");
    assert_eq!(text(&command.redirections[1].target), "out.txt");
    assert_eq!(text(&command.redirections[2].target), "log.txt");
}

#[test]
fn redirections_can_be_interspersed_with_words() {
    let ast = parse_ok("echo > out.txt Hello");
    let command = first_command(&ast);

    assert_eq!(text(&command.name), "echo");
    assert_eq!(command.arguments.len(), 1);
    assert_eq!(text(&command.arguments[0]), "Hello");
    assert_eq!(command.redirections.len(), 1);
    assert_eq!(text(&command.redirections[0].target), "out.txt");
}

#[test]
fn redirection_targets_preserve_quotes_and_variables() {
    let ast = parse_ok(r#"echo Hello > "my file.txt""#);
    let command = first_command(&ast);
    let target = &command.redirections[0].target;

    assert!(matches!(
        target.parts[0],
        WordPart::DoubleQuoted { ref text, .. } if text == "my file.txt"
    ));
    assert_eq!(text(target), "my file.txt");
}

#[test]
fn parses_semicolon_separated_commands() {
    let ast = parse_ok("echo Hello ; echo World");

    assert_eq!(ast.list.len(), 2);
    assert_eq!(ast.list.items[0].separator, Some(Separator::Semicolon));
    assert_eq!(ast.list.items[1].separator, None);

    let Expression::Command(first) = &ast.list.items[0].expression else {
        panic!("expected a command");
    };
    let Expression::Command(second) = &ast.list.items[1].expression else {
        panic!("expected a command");
    };

    assert_eq!(text(&first.name), "echo");
    assert_eq!(text(&first.arguments[0]), "Hello");
    assert_eq!(text(&second.name), "echo");
    assert_eq!(text(&second.arguments[0]), "World");
}

#[test]
fn parses_newline_separated_commands() {
    let ast = parse_ok("echo Hello\necho World");

    assert_eq!(ast.list.len(), 2);
    assert_eq!(ast.list.items[0].separator, Some(Separator::Newline));
    assert_eq!(ast.list.items[1].separator, None);
}

#[test]
fn accepts_trailing_and_leading_separators() {
    let ast = parse_ok("\n echo Hello ;\n");

    assert_eq!(ast.list.len(), 1);
    assert_eq!(ast.list.items[0].separator, Some(Separator::Semicolon));
}

#[test]
fn parses_and_chaining() {
    let ast = parse_ok("mkdir projects && cd projects");
    let Expression::AndOr(expression) = first_expression(&ast) else {
        panic!("expected and/or expression");
    };

    assert_eq!(expression.operator, LogicalOperator::And);

    let Expression::Command(left) = &expression.left else {
        panic!("expected left command");
    };
    let Expression::Command(right) = &expression.right else {
        panic!("expected right command");
    };

    assert_eq!(text(&left.name), "mkdir");
    assert_eq!(text(&right.name), "cd");
}

#[test]
fn parses_or_chaining() {
    let ast = parse_ok(r#"false || echo "Fallback""#);
    let Expression::AndOr(expression) = first_expression(&ast) else {
        panic!("expected and/or expression");
    };

    assert_eq!(expression.operator, LogicalOperator::Or);
}

#[test]
fn logical_operators_are_left_associative() {
    let ast = parse_ok("a && b && c");
    let Expression::AndOr(outer) = first_expression(&ast) else {
        panic!("expected outer and expression");
    };

    assert_eq!(outer.operator, LogicalOperator::And);
    assert!(matches!(outer.right, Expression::Command(_)));

    let Expression::AndOr(inner) = &outer.left else {
        panic!("expected inner and expression");
    };
    assert_eq!(inner.operator, LogicalOperator::And);
}

#[test]
fn and_and_or_have_equal_precedence() {
    let ast = parse_ok("a && b || c");
    let Expression::AndOr(outer) = first_expression(&ast) else {
        panic!("expected outer and/or expression");
    };

    assert_eq!(outer.operator, LogicalOperator::Or);

    let Expression::AndOr(inner) = &outer.left else {
        panic!("expected inner and/or expression");
    };
    assert_eq!(inner.operator, LogicalOperator::And);
}

#[test]
fn pipelines_bind_tighter_than_logical_operators() {
    let ast = parse_ok("a | b && c | d");
    let Expression::AndOr(expression) = first_expression(&ast) else {
        panic!("expected and/or expression");
    };

    assert_eq!(expression.operator, LogicalOperator::And);
    assert!(matches!(expression.left, Expression::Pipeline(_)));
    assert!(matches!(expression.right, Expression::Pipeline(_)));
}

#[test]
fn logical_operator_allows_a_newline_after_it() {
    let ast = parse_ok("mkdir projects &&\ncd projects");
    let Expression::AndOr(expression) = first_expression(&ast) else {
        panic!("expected and/or expression");
    };

    assert_eq!(expression.operator, LogicalOperator::And);
}

#[test]
fn parses_parenthesized_groups() {
    let ast = parse_ok("(echo Hello ; echo World)");
    let Expression::Group(group) = first_expression(&ast) else {
        panic!("expected a group");
    };

    assert_eq!(group.list.len(), 2);
    assert_eq!(group.list.items[0].separator, Some(Separator::Semicolon));
    assert_eq!(group.span.start().offset(), 0);
    assert_eq!(group.span.end().offset(), 25);
    assert_eq!(ast.list.len(), 1);
}

#[test]
fn groups_can_contain_newline_separated_commands() {
    let ast = parse_ok("(echo Hello\necho World)");
    let Expression::Group(group) = first_expression(&ast) else {
        panic!("expected a group");
    };

    assert_eq!(group.list.len(), 2);
    assert_eq!(group.list.items[0].separator, Some(Separator::Newline));
}

#[test]
fn groups_can_be_logical_operands() {
    let ast = parse_ok("echo a && (echo b ; echo c)");
    let Expression::AndOr(expression) = first_expression(&ast) else {
        panic!("expected and/or expression");
    };

    assert!(matches!(expression.right, Expression::Group(_)));
}

#[test]
fn groups_can_be_pipeline_stages() {
    let ast = parse_ok("(echo a | grep a) | sort");
    let Expression::Pipeline(pipeline) = first_expression(&ast) else {
        panic!("expected a pipeline");
    };

    assert_eq!(pipeline.stages.len(), 2);
    assert!(matches!(pipeline.stages[0], PipelineStage::Group(_)));
    assert!(matches!(pipeline.stages[1], PipelineStage::Command(_)));
}

#[test]
fn groups_can_be_nested() {
    let ast = parse_ok("((echo Hello))");
    let Expression::Group(outer) = first_expression(&ast) else {
        panic!("expected outer group");
    };
    let Expression::Group(inner) = &outer.list.items[0].expression else {
        panic!("expected inner group");
    };

    assert_eq!(inner.list.len(), 1);
}

#[test]
fn empty_input_produces_an_empty_program() {
    let ast = parse_ok("");

    assert!(ast.is_empty());
    assert_eq!(ast.list.len(), 0);
    assert_eq!(ast.span.start().offset(), 0);
    assert_eq!(ast.span.end().offset(), 0);
}

#[test]
fn whitespace_only_input_produces_an_empty_program() {
    let ast = parse_ok("   \t \u{00A0}");

    assert!(ast.is_empty());
}

#[test]
fn blank_lines_produce_an_empty_program() {
    let ast = parse_ok(" \n \n\t");

    assert!(ast.is_empty());
}

#[test]
fn parses_unicode_words_and_strings() {
    let ast = parse_ok(r#"echo café 'こんにちは世界' "Привет""#);
    let command = first_command(&ast);

    assert_eq!(text(&command.arguments[0]), "café");
    assert_eq!(text(&command.arguments[1]), "こんにちは世界");
    assert_eq!(text(&command.arguments[2]), "Привет");
}

#[test]
fn unicode_columns_are_counted_in_scalars() {
    let ast = parse_ok("echo café");
    let command = first_command(&ast);

    assert_eq!(command.arguments[0].span.start().column(), 6);
    assert_eq!(command.arguments[0].span.end().column(), 10);
}

#[test]
fn missing_pipe_operand_is_reported_at_the_pipe() {
    assert_error("echo hello |", ParseErrorKind::MissingPipeOperand, 1, 12);
}

#[test]
fn repeated_pipe_without_a_command_is_reported() {
    assert_error(
        "echo hello | | sort",
        ParseErrorKind::MissingPipeOperand,
        1,
        12,
    );
}

#[test]
fn leading_pipe_is_an_unexpected_operator() {
    assert_error(
        "| echo hello",
        ParseErrorKind::UnexpectedOperator { operator: "|" },
        1,
        1,
    );
}

#[test]
fn leading_and_is_an_unexpected_operator() {
    assert_error(
        "&& echo hello",
        ParseErrorKind::UnexpectedOperator { operator: "&&" },
        1,
        1,
    );
}

#[test]
fn leading_or_is_an_unexpected_operator() {
    assert_error(
        "|| echo hello",
        ParseErrorKind::UnexpectedOperator { operator: "||" },
        1,
        1,
    );
}

#[test]
fn logical_operator_at_end_of_input_is_reported() {
    assert_error("echo hello &&", ParseErrorKind::UnexpectedEndOfInput, 1, 12);
}

#[test]
fn logical_operator_followed_by_separator_is_reported() {
    assert_error(
        "echo hello && ; echo world",
        ParseErrorKind::MissingOperand { operator: "&&" },
        1,
        12,
    );
}

#[test]
fn missing_output_redirection_target_is_reported() {
    assert_error(
        "echo hello >",
        ParseErrorKind::MissingRedirectionTarget,
        1,
        12,
    );
}

#[test]
fn missing_append_redirection_target_is_reported() {
    assert_error(
        "echo hello >>",
        ParseErrorKind::MissingRedirectionTarget,
        1,
        12,
    );
}

#[test]
fn missing_input_redirection_target_is_reported() {
    assert_error(
        "echo hello <",
        ParseErrorKind::MissingRedirectionTarget,
        1,
        12,
    );
}

#[test]
fn stacked_redirection_operator_is_reported() {
    assert_error(
        "echo hello > > file",
        ParseErrorKind::MissingRedirectionTarget,
        1,
        12,
    );
}

#[test]
fn unclosed_parenthesis_is_reported_at_the_opening_parenthesis() {
    assert_error("(echo hello", ParseErrorKind::UnclosedParenthesis, 1, 1);
}

#[test]
fn unclosed_nested_parenthesis_is_reported_at_the_inner_parenthesis() {
    assert_error("((echo hello", ParseErrorKind::UnclosedParenthesis, 1, 2);
}

#[test]
fn unexpected_closing_parenthesis_is_reported() {
    assert_error(
        "echo hello )",
        ParseErrorKind::UnexpectedClosingParenthesis,
        1,
        12,
    );
}

#[test]
fn empty_group_is_missing_a_command() {
    assert_error("()", ParseErrorKind::MissingCommand, 1, 1);
}

#[test]
fn empty_group_with_whitespace_is_missing_a_command() {
    assert_error("(  )", ParseErrorKind::MissingCommand, 1, 1);
}

#[test]
fn leading_semicolon_is_an_unexpected_operator() {
    assert_error(
        "; echo hello",
        ParseErrorKind::UnexpectedOperator { operator: ";" },
        1,
        1,
    );
}

#[test]
fn doubled_semicolon_is_an_unexpected_operator() {
    assert_error(
        "echo hello ;; echo world",
        ParseErrorKind::UnexpectedOperator { operator: ";" },
        1,
        13,
    );
}

#[test]
fn redirection_only_command_is_missing_a_command() {
    assert_error("> output.txt", ParseErrorKind::MissingCommand, 1, 1);
}

#[test]
fn missing_separator_is_reported() {
    assert_error(
        "echo hello (echo world)",
        ParseErrorKind::InvalidCommandStructure {
            message: "expected `;`, a newline, `&`, or end of input".to_string(),
        },
        1,
        12,
    );
}

#[test]
fn error_positions_track_later_lines() {
    assert_error(
        "echo ok\necho hello >\necho world",
        ParseErrorKind::MissingRedirectionTarget,
        2,
        12,
    );
}

#[test]
fn unicode_error_positions_are_counted_in_scalars() {
    assert_error("echo café |", ParseErrorKind::MissingPipeOperand, 1, 11);
}

#[test]
fn error_display_includes_line_column_and_message() {
    let error = parse_err("echo hello |");

    assert_eq!(
        error.to_string(),
        "parse error at line 1, column 12: expected a command after `|`"
    );
}

#[test]
fn unexpected_operator_display_is_readable() {
    let error = parse_err("&& echo hello");

    assert_eq!(
        error.to_string(),
        "parse error at line 1, column 1: unexpected operator `&&`"
    );
}

#[test]
fn program_span_covers_multiple_lines() {
    let ast = parse_ok("echo a\necho b");

    assert_eq!(ast.span.start().line(), 1);
    assert_eq!(ast.span.end().line(), 2);
    assert_eq!(ast.list.span.start().line(), 1);
    assert_eq!(ast.list.span.end().line(), 2);
}

#[test]
fn parse_rejects_empty_token_slices_without_panicking() {
    let ast = parse(&[]).expect("an empty token slice is an empty program");

    assert!(ast.is_empty());
}

#[test]
fn quote_metadata_is_available_on_token_streams() {
    let tokens = lex_spanned(r#"echo 'one' "two""#).expect("input should lex");

    assert_eq!(tokens[1].quote, Some(QuoteKind::Single));
    assert_eq!(tokens[2].quote, Some(QuoteKind::Double));
    assert_eq!(tokens[0].quote, None);
}

#[test]
fn list_type_helpers_are_consistent() {
    let ast = parse_ok("echo hello");
    let list: &List = &ast.list;

    assert_eq!(list.len(), 1);
    assert!(!list.is_empty());
}

#[test]
fn command_name_and_arguments_are_distinct() {
    let ast = parse_ok("git status --short");
    let command = first_command(&ast);

    assert_eq!(text(&command.name), "git");
    assert_eq!(command.arguments.len(), 2);
    assert_eq!(command.name.span.end().offset(), 3);
    assert_eq!(command.arguments[0].span.start().offset(), 4);
}
