use anyhow::{anyhow, Result};
use dirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use toml;
use tracing::debug;

use super::FormatMode;

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct ShellConfig {
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub debug: DebugConfig,
    pub default_ledger: Option<String>,
    pub ledgers: Option<Vec<LedgerConfig>>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct LedgerConfig {
    pub name: String,
    pub profile: Option<String>,
    pub region: Option<String>,
    pub qldb_session_endpoint: Option<String>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct UiConfig {
    #[serde(default = "default_true")]
    pub auto_commit: bool,

    pub prompt: Option<String>,

    #[serde(default)]
    pub format: FormatMode,

    #[serde(default)]
    pub edit_mode: EditMode,

    #[serde(default = "default_true")]
    pub display_welcome: bool,

    #[serde(default = "default_true")]
    pub display_ctrl_signals: bool,

    #[serde(default = "default_true")]
    pub display_query_metrics: bool,

    #[serde(default)]
    pub terminator_required: bool,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct DebugConfig {
    pub log: Option<PathBuf>,
}

fn default_true() -> bool {
    true
}

#[derive(StructOpt, Serialize, Deserialize, Clone, Debug)]
pub enum EditMode {
    Emacs,
    Vi,
}

impl Default for EditMode {
    fn default() -> Self {
        EditMode::Emacs
    }
}

impl ShellConfig {
    pub fn parse(config: impl AsRef<str>) -> Result<ShellConfig> {
        let config = toml::from_str(config.as_ref())?;
        Ok(config)
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<ShellConfig> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)?;
        ShellConfig::parse(contents)
            .map_err(|e| anyhow!("unable to load config at {}: {}", path.display(), e))
    }

    pub fn default_config_file_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().ok_or(anyhow!("$XDG_CONFIG_HOME not set"))?;
        let shell_dir = config_dir.join("qldbshell");
        fs::create_dir_all(&shell_dir)?;
        Ok(shell_dir.join("default_config.toml"))
    }

    pub fn load_default() -> Result<ShellConfig> {
        let config_file = ShellConfig::default_config_file_path()?;
        if !config_file.exists() {
            debug!(
                path = config_file.display().to_string().as_str(),
                "The default config file does not exist"
            );
            Ok(ShellConfig::default())
        } else {
            debug!(
                path = config_file.display().to_string().as_str(),
                "Loading config"
            );
            ShellConfig::load(&config_file)
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
        let _ = ShellConfig::load(&path)?;
        Ok(())
    }

    /// Loads a minimal config file.
    #[test]
    fn load_sample_config() -> Result<()> {
        let config = ShellConfig::parse(
            r#"
default_ledger = "my-ledger"

[[ledgers]]
name = "my-ledger"

[ui]
format = "table"
"#,
        )?;

        assert_eq!(&Some("my-ledger"), &config.default_ledger.as_deref());
        let ledgers = config.ledgers.unwrap();
        match ledgers.first() {
            Some(ledger) => {
                assert_eq!("my-ledger", &ledger.name);
            }
            None => panic!("config did not configure `my-ledger`"),
        };

        Ok(())
    }
}
