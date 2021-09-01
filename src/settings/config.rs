use anyhow::{anyhow, Result};
use dirs;
use ion_rs::value::owned::OwnedStruct;
use ion_rs::value::reader::{element_reader, ElementReader};
use ion_rs::value::{Element, Sequence, Struct};
use std::convert::TryFrom;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use structopt::StructOpt;
use tracing::debug;

use crate::error::{usage_error, ShellError};

use super::FormatMode;

#[derive(Default, Clone, Debug)]
pub struct ShellConfig {
    pub ui: UiConfig,
    pub debug: DebugConfig,
    pub default_ledger: Option<String>,
    pub ledgers: Option<Vec<LedgerConfig>>,
}

#[derive(Default, Clone, Debug)]
pub struct LedgerConfig {
    pub name: String,
    pub profile: Option<String>,
    pub region: Option<String>,
    pub qldb_session_endpoint: Option<String>,
}

#[derive(Default, Clone, Debug)]
pub struct UiConfig {
    pub auto_commit: bool,
    pub prompt: Option<String>,
    pub format: FormatMode,
    pub edit_mode: EditMode,
    pub display_welcome: bool,
    pub display_ctrl_signals: bool,
    pub display_query_metrics: bool,
    pub terminator_required: bool,
}

#[derive(Default, Clone, Debug)]
pub struct DebugConfig {
    pub log: Option<PathBuf>,
}

#[derive(StructOpt, Clone, Debug)]
pub enum EditMode {
    Emacs,
    Vi,
}

impl FromStr for EditMode {
    type Err = ShellError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match &s.to_lowercase()[..] {
            "emacs" => EditMode::Emacs,
            "vi" => EditMode::Vi,
            _ => return Err(usage_error(format!("{} is not a valid edit mode", s))),
        })
    }
}

impl Default for EditMode {
    fn default() -> Self {
        EditMode::Emacs
    }
}

impl TryFrom<&OwnedStruct> for ShellConfig {
    type Error = ShellError;

    fn try_from(value: &OwnedStruct) -> Result<Self, Self::Error> {
        let mut config = ShellConfig::default();

        if let Some(elem) = value.get("ui") {
            let ui = elem
                .as_struct()
                .ok_or(usage_error("`ui` should be a struct"))?;
            config.ui = UiConfig::try_from(ui)?;
        }

        if let Some(elem) = value.get("debug") {
            let debug = elem
                .as_struct()
                .ok_or(usage_error("`debug` should be a struct"))?;
            config.debug = DebugConfig::try_from(debug)?;
        }

        if let Some(elem) = value.get("default_ledger") {
            config.default_ledger = Some(
                elem.as_str()
                    .ok_or(usage_error("`default_ledger` should be a string"))?
                    .to_string(),
            );
        }

        if let Some(elem) = value.get("ledgers") {
            let ledgers = elem
                .as_sequence()
                .ok_or(usage_error("`ledgers` should be a list"))?;
            let deser: Result<Vec<_>, ShellError> = ledgers
                .iter()
                .map(|elem| {
                    let ledger = elem
                        .as_struct()
                        .ok_or(usage_error("`ledgers` should be a list of structs"))?;
                    LedgerConfig::try_from(ledger)
                })
                .collect();
            let deser = deser?;
            if !deser.is_empty() {
                config.ledgers = Some(deser);
            }
        }

        Ok(config)
    }
}

impl TryFrom<&OwnedStruct> for UiConfig {
    type Error = ShellError;

    fn try_from(value: &OwnedStruct) -> Result<Self, Self::Error> {
        let mut ui = UiConfig::default();

        ui.auto_commit = if let Some(elem) = value.get("auto_commit") {
            elem.as_bool()
                .ok_or(usage_error("`ui.auto_commit` should be a bool"))?
        } else {
            true
        };

        if let Some(elem) = value.get("prompt") {
            ui.prompt = Some(
                elem.as_str()
                    .ok_or(usage_error("`ui.prompt` should be a string"))?
                    .to_string(),
            );
        };

        if let Some(elem) = value.get("format") {
            let format = elem
                .as_str()
                .ok_or(usage_error("`ui.format` should be a string"))?;
            ui.format = FormatMode::from_str(format)?;
        }

        if let Some(elem) = value.get("edit_mode") {
            let edit_mode = elem
                .as_str()
                .ok_or(usage_error("`ui.edit_mode` should be a string"))?;
            ui.edit_mode = EditMode::from_str(edit_mode)?;
        }

        ui.display_welcome = if let Some(elem) = value.get("display_welcome") {
            elem.as_bool()
                .ok_or(usage_error("`ui.display_welcome` should be a bool"))?
        } else {
            true
        };

        ui.display_ctrl_signals = if let Some(elem) = value.get("display_ctrl_signals") {
            elem.as_bool()
                .ok_or(usage_error("`ui.display_ctrl_signals` should be a bool"))?
        } else {
            true
        };

        ui.display_query_metrics = if let Some(elem) = value.get("display_query_metrics") {
            elem.as_bool()
                .ok_or(usage_error("`ui.display_query_metrics` should be a bool"))?
        } else {
            true
        };

        if let Some(elem) = value.get("terminator_required") {
            ui.terminator_required = elem
                .as_bool()
                .ok_or(usage_error("`ui.terminator_required` should be a bool"))?
        }

        Ok(ui)
    }
}

impl TryFrom<&OwnedStruct> for DebugConfig {
    type Error = ShellError;

    fn try_from(value: &OwnedStruct) -> Result<Self, Self::Error> {
        let mut debug = DebugConfig::default();
        if let Some(elem) = value.get("log") {
            debug.log = Some(PathBuf::from(
                elem.as_str()
                    .ok_or(usage_error("`debug.log` should be a string"))?,
            ));
        }
        Ok(debug)
    }
}

impl TryFrom<&OwnedStruct> for LedgerConfig {
    type Error = ShellError;

    fn try_from(value: &OwnedStruct) -> Result<Self, Self::Error> {
        let mut ledger = LedgerConfig::default();

        ledger.name = match value.get("name") {
            Some(elem) => elem
                .as_str()
                .ok_or(usage_error("`ledger.name` should be a string"))?
                .to_string(),
            None => Err(usage_error("`ledger.name` is a required field"))?,
        };

        if let Some(elem) = value.get("profile") {
            ledger.profile = Some(
                elem.as_str()
                    .ok_or(usage_error("`ledger.profile` should be a string"))?
                    .to_string(),
            );
        }

        if let Some(elem) = value.get("region") {
            ledger.region = Some(
                elem.as_str()
                    .ok_or(usage_error("`ledger.region` should be a string"))?
                    .to_string(),
            );
        }

        if let Some(elem) = value.get("qldb_session_endpoint") {
            ledger.qldb_session_endpoint = Some(
                elem.as_str()
                    .ok_or(usage_error(
                        "`ledger.qldb_session_endpoint` should be a string",
                    ))?
                    .to_string(),
            );
        }

        Ok(ledger)
    }
}

impl ShellConfig {
    pub fn parse(serialized: &[u8]) -> Result<ShellConfig> {
        // At the time of writing, there is no serde support for Ion, so we
        // hydrate by hand.
        //
        // First, we pull out the Struct that contains all the config. There
        // should be 0 (empty file) or 1 of these.
        let mut reader = element_reader().iterate_over(serialized)?;
        let config = match reader.next() {
            Some(r) => {
                let elem = r?;
                ShellConfig::try_from(elem.as_struct().ok_or(usage_error(format!(
                    "config should contain a single struct (found a {:?})",
                    elem.ion_type()
                )))?)
            }
            None => Ok(ShellConfig::default()),
        }?;
        if let Some(_) = reader.next() {
            Err(usage_error(
                "config should contain a single struct (found multiple)",
            ))?
        }

        Ok(config)
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<ShellConfig> {
        let path = path.as_ref();
        let contents = fs::read(&path)?;
        Ok(ShellConfig::parse(&contents[..]).map_err(|e| {
            usage_error(format!(
                "unable to load config at {}: {}",
                path.display(),
                e
            ))
        })?)
    }

    pub fn default_config_file_path() -> Result<PathBuf> {
        let config_dir = config_dir_path().ok_or(usage_error("$XDG_CONFIG_HOME not set"))?;
        let shell_dir = config_dir.join("qldbshell");
        fs::create_dir_all(&shell_dir)?;
        Ok(shell_dir.join("config.ion"))
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

// On Macos, we override the XDG default of `~/Library/Application Support` to
// match the Linux convention of `~/.config`. See the discussion in #141 for why
// this decision was made.
//
// The code below is, essentially, the definition of `dirs::config_dir()` for
// Linux. We cannot call that directly because it is conditionally compiled, and
// thus we need our own slightly modified version of the code for Macos.
#[cfg(any(target_os = "macos", target_os = "ios"))]
fn config_dir_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .and_then(|path| {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                Some(path)
            } else {
                None
            }
        })
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
}

// On other platforms, we rely on the XDG default convention.
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn config_dir_path() -> Option<PathBuf> {
    dirs::config_dir()
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
        let path = tmp.path().join("empty.ion");
        File::create(&path)?;
        let _ = ShellConfig::load(&path)?;
        Ok(())
    }

    /// Loads a minimal config file.
    #[test]
    fn load_sample_config() -> Result<()> {
        let config = ShellConfig::parse(
            br#"
{
  default_ledger: "my-ledger",
  ui: {
    format: "table"
  },
  ledgers: [
    { name: "my-ledger" }
  ],
}
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
