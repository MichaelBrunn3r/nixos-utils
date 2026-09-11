use std::{io, io::IsTerminal};

use djson::{
    eval::{EvalError, Scope, Value, evaluate_ast, stdlib},
    parser::{Parser, ParserError, ast::Statement},
};
use miette::{NamedSource, Report, miette};
use reedline::{
    ColumnarMenu, Completer, CompletionResult, DefaultPrompt, DefaultPromptSegment, KeyCode,
    KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal, Span, Suggestion, Vi,
    default_vi_insert_keybindings, default_vi_normal_keybindings,
};

pub(super) struct Repl {
    scope: Scope,
    state: State,
}

impl Repl {
    #[must_use]
    pub(super) fn new() -> Self {
        Self {
            scope: Scope::child(stdlib::prelude()),
            state: State::Running,
        }
    }

    pub(super) fn run() -> Result<(), Report> {
        let mut repl = Self::new();
        if io::stdin().is_terminal() {
            repl.run_interactive()
        } else {
            repl.run_piped()
        }
    }

    /// Reads lines through the `reedline` editor, which owns the prompt and history.
    fn run_interactive(&mut self) -> Result<(), Report> {
        let menu = ColumnarMenu::default().with_name(COMMAND_MENU);
        let mut editor = Reedline::create()
            .with_completer(Box::new(CommandCompleter))
            .with_menu(ReedlineMenu::EngineCompleter(Box::new(menu)))
            .with_edit_mode(Box::new(vi_edit_mode()));
        let prompt = DefaultPrompt::new(
            DefaultPromptSegment::Basic("djson".to_owned()),
            DefaultPromptSegment::Empty,
        );

        loop {
            match editor.read_line(&prompt) {
                Ok(Signal::Success(line)) => {
                    if report(self.submit(&line), &line) {
                        break;
                    }
                }
                Ok(Signal::CtrlD) => break,
                Ok(_) => {} // Ctrl+C and other signals discard the current line
                Err(error) => return Err(miette!("failed to read standard input: {error}")),
            }
        }

        Ok(())
    }

    /// Reads lines directly from standard input when it is not a terminal.
    fn run_piped(&mut self) -> Result<(), Report> {
        let stdin = io::stdin();
        let mut input = String::new();

        loop {
            input.clear();
            if stdin
                .read_line(&mut input)
                .map_err(|error| miette!("failed to read standard input: {error}"))?
                == 0
            {
                break;
            }

            if report(self.submit(&input), &input) {
                break;
            }
        }

        Ok(())
    }

    pub(super) fn submit(&mut self, line: &str) -> Event {
        if matches!(self.state, State::Exited) {
            return Event::Exit;
        }

        let line = line.trim();
        if line.is_empty() {
            return Event::NoOutput;
        }
        if let Some(command) = Command::parse(line) {
            return command.run(self);
        }

        evaluate_line(line, &mut self.scope).map_or_else(Event::Error, |value| {
            value.map_or(Event::NoOutput, Event::Value)
        })
    }
}

/// A REPL command, defined once and shared by parsing, completion, and help.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Help,
    Clear,
    Quit,
}

impl Command {
    /// Every command, in the order shown by completion and `:help`.
    const ALL: [Self; 3] = [Self::Help, Self::Clear, Self::Quit];

    /// Every spelling accepted at the prompt; the first is canonical.
    const fn names(self) -> &'static [&'static str] {
        match self {
            Self::Help => &[":help"],
            Self::Clear => &[":clear"],
            Self::Quit => &[":quit", ":q"],
        }
    }

    /// Canonical spelling, used by completion and `:help`.
    const fn name(self) -> &'static str {
        self.names()[0]
    }

    const fn description(self) -> &'static str {
        match self {
            Self::Help => "List the REPL commands",
            Self::Clear => "Reset the evaluation scope",
            Self::Quit => "Leave the REPL",
        }
    }

    /// Recognizes an exact command line, matching any accepted alias.
    fn parse(line: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|command| command.names().contains(&line))
    }

    /// Applies a command and returns the event it produces.
    fn run(self, repl: &mut Repl) -> Event {
        match self {
            Self::Quit => {
                repl.state = State::Exited;
                Event::Exit
            }
            Self::Help => Event::Help,
            Self::Clear => {
                repl.scope = Scope::child(stdlib::prelude());
                Event::Cleared
            }
        }
    }
}

#[derive(Debug)]
enum State {
    Running,
    Exited,
}

pub(super) enum Event {
    Value(Value),
    Help,
    Cleared,
    Error(InputError),
    Exit,
    NoOutput,
}

#[derive(Debug)]
pub(super) enum InputError {
    Parse(ParserError),
    MultipleStatements,
    Evaluation(EvalError),
}

fn evaluate_line(line: &str, scope: &mut Scope) -> Result<Option<Value>, InputError> {
    let ast = Parser::new(line)
        .parse_stmnts()
        .map_err(InputError::Parse)?;
    if ast.statements.len() != 1 {
        return Err(InputError::MultipleStatements);
    }

    let produces_value = !matches!(ast.statements[0], Statement::Let(_));
    evaluate_ast(&ast, scope)
        .map(|value| produces_value.then_some(value))
        .map_err(InputError::Evaluation)
}

/// Prints the outcome of a submitted line and reports whether the REPL should stop.
fn report(event: Event, line: &str) -> bool {
    match event {
        Event::Value(value) => println!("{value:#?}"),
        Event::Help => {
            println!("Enter one djson statement per line. Commands:");
            for command in Command::ALL {
                println!(
                    "  {:<14} {}",
                    command.names().join(", "),
                    command.description()
                );
            }
        }
        Event::Cleared | Event::NoOutput => {}
        Event::Exit => return true,
        Event::Error(InputError::Parse(error)) => {
            let source = NamedSource::new("<repl>", line.trim().to_owned());
            eprintln!("{}", Report::new(error).with_source_code(source));
        }
        Event::Error(InputError::MultipleStatements) => {
            eprintln!("error: the REPL accepts one statement per line");
        }
        Event::Error(InputError::Evaluation(error)) => eprintln!("error: {error:?}"),
    }

    false
}

/// Name shared by the command completion menu and the keybinding that opens it.
const COMMAND_MENU: &str = "command_menu";

/// Completes `:` commands such as `:quit`; any other line gets no suggestions.
struct CommandCompleter;

impl Completer for CommandCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
        let prefix = &line[..pos];
        if !prefix.starts_with(':') || prefix.contains(char::is_whitespace) {
            return CompletionResult::fresh(Vec::<Suggestion>::new());
        }

        let suggestions: Vec<Suggestion> = Command::ALL
            .iter()
            .filter(|command| command.name().starts_with(prefix))
            .map(|command| Suggestion {
                value: command.name().to_owned(),
                description: Some(command.description().to_owned()),
                span: Span::new(0, pos),
                ..Suggestion::default()
            })
            .collect();

        CompletionResult::fresh(suggestions)
    }
}

/// Vi editing, with Tab bound to the command completion menu in insert mode.
///
/// reedline starts in insert mode; Esc enters normal mode and the prompt
/// indicator reports the current mode.
fn vi_edit_mode() -> Vi {
    let mut insert = default_vi_insert_keybindings();
    insert.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu(COMMAND_MENU.to_owned()),
            ReedlineEvent::MenuNext,
        ]),
    );
    Vi::new(insert, default_vi_normal_keybindings())
}

#[cfg(test)]
mod tests {
    use djson::eval::Value;
    use reedline::{Completer, CompletionResult};

    use super::{Command, CommandCompleter, Event, InputError, Repl};

    #[test]
    fn every_accepted_spelling_is_handled() {
        for command in Command::ALL {
            for name in command.names() {
                let mut repl = Repl::new();
                assert!(
                    !matches!(repl.submit(name), Event::Error(_)),
                    "{name} was not handled"
                );
            }
        }
    }

    #[test]
    fn every_command_is_completable() {
        let mut completer = CommandCompleter;

        for command in Command::ALL {
            let name = command.name();
            let CompletionResult::Fresh { suggestions, .. } = completer.complete(name, name.len())
            else {
                panic!("expected fresh completions");
            };
            assert!(
                suggestions
                    .iter()
                    .any(|suggestion| suggestion.value == name),
                "{name} is missing from completion"
            );
        }
    }

    #[test]
    fn completes_commands_by_prefix() {
        let mut completer = CommandCompleter;

        let CompletionResult::Fresh { suggestions, .. } = completer.complete(":c", 2) else {
            panic!("expected fresh completions");
        };
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].value, ":clear");
    }

    #[test]
    fn ignores_lines_that_are_not_commands() {
        let mut completer = CommandCompleter;

        for line in ["answer", ":quit now", ""] {
            let CompletionResult::Fresh { suggestions, .. } = completer.complete(line, line.len())
            else {
                panic!("expected fresh completions");
            };
            assert!(
                suggestions.is_empty(),
                "expected no completions for {line:?}"
            );
        }
    }

    #[test]
    fn evaluates_expression_and_persists_bindings() {
        let mut repl = Repl::new();

        assert!(matches!(repl.submit("let answer = 40"), Event::NoOutput));
        assert!(matches!(
            repl.submit("answer + 2"),
            Event::Value(Value::Int(42))
        ));
    }

    #[test]
    fn rejects_multiple_statements() {
        let mut repl = Repl::new();

        assert!(matches!(
            repl.submit("let answer = 40\nanswer"),
            Event::Error(InputError::MultipleStatements)
        ));
    }

    #[test]
    fn returns_documents_as_values() {
        let mut repl = Repl::new();

        let Event::Value(Value::Map(value)) = repl.submit("answer: 42") else {
            panic!("expected a document value");
        };
        assert_eq!(value.get("answer"), Some(&Value::Int(42)));
    }

    #[test]
    fn commands_change_repl_state() {
        let mut repl = Repl::new();

        assert!(matches!(repl.submit(":help"), Event::Help));
        assert!(matches!(repl.submit(":clear"), Event::Cleared));
        assert!(matches!(repl.submit(":quit"), Event::Exit));
        assert!(matches!(repl.submit("1 + 1"), Event::Exit));
    }
}
