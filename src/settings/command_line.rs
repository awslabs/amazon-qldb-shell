use crate::settings::{Setter, Setting};
use anyhow::Result;
use pest::Parser;
use pest_derive::Parser;
use std::path::PathBuf;
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

#[derive(Parser)]
#[grammar = "settings/command_line_options.pest"]
pub struct CommandLineOptionParser;

impl CommandLineOptionParser {
    pub fn parse_on_off(s: &str) -> Result<Setting<bool>> {
        let assignment = CommandLineOptionParser::parse(Rule::on_off_assignment, s)?
            .next()
            .unwrap();
        let mut rule = assignment.into_inner();
        let name = rule.next().unwrap().as_str();
        let value = match &rule.next().unwrap().as_str().to_lowercase()[..] {
            "on" => true,
            "off" => false,
            _ => unreachable!("by the grammar"),
        };

        Ok(Setting {
            name: name.to_string(),
            modified: true,
            setter: Setter::CommandLine,
            value,
        })
    }
}

#[cfg(test)]
mod settings_command_line_tests {
    use super::*;

    #[test]
    fn test_parse_on_off() -> Result<()> {
        assert_eq!(true, CommandLineOptionParser::parse_on_off("foo=on")?.value);
        assert_eq!(true, CommandLineOptionParser::parse_on_off("foo=ON")?.value);
        assert_eq!(
            false,
            CommandLineOptionParser::parse_on_off("foo=off")?.value
        );
        assert_eq!(
            false,
            CommandLineOptionParser::parse_on_off("foo=OFF")?.value
        );
        assert_eq!(
            true,
            CommandLineOptionParser::parse_on_off("foo=true").is_err()
        );

        Ok(())
    }
}
