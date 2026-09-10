use std::{env, fs, process};

use djson::{
    eval::{evaluate_ast, stdlib},
    parser::Parser,
};
use miette::{NamedSource, Report, miette};

fn main() {
    if let Err(report) = run() {
        eprintln!("djson: {report:?}");
        process::exit(1);
    }
}

fn run() -> Result<(), Report> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| miette!("usage: djson <file>"))?;
    let input =
        fs::read_to_string(&path).map_err(|error| miette!("failed to read {path:?}: {error}"))?;
    let source = NamedSource::new(path.clone(), input.clone());
    let ast = Parser::new(&input)
        .parse()
        .map_err(|error| Report::new(error).with_source_code(source.clone()))?;
    let scope = stdlib::prelude();
    let value = evaluate_ast(&ast, &scope)
        .map_err(|error| miette!("failed to evaluate {path:?}: {error:?}"))?;

    println!("{value:#?}");
    Ok(())
}
