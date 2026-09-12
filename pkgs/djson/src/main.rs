mod cli;

#[cfg(test)]
#[allow(dead_code)]
#[path = "test_utils.rs"]
mod test_utils;

use std::process;

fn main() {
    if let Err(report) = cli::run() {
        eprintln!("djson: {report:?}");
        process::exit(1);
    }
}
