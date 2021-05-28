use anyhow::Result;
use serde_derive::Deserialize;
use std::fs;
use std::path::Path;
use toml;

pub(crate) fn parse<P: AsRef<Path>>(p: P) -> Result<Config> {
    let s = fs::read_to_string(p)?;
    let config = toml::from_str(&s)?;
    Ok(config)
}

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) debug: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { debug: false }
    }
}
