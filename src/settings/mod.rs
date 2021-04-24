use anyhow::{anyhow, Result};
use command_line::CommandLineOptionParser;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;
use thiserror::Error;

pub use config::Config;

mod command_line;
mod config;

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

    pub fn apply_cli(&mut self, opt: &Opt) -> Result<()> {
        self.format.apply_value(&opt.format, Setter::CommandLine);
        self.ledger.apply_value(&opt.ledger, Setter::CommandLine);
        self.show_query_metrics
            .apply_value(&!opt.no_query_metrics, Setter::CommandLine);
        self.profile.apply_opt(&opt.profile, Setter::CommandLine);
        self.qldb_session_endpoint
            .apply_opt(&opt.qldb_session_endpoint, Setter::CommandLine);
        self.region.apply_opt(&opt.region, Setter::CommandLine);
        self.terminator_required
            .apply_value(&opt.terminator_required, Setter::CommandLine);

        let options = match opt.options {
            Some(ref o) => o,
            None => return Ok(()),
        };

        for unparsed in options {
            let supplied = CommandLineOptionParser::parse_on_off(unparsed)?;
            let existing = match &supplied.name[..] {
                "auto_commit" => &mut self.auto_commit,
                _ => Err(anyhow!("unknown option {}", supplied.name))?,
            };

            existing.apply_value(&supplied.value, Setter::CommandLine);
        }

        Ok(())
    }

    pub fn set_region(&mut self, region: String, setter: Setter) {
        self.region.apply_opt(&Some(region), setter)
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

    #[structopt(short, long, parse(from_os_str))]
    pub config: Option<PathBuf>,

    #[structopt(short = "-s", long = "--qldb-session-endpoint")]
    pub qldb_session_endpoint: Option<String>,

    #[structopt(short, long = "--profile")]
    pub profile: Option<String>,

    #[structopt(short, long = "--verbose", parse(from_occurrences))]
    /// Configure verbosity of logging. By default, only errors will be logged.
    /// Repeated usages of this (e.g. `-vv`) will increase the level. The
    /// highest level is `-vvv` which corresponds to `trace`.
    pub verbose: u8,

    #[structopt(short, long = "--format", default_value = "ion")]
    pub format: FormatMode,

    #[structopt(short, long = "--execute")]
    pub execute: Option<ExecuteStatementOpt>,

    #[structopt(short = "-o", long = "--opt")]
    pub options: Option<Vec<String>>,

    // FIXME: Deprecate the 3 below, replacing with `options`.
    #[structopt(long = "--terminator-required")]
    pub terminator_required: bool,

    #[structopt(long = "--no-query-metrics")]
    pub no_query_metrics: bool,
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
