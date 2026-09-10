mod cli;

use std::process;

fn main() {
    if let Err(report) = cli::run() {
        eprintln!("djson: {report:?}");
        process::exit(1);
    }
}
