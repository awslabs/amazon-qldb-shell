use anyhow::{anyhow, Result};
use dirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use toml;

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub auto_commit: Option<bool>,
    pub ui: Option<UiTomlTable>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct UiTomlTable {
    pub prompt: Option<String>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Config> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)
            .map_err(|e| anyhow!("unable to load config at {}: {}", path.display(), e))?;
        Ok(config)
    }

    pub fn default_config_file_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().ok_or(anyhow!("$XDG_CONFIG_HOME not set"))?;
        let shell_dir = config_dir.join("qldbshell");
        fs::create_dir_all(&shell_dir)?;
        Ok(shell_dir.join("default_config.toml"))
    }

    pub fn load_default() -> Result<Config> {
        let config_file = Config::default_config_file_path()?;
        if !config_file.exists() {
            Ok(Config::default())
        } else {
            Config::load(&config_file)
        }
    }
}

#[cfg(test)]
mod settings_config_tests {
    use super::*;
    use fs::File;
    use tempdir::TempDir;

    /// Tests that an empty config is valid. This makes sure we don't forget an
    /// `Optional` in any new fields we add.
    #[test]
    fn load_empty_config() -> Result<()> {
        let tmp = TempDir::new("config")?;
        let path = tmp.path().join("empty.toml");
        File::create(&path)?;
        let _ = Config::load(&path)?;
        Ok(())
    }
}
