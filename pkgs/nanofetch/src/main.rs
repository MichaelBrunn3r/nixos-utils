mod app;
mod data_sources;
mod gather;
#[cfg(test)]
#[allow(dead_code)]
mod render;

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

use crate::gather::gather;

/// Gather system facts and render them.
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
        println!("{}", app::render(&data));
    }
    Ok(())
}
