use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;
use url::Url;

use crate::error::{usage_error, ShellError};

#[derive(Debug, StructOpt, Default)]
/// A shell for interacting with Amazon QLDB. For more details,
/// see the shell topic in the QLDB Developer Guide: https://docs.aws.amazon.com/qldb/latest/developerguide/data-shell.html
#[structopt(name = "qldb")]
pub struct Opt {
    /// The AWS Region code of the QLDB ledger to connect to. For example: us-east-1.
    /// By default, the shell will pick a default region as described in the shell guide.
    #[structopt(short, long = "--region")]
    pub region: Option<String>,

    /// The name of the ledger to connect to. If a ledger with this name is
    /// configured in the config file, then additional configuration (such as
    /// the region) may be applied.
    #[structopt(short, long = "--ledger")]
    pub ledger: Option<String>,

    /// Config file to load. By default, this file is in
    /// $XDG_CONFIG_HOME/qldbshell/config.ion
    #[structopt(short, long, parse(from_os_str))]
    pub config: Option<PathBuf>,

    /// The qldb-session API endpoint to connect to.
    /// For a complete list of available QLDB Regions and endpoints, see the shell guide.
    #[structopt(short = "-s", long = "--qldb-session-endpoint", parse(try_from_str = Url::try_from))]
    pub qldb_session_endpoint: Option<Url>,

    /// The location of your AWS credentials profile to use for authentication.
    /// By default, the shell will pick a default profile as described in the shell guide.
    #[structopt(short, long = "--profile")]
    pub profile: Option<String>,

    /// Configure verbosity of logging. By default, only errors will be logged.
    /// Repeated usages of this (e.g. `-vv`) will increase the level. The
    /// highest level is `-vvv` which corresponds to `trace`.
    #[structopt(short, long = "--verbose", parse(from_occurrences))]
    pub verbose: u8,

    /// The output format of your query results. By default, the format is ion.
    #[structopt(short, long = "--format")]
    pub format: Option<FormatMode>,
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

impl FromStr for FormatMode {
    type Err = ShellError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match &s.to_lowercase()[..] {
            "ion" | "ion-text" => FormatMode::Ion,
            "table" => FormatMode::Table,
            "json" => todo!("json is not yet supported"),
            _ => return Err(usage_error(format!("{} is not a valid format mode", s))),
        })
    }
}
