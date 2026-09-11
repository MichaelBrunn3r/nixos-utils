mod repl;

use std::{fs, io, io::Read, path::PathBuf};

use clap::{Parser as ClapParser, Subcommand};
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

    /// Read the document from this file instead of standard input.
    file: Option<PathBuf>,
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
        .parse()
        .map_err(|error| Report::new(error).with_source_code(source.clone()))?;
    let mut scope = Scope::child(stdlib::prelude());
    let value = evaluate_ast(&ast, &mut scope)
        .map_err(|error| miette!("failed to evaluate document: {error:?}"))?;

    println!("{value:#?}");
    Ok(())
}
