use anyhow::{anyhow, Result};
use dirs;
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{collections::HashMap, fs};
use thiserror::Error;
use toml;

use std::str::FromStr;
use structopt::StructOpt;

#[derive(Clone, Debug)]
pub enum Setter {
    Config,
    CommandLine,
    Environment,
}

#[derive(Clone, Debug)]
pub struct Setting<T: Clone> {
    name: String,
    modified: bool,
    setter: Setter,
    pub value: T,
}

impl<T> Setting<T>
where
    T: Clone,
{
    fn apply_value(&mut self, other: &T, setter: Setter) {
        self.modified = true;
        self.setter = setter;
        self.value = other.clone();
    }

    fn apply_value_opt(&mut self, other: &Option<T>, setter: Setter) {
        if let Some(value) = other {
            self.modified = true;
            self.setter = setter;
            self.value = value.clone();
        }
    }
}

impl<T> Setting<Option<T>>
where
    T: Clone,
{
    fn apply_opt(&mut self, other: &Option<T>, setter: Setter) {
        match (&self.value, other) {
            (None, None) => {}
            (Some(_), None) => {
                self.modified = true;
                self.setter = setter;
                self.value = None;
            }
            (_, Some(_)) => self.apply_value(other, setter),
        }
    }
}

#[derive(Parser)]
#[grammar = "settings.pest"]
pub struct SettingParser;

impl SettingParser {
    pub fn parse_bool(s: &str, setter: Setter) -> Result<Setting<bool>> {
        let assignment = SettingParser::parse(Rule::assignment, s)?.next().unwrap();
        let mut rule = assignment.into_inner();
        let name = rule.next().unwrap().as_str();
        let value = rule.next().unwrap().as_str();
        let value = match &value.to_lowercase()[..] {
            "" | "true" | "on" => true,
            "false" | "off" => false,
            _ => Err(anyhow!(
                "expecting 'name=enabled', where enabled is one of: true, false, on or off"
            ))?,
        };

        Ok(Setting {
            name: name.to_string(),
            modified: true,
            setter,
            value,
        })
    }
}

#[derive(Debug)]
pub struct CommandLineSetting(Setting<bool>);

impl FromStr for CommandLineSetting {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let setting = SettingParser::parse_bool(s, Setter::CommandLine)?;
        Ok(CommandLineSetting(setting))
    }
}

#[derive(Debug)]
pub struct Environment {
    pub auto_commit: Setting<bool>,
    pub format: Setting<FormatMode>,
    pub ledger: Setting<String>,
    pub prompt: Setting<String>,
    pub profile: Setting<Option<String>>,
    pub qldb_session_endpoint: Setting<Option<String>>,
    pub region: Setting<Option<String>>,
    pub show_query_metrics: Setting<bool>,
    pub terminator_required: Setting<bool>,
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            auto_commit: Setting {
                name: "auto_commit".to_string(),
                modified: false,
                setter: Setter::Environment,
                value: true,
            },
            format: Setting {
                name: "format".to_string(),
                modified: false,
                setter: Setter::Environment,
                value: FormatMode::Ion,
            },
            ledger: Setting {
                name: "ledger".to_string(),
                modified: false,
                setter: Setter::Environment,
                // FIXME: How to assert that there should be a ledger name specified
                value: "!!unknown".to_string(),
            },
            prompt: Setting {
                name: "prompt".to_string(),
                modified: false,
                setter: Setter::Environment,
                value: "qldb>".to_string(),
            },
            profile: Setting {
                name: "profile".to_string(),
                modified: false,
                setter: Setter::Environment,
                value: None,
            },
            qldb_session_endpoint: Setting {
                name: "qldb_session_endpoint".to_string(),
                modified: false,
                setter: Setter::Environment,
                value: None,
            },
            region: Setting {
                name: "region".to_string(),
                modified: false,
                setter: Setter::Environment,
                value: None,
            },
            show_query_metrics: Setting {
                name: "show_query_metrics".to_string(),
                modified: false,
                setter: Setter::Environment,
                value: true,
            },
            terminator_required: Setting {
                name: "terminator_required".to_string(),
                modified: false,
                setter: Setter::Environment,
                value: false,
            },
        }
    }

    pub fn apply_config(&mut self, config: &Config) {
        self.auto_commit
            .apply_value_opt(&config.auto_commit, Setter::Config);
        if let Some(ref ui) = config.ui {
            self.prompt.apply_value_opt(&ui.prompt, Setter::Config);
        }
    }

    pub fn apply_cli(&mut self, opt: &Opt) {
        match opt.auto_commit {
            AutoCommitMode::On => self.auto_commit.apply_value(&true, Setter::CommandLine),
            AutoCommitMode::Off => self.auto_commit.apply_value(&false, Setter::CommandLine),
        }
        self.format.apply_value(&opt.format, Setter::CommandLine);
        self.ledger.apply_value(&opt.ledger, Setter::CommandLine);
        self.show_query_metrics.apply_value(&!opt.no_query_metrics, Setter::CommandLine);
        self.profile.apply_opt(&opt.profile, Setter::CommandLine);
        self.qldb_session_endpoint
            .apply_opt(&opt.qldb_session_endpoint, Setter::CommandLine);
        self.region.apply_opt(&opt.region, Setter::CommandLine);
        self.terminator_required.apply_value(&opt.terminator_required, Setter::CommandLine);

        let options = match opt.options {
            Some(ref o) => o,
            None => return,
        };

        let mut named = HashMap::new();
        for setting in options {
            named.insert(setting.0.name.to_string(), setting.0.clone());
        }

        debug!("cli environment options: {:#?}", named);

        if let Some(setting) = named.get("auto_commit") {
            self.auto_commit
                .apply_value_opt(&Some(setting.value), Setter::CommandLine);
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct UiTomlTable {
    prompt: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    auto_commit: Option<bool>,
    ui: Option<UiTomlTable>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Config> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)
            .map_err(|e| anyhow!("unable to load config at {}: {}", path.display(), e))?;
        Ok(config)
    }

    pub fn load_default() -> Result<Config> {
        let config_dir = dirs::config_dir().ok_or(anyhow!("$XDG_CONFIG_HOME not set"))?;
        let shell_dir = config_dir.join("qldbshell");
        fs::create_dir_all(&shell_dir)?;
        let config_file = shell_dir.join("default_config.toml");
        if !config_file.exists() {
            Ok(Config::default())
        } else {
            Config::load(&config_file)
        }
    }
}

#[derive(Debug, StructOpt, Default)]
#[structopt(
    name = "qldb-shell",
    about = "A shell for interacting with Amazon QLDB."
)]
pub struct Opt {
    #[structopt(short, long = "--region")]
    pub region: Option<String>,

    #[structopt(short, long = "--ledger")]
    pub ledger: String,

    #[structopt(short = "-s", long = "--qldb-session-endpoint")]
    pub qldb_session_endpoint: Option<String>,

    #[structopt(short, long = "--profile")]
    pub profile: Option<String>,

    #[structopt(short, long = "--verbose")]
    pub verbose: bool,

    #[structopt(short, long = "--format", default_value = "ion")]
    pub format: FormatMode,

    #[structopt(short, long = "--execute")]
    pub execute: Option<ExecuteStatementOpt>,

    #[structopt(short = "-o", long = "--opt")]
    pub options: Option<Vec<CommandLineSetting>>,

    // FIXME: Deprecate the 3 below, replacing with `options`.
    #[structopt(long = "--terminator-required")]
    pub terminator_required: bool,

    #[structopt(long = "--auto-commit", default_value = "on")]
    pub auto_commit: AutoCommitMode,

    #[structopt(long = "--no-query-metrics")]
    pub no_query_metrics: bool,
}

#[derive(Debug, PartialEq)]
pub enum AutoCommitMode {
    On,
    Off,
}

impl Default for AutoCommitMode {
    fn default() -> Self {
        AutoCommitMode::On
    }
}

#[derive(Error, Debug)]
pub enum ParseAutoCommitModeErr {
    #[error("{0} is not a valid auto-commit mode")]
    InvalidAutoCommitMode(String),
}

impl FromStr for AutoCommitMode {
    type Err = ParseAutoCommitModeErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match &s.to_lowercase()[..] {
            "on" | "true" | "yes" => AutoCommitMode::On,
            "off" | "false" | "no" => AutoCommitMode::Off,
            _ => return Err(ParseAutoCommitModeErr::InvalidAutoCommitMode(s.into())),
        })
    }
}

#[derive(Debug, Clone)]
pub enum FormatMode {
    Ion,
    Table,
    // Removing a warning temporarily
    // Json,
}

impl Default for FormatMode {
    fn default() -> Self {
        FormatMode::Ion
    }
}

#[derive(Error, Debug)]
pub enum ParseFormatModeErr {
    #[error("{0} is not a valid format mode")]
    InvalidFormatMode(String),
}

impl FromStr for FormatMode {
    type Err = ParseFormatModeErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match &s.to_lowercase()[..] {
            "ion" | "ion-text" => FormatMode::Ion,
            "table" => FormatMode::Table,
            "json" => todo!("json is not yet supported"),
            _ => return Err(ParseFormatModeErr::InvalidFormatMode(s.into())),
        })
    }
}

#[derive(Debug)]
pub enum ExecuteStatementOpt {
    SingleStatement(String),
    Stdin,
}

impl FromStr for ExecuteStatementOpt {
    type Err = String; // never happens

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "-" => ExecuteStatementOpt::Stdin,
            _ => ExecuteStatementOpt::SingleStatement(s.into()),
        })
    }
}

#[cfg(test)]
mod settings_tests {
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
