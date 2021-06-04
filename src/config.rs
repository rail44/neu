use anyhow::Result;
use serde_derive::Deserialize;
use std::fs;
use std::path::Path;

pub(crate) fn parse<P: AsRef<Path>>(p: P) -> Result<NeuConfig> {
    let s = fs::read_to_string(p)?;
    let config = toml::from_str(&s)?;
    Ok(config)
}

#[derive(Deserialize)]
pub(crate) struct NeuConfig {
    pub(crate) debug: bool,
}

impl Default for NeuConfig {
    fn default() -> Self {
        NeuConfig { debug: false }
    }
}
