mod app;
pub(crate) mod config;
mod data_sources;
mod gather;
mod percentage;

use std::{env, path::PathBuf, str::FromStr};

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

    #[arg(long, default_value = "", hide_default_value = true)]
    cfg: ConfigSource,

    /// Override the cache directory (default: the system temp dir).
    #[arg(long)]
    cache_dir: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();
    let _config = args.cfg.load()?;

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

//region ConfigSource
/// Describes where the configuration should be loaded from.
#[derive(Clone, Debug)]
enum ConfigSource {
    /// Resolve a config file automatically, falling back to defaults.
    Resolve,
    /// Load a config file from this path.
    Path(PathBuf),
    /// Parse this inline YAML document.
    InlineYaml(String),
    /// Use the built-in default configuration.
    Default,
}

impl ConfigSource {
    /// Load a configuration from this source.
    fn load(self) -> anyhow::Result<config::Config> {
        match self {
            Self::Resolve => Self::resolve_path().map_or_else(
                || Ok(config::Config::default()),
                |path| config::Config::from_file(&path),
            ),
            Self::Path(path) => config::Config::from_file(&path),
            Self::InlineYaml(yaml) => config::Config::from_yaml(&yaml),
            Self::Default => Ok(config::Config::default()),
        }
    }

    /// Find the first existing config file path.
    ///
    /// Search order:
    /// 1. `./nanofetch.yaml`
    /// 2. `./.nanofetch.yaml`
    /// 3. `~/nanofetch.yaml`
    /// 4. `~/.nanofetch.yaml`
    /// 5. `$XDG_CONFIG_HOME/nanofetch/config.yaml`
    /// 6. `~/.config/nanofetch/config.yaml`
    fn resolve_path() -> Option<PathBuf> {
        let mut paths = vec![
            PathBuf::from("nanofetch.yaml"),
            PathBuf::from(".nanofetch.yaml"),
        ];
        if let Some(home) = env::var_os("HOME") {
            let home = PathBuf::from(home);
            paths.push(home.join("nanofetch.yaml"));
            paths.push(home.join(".nanofetch.yaml"));
        }
        if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
            paths.push(PathBuf::from(config_home).join("nanofetch/config.yaml"));
        }
        if let Some(home) = env::var_os("HOME") {
            paths.push(PathBuf::from(home).join(".config/nanofetch/config.yaml"));
        }
        paths.into_iter().find(|path| path.is_file())
    }
}

impl FromStr for ConfigSource {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Ok(Self::Resolve);
        }
        if matches!(value, "default") {
            return Ok(Self::Default);
        }
        if let Some(yaml) = value.strip_prefix("yaml=") {
            return Ok(Self::InlineYaml(yaml.to_owned()));
        }
        Ok(Self::Path(PathBuf::from(value)))
    }
}

//endregion ConfigSource
