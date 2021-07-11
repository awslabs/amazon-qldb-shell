use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;
use thiserror::Error;
use url::Url;

#[derive(Debug, StructOpt, Default)]
#[structopt(
    name = "qldb-shell",
    about = "A shell for interacting with Amazon QLDB."
)]
pub struct Opt {
    #[structopt(short, long = "--region")]
    pub region: Option<String>,

    /// The name of the ledger to connect to. If a ledger with this name is
    /// configured in the config file, then additional configuration (such as
    /// the region) may be applied.
    #[structopt(short, long = "--ledger")]
    pub ledger: Option<String>,

    /// Config file to load. By default, this file is in
    /// $XDG_CONFIG_HOME/qldbshell/default_config.toml.
    #[structopt(short, long, parse(from_os_str))]
    pub config: Option<PathBuf>,

    #[structopt(short = "-s", long = "--qldb-session-endpoint", parse(try_from_str = Url::try_from))]
    pub qldb_session_endpoint: Option<Url>,

    #[structopt(short, long = "--profile")]
    pub profile: Option<String>,

    /// Configure verbosity of logging. By default, only errors will be logged.
    /// Repeated usages of this (e.g. `-vv`) will increase the level. The
    /// highest level is `-vvv` which corresponds to `trace`.
    #[structopt(short, long = "--verbose", parse(from_occurrences))]
    pub verbose: u8,

    #[structopt(short, long = "--format", default_value = "ion")]
    pub format: FormatMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
