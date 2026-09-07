use std::{env, fs, process};

use yacl::{eval::evaluate_ast, parser::Parser, scope::Scope};

fn main() {
    if let Err(message) = run() {
        eprintln!("yacl: {message}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: yacl <file>".to_owned())?;
    let input =
        fs::read_to_string(&path).map_err(|error| format!("failed to read {path:?}: {error}"))?;
    let ast = Parser::new(&input)
        .parse()
        .map_err(|error| format!("failed to parse {path:?}: {error:?}"))?;
    let scope = Scope::root();
    let value = evaluate_ast(&ast, &scope)
        .map_err(|error| format!("failed to evaluate {path:?}: {error:?}"))?;

    println!("{value:#?}");
    Ok(())
}
