use std::{env, fs, process};

use djson::{
    eval::{evaluate_ast, stdlib},
    parser::Parser,
};

fn main() {
    if let Err(message) = run() {
        eprintln!("djson: {message}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: djson <file>".to_owned())?;
    let input =
        fs::read_to_string(&path).map_err(|error| format!("failed to read {path:?}: {error}"))?;
    let ast = Parser::new(&input)
        .parse()
        .map_err(|error| format!("failed to parse {path:?}: {error:?}"))?;
    let scope = stdlib::prelude();
    let value = evaluate_ast(&ast, &scope)
        .map_err(|error| format!("failed to evaluate {path:?}: {error:?}"))?;

    println!("{value:#?}");
    Ok(())
}
