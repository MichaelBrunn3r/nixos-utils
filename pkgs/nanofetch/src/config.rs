use std::{fs, path::Path};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize)]
pub struct Config {
    pub(crate) dummy: bool,
}

impl Config {
    pub(crate) fn from_file(path: &Path) -> anyhow::Result<Self> {
        let yaml = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        Self::from_yaml(&yaml)
            .with_context(|| format!("failed to parse config file {}", path.display()))
    }

    pub(crate) fn from_yaml(yaml: &str) -> anyhow::Result<Self> {
        let partial = if yaml.trim().is_empty() {
            PartialConfig::default()
        } else {
            serde_yaml::from_str(yaml).context("failed to parse YAML")?
        };
        let mut config = Self::default();
        config.merge(&partial);
        Ok(config)
    }

    pub(crate) const fn merge(&mut self, partial: &PartialConfig) {
        if let Some(dummy) = partial.dummy {
            self.dummy = dummy;
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PartialConfig {
    dummy: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn yaml_overrides_default() {
        let config = Config::from_yaml("dummy: true").expect("valid YAML");

        assert!(config.dummy);
    }

    #[test]
    fn incomplete_yaml_keeps_defaults() {
        let config = Config::from_yaml("{}").expect("valid YAML");

        assert!(!config.dummy);
    }

    #[test]
    fn empty_yaml_keeps_defaults() {
        let config = Config::from_yaml("").expect("valid YAML");

        assert!(!config.dummy);
    }
}
