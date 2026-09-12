mod repl;

use std::{fs, io, io::Read, path::PathBuf};

use clap::{Parser as ClapParser, Subcommand, ValueEnum};
use djson::{
    eval::{Scope, evaluate_ast, stdlib},
    parser::Parser,
};
use miette::{NamedSource, Report, miette};

use crate::cli::repl::Repl;

/// Parse and evaluate a djson document.
#[derive(ClapParser, Debug)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Choose the output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Djson)]
    fmt: OutputFormat,

    /// Read the document from this file instead of standard input.
    file: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Djson,
    Debug,
    #[cfg(feature = "fmt_json")]
    Json,
    #[cfg(feature = "fmt_yaml")]
    Yaml,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Evaluate one statement per input line interactively.
    Repl,
}

pub fn run() -> Result<(), Report> {
    let args = Args::parse();
    if matches!(args.command, Some(Command::Repl)) {
        return Repl::run();
    }

    let (source_name, input) = if let Some(path) = args.file {
        let input = fs::read_to_string(&path)
            .map_err(|error| miette!("failed to read {path:?}: {error}"))?;
        (path.display().to_string(), input)
    } else {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| miette!("failed to read standard input: {error}"))?;
        ("<stdin>".to_owned(), input)
    };
    let source = NamedSource::new(source_name, input.clone());
    let ast = Parser::new(&input)
        .parse_stmnts()
        .map_err(|error| Report::new(error).with_source_code(source.clone()))?;
    let mut scope = Scope::child(stdlib::prelude());
    let value = evaluate_ast(&ast, &mut scope)
        .map_err(|error| miette!("failed to evaluate document: {error:?}"))?;

    let output = render_value(&value, args.fmt)
        .map_err(|error| miette!("failed to render value: {error}"))?;
    println!("{output}");
    Ok(())
}

fn render_value(value: &djson::eval::Value, format: OutputFormat) -> Result<String, String> {
    match format {
        OutputFormat::Djson => Ok(djson_document(value)),
        OutputFormat::Debug => Ok(format!("{value:#?}")),
        #[cfg(feature = "fmt_json")]
        OutputFormat::Json => serde_json::to_string_pretty(&json_value(value)?)
            .map_err(|error| format!("failed to serialize JSON value: {error}")),
        #[cfg(feature = "fmt_yaml")]
        OutputFormat::Yaml => serde_yaml::to_string(&json_value(value)?)
            .map_err(|error| format!("failed to serialize YAML value: {error}")),
    }
}

fn djson_document(value: &djson::eval::Value) -> String {
    let rendered = djson_value(value);
    let Some(body) = rendered
        .strip_prefix("{\n")
        .and_then(|value| value.strip_suffix("\n}"))
    else {
        return rendered;
    };

    body.lines()
        .map(|line| line.strip_prefix("    ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn djson_value(value: &djson::eval::Value) -> String {
    use djson::eval::Value;

    fn quote(value: &str) -> String {
        let mut quoted = String::with_capacity(value.len() + 2);
        quoted.push('"');
        for character in value.chars() {
            match character {
                '\\' => quoted.push_str("\\\\"),
                '"' => quoted.push_str("\\\""),
                '\n' => quoted.push_str("\\n"),
                '\r' => quoted.push_str("\\r"),
                '\t' => quoted.push_str("\\t"),
                character => quoted.push(character),
            }
        }
        quoted.push('"');
        quoted
    }

    fn render(value: &Value, indent: usize) -> String {
        let indentation = " ".repeat(indent);
        let format_key = |key: &str| {
            let is_identifier = key.chars().enumerate().all(|(index, character)| {
                (index == 0 && (character.is_ascii_alphabetic() || character == '_'))
                    || (index > 0 && (character.is_ascii_alphanumeric() || character == '_'))
            });
            if is_identifier && !matches!(key, "true" | "false") {
                key.to_owned()
            } else {
                quote(key)
            }
        };
        match value {
            Value::None => "none".to_owned(),
            Value::Bool(value) => value.to_string(),
            Value::Int(value) => value.to_string(),
            Value::Float(value) if value.is_nan() => "import(\"std.math\").NAN".to_owned(),
            Value::Float(value) if value.is_infinite() && value.is_sign_positive() => {
                "import(\"std.math\").INFINITY".to_owned()
            }
            Value::Float(value) if value.is_infinite() => {
                "-import(\"std.math\").INFINITY".to_owned()
            }
            Value::Float(value) => value.to_string(),
            Value::Str(value) => quote(value),
            Value::List(values) => {
                if values.is_empty() {
                    return "[]".to_owned();
                }
                let values = values
                    .iter()
                    .map(|value| format!("{}{}", " ".repeat(indent + 4), render(value, indent + 4)))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("[\n{values}\n{indentation}]")
            }
            Value::Map(map) => {
                let entries = map
                    .clone()
                    .into_iter()
                    .map(|(key, value)| {
                        format!(
                            "{}{}: {}",
                            " ".repeat(indent + 4),
                            format_key(&key),
                            render(&value, indent + 4)
                        )
                    })
                    .collect::<Vec<_>>();
                if entries.is_empty() {
                    "{}".to_owned()
                } else {
                    format!("{{\n{}\n{indentation}}}", entries.join(",\n"))
                }
            }
            Value::Function(_) => "{ type: \"function\" }".to_owned(),
        }
    }

    render(value, 0)
}

#[cfg(any(feature = "fmt_json", feature = "fmt_yaml"))]
fn json_value(value: &djson::eval::Value) -> Result<serde_json::Value, String> {
    use djson::eval::Value;

    match value {
        Value::None => Ok(serde_json::Value::Null),
        Value::Bool(value) => Ok(serde_json::Value::Bool(*value)),
        Value::Int(value) => Ok(serde_json::Value::Number((*value).into())),
        Value::Float(value) => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .ok_or_else(|| format!("cannot represent {value} as JSON")),
        Value::Str(value) => Ok(serde_json::Value::String(value.clone())),
        Value::List(values) => values
            .iter()
            .map(json_value)
            .collect::<Result<Vec<_>, _>>()
            .map(serde_json::Value::Array),
        Value::Map(map) => map
            .clone()
            .into_iter()
            .map(|(key, value)| json_value(&value).map(|value| (key, value)))
            .collect::<Result<serde_json::Map<_, _>, _>>()
            .map(serde_json::Value::Object),
        Value::Function(_) => Ok(serde_json::json!({ "type": "function" })),
    }
}

#[cfg(all(test, feature = "fmt_json", feature = "fmt_yaml"))]
mod tests {
    use djson::{
        eval::{Scope, evaluate_ast, stdlib},
        parser::Parser,
    };

    use super::{OutputFormat, render_value};
    use crate::test_utils::{dedent, fmt_snapshot_case};

    #[test]
    fn renders_document_formats() {
        let input = dedent(
            r#"{
                none_value: none,
                bool_value: true,
                int_value: 42,
                float_value: 3.14,
                string_value: "Ada",
                list_value: [none, false, 7],
                map_value: { nested: "value", "needs quoting": true },
                function_value: import("std.assert").assert,
            }"#,
        );
        let ast = Parser::new(&input).parse_stmnts().expect("valid document");
        let mut scope = Scope::child(stdlib::prelude());
        let value = evaluate_ast(&ast, &mut scope).expect("evaluated document");
        let mut debug = render_value(&value, OutputFormat::Debug).expect("debug output");
        let address_start =
            debug.find("Function(\n").expect("function debug output") + "Function(\n".len();
        let address_end = address_start
            + debug[address_start..]
                .find('\n')
                .expect("function debug address");
        debug.replace_range(address_start..address_end, "<function>");
        let json = render_value(&value, OutputFormat::Json).expect("JSON output");
        let yaml = render_value(&value, OutputFormat::Yaml).expect("YAML output");
        let djson = render_value(&value, OutputFormat::Djson).expect("djson output");

        insta::assert_snapshot!(fmt_snapshot_case(
            "document",
            &[
                ("input", &input),
                ("djson", &djson),
                ("json", &json),
                ("yaml", &yaml),
                ("debug", &debug),
            ]
        ));
    }
}
