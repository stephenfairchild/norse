use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Config {
    pub github: GithubConfig,
    #[serde(default)]
    pub model: ModelConfig,
}

#[derive(Deserialize)]
pub struct GithubConfig {
    pub token: String,
    pub orgs: Vec<String>,
}

#[derive(Deserialize, Default)]
pub struct ModelConfig {
    #[serde(default)]
    pub models: Vec<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = dirs::home_dir()
            .context("could not determine home directory")?
            .join(".norse");
        let text = fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("invalid {}", path.display()))
    }
}
