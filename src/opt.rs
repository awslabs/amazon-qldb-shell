use anyhow::Result;
use std::str::FromStr;
use structopt::StructOpt;
use thiserror::Error;

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

    #[structopt(long = "--terminator-required")]
    pub terminator_required: bool,

    #[structopt(long = "--auto-commit", default_value = "on")]
    pub auto_commit: String,

    #[structopt(long = "--no-query-metrics")]
    pub no_query_metrics: bool,
}

#[derive(Debug)]
pub enum FormatMode {
    Ion,
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
