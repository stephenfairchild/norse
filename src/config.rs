use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Config {
    pub github: GithubConfig,
}

#[derive(Deserialize)]
pub struct GithubConfig {
    pub token: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let text = fs::read_to_string("config.toml")
            .context("could not read config.toml")?;
        toml::from_str(&text).context("invalid config.toml")
    }
}
