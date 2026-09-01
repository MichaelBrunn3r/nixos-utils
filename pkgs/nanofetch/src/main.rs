mod data_sources;
mod gather;
mod render;

use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

use crate::gather::gather;
use crate::render::render;

/// Gather system facts and render them as a table.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Print the gathered data as JSON.
    #[arg(long)]
    json: bool,

    /// Override the cache directory (default: the system temp dir).
    #[arg(long)]
    cache_dir: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let data = gather();

    if args.json {
        println!(
            "{}",
            serde_json::to_string(&data).context("failed to serialize data")?
        );
    } else {
        // Respect NO_COLOR (https://no-color.org)
        let colored = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
        println!("{}", render(&data, colored));
    }
    Ok(())
}
