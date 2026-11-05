//! Integration tests for the scripting runtime: loops, functions, control
//! flow, background sequencing, and execution limits.

use shell_builtins::Dispatcher;
use shell_runtime::Session;

fn session() -> Session {
    Session::new(Default::default()).expect("session should start")
}

fn run(dispatcher: &mut Dispatcher, session: &mut Session, input: &str) -> String {
    let outcome = dispatcher.execute(session, input);
    outcome.output().unwrap_or_default().to_owned()
}

#[test]
fn for_loop_iterates_over_words_and_expands_the_loop_variable() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    let output = run(
        &mut dispatcher,
        &mut session,
        "for name in alice bob { echo hello $name }",
    );

    assert_eq!(output, "hello alice\nhello bob\n");
}

#[test]
fn for_loop_splits_variables_and_supports_ascending_and_descending_ranges() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    let output = run(
        &mut dispatcher,
        &mut session,
        "names = \"alice bob\"; for name in $names { echo $name }; for i in 1..3 { echo $i }",
    );

    assert_eq!(output, "alice\nbob\n1\n2\n3\n");
}

#[test]
fn break_and_continue_only_affect_the_nearest_loop() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    let output = run(
        &mut dispatcher,
        &mut session,
        "for i in 1..5 { echo before; break; echo never }; for i in 1..1 { echo tick; continue; echo never }",
    );

    assert_eq!(output, "before\ntick\n");
}

#[test]
fn while_loop_terminates_with_break() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    let output = run(&mut dispatcher, &mut session, "while echo tick { break }");

    assert_eq!(output, "tick\n");
}

#[test]
fn break_continue_and_return_outside_their_context_report_errors() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    assert!(run(&mut dispatcher, &mut session, "break").contains("not inside a loop"));
    assert!(run(&mut dispatcher, &mut session, "continue").contains("not inside a loop"));
    assert!(run(&mut dispatcher, &mut session, "return 1").contains("not inside a function"));
}

#[test]
fn functions_receive_arguments_and_return_status() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    let output = run(
        &mut dispatcher,
        &mut session,
        "function greet(name) { echo hello $name; return 7 }; greet world; echo $?",
    );

    assert_eq!(output, "hello world\n7\n");
}

#[test]
fn function_parameters_do_not_leak_into_the_caller() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    let output = run(
        &mut dispatcher,
        &mut session,
        "name = outer; function show(name) { echo $name }; show inner; echo $name",
    );

    assert_eq!(output, "inner\nouter\n");
}

#[test]
fn recursive_functions_stop_at_the_configured_depth() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    let output = run(
        &mut dispatcher,
        &mut session,
        "function dive() { dive }; dive",
    );

    assert!(
        output.contains("maximum function recursion depth"),
        "{output}"
    );
}

#[test]
fn background_separator_continues_the_command_list() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    let output = run(
        &mut dispatcher,
        &mut session,
        "echo first & echo second; echo third",
    );

    assert_eq!(output, "first\nsecond\nthird\n");
}

#[test]
fn exit_inside_a_background_expression_does_not_stop_the_shell() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    let outcome = dispatcher.execute(&mut session, "exit & echo still-running");

    assert_eq!(outcome.action(), shell_builtins::CommandAction::Continue);
    assert_eq!(outcome.output(), Some("Goodbye.\nstill-running\n"));
}

#[test]
fn history_limit_changes_the_session_retention() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    dispatcher.execute(&mut session, "history limit 3");
    dispatcher.execute(&mut session, "echo one");
    dispatcher.execute(&mut session, "echo two");
    let outcome = dispatcher.execute(&mut session, "history");

    let output = outcome.output().expect("history should produce output");
    assert!(!output.contains("history limit 3"), "{output}");
    assert!(output.contains("echo one"), "{output}");
    assert!(output.contains("echo two"), "{output}");
    assert!(
        dispatcher.history().len() <= 3,
        "{:?}",
        dispatcher.history()
    );
}

#[test]
fn sensitive_commands_are_not_recorded_in_memory_history() {
    let mut dispatcher = Dispatcher::new();
    let mut session = session();

    dispatcher.execute(&mut session, "export API_TOKEN=super-secret");
    dispatcher.execute(&mut session, "echo visible");
    let outcome = dispatcher.execute(&mut session, "history");

    let output = outcome.output().expect("history should produce output");
    assert!(!output.contains("super-secret"), "{output}");
    assert!(!output.contains("API_TOKEN"), "{output}");
    assert!(output.contains("echo visible"), "{output}");
    assert!(
        dispatcher
            .history()
            .iter()
            .all(|entry| !entry.contains("super-secret"))
    );
}
