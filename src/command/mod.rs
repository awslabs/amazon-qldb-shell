use anyhow::Result;
use std::convert::TryFrom;
use std::ffi::OsString;
use structopt::clap::AppSettings;
use structopt::StructOpt;
use url::Url;

use crate::settings::config::EditMode;

pub fn backslash<I>(iter: I) -> Result<Backslash>
where
    I: IntoIterator,
    I::Item: Into<OsString> + Clone,
{
    let clap = Backslash::clap();
    let clap = clap.setting(AppSettings::NoBinaryName);
    Ok(Backslash::from_clap(&clap.get_matches_from_safe(iter)?))
}

#[derive(StructOpt, Debug)]
#[structopt(name = "backslash", no_version)]
pub enum Backslash {
    Set(SetCommand),
    Use(UseCommand),
}

#[derive(StructOpt, Debug, Clone)]
pub enum TrueFalse {
    True,
    False,
}

impl From<&TrueFalse> for bool {
    fn from(tf: &TrueFalse) -> Self {
        match tf {
            TrueFalse::True => true,
            TrueFalse::False => false,
        }
    }
}

#[derive(StructOpt, Debug)]
pub enum SetCommand {
    EditMode(EditMode),
    TerminatorRequired(TrueFalse),
}

// FIXME: is there a way to share this with the main CLI opts?
#[derive(StructOpt, Debug)]
pub struct UseCommand {
    #[structopt(short, long = "--ledger")]
    pub ledger: Option<String>,
    #[structopt(short, long = "--region")]
    pub region: Option<String>,
    #[structopt(short = "-s", long = "--qldb-session-endpoint", parse(try_from_str = Url::try_from))]
    pub qldb_session_endpoint: Option<Url>,
    #[structopt(short, long = "--profile")]
    pub profile: Option<String>,
}

#[cfg(test)]
mod command_tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn set_input_mode() -> Result<()> {
        let backslash = super::backslash(&["set", "edit-mode", "emacs"])?;
        if let Backslash::Set(SetCommand::EditMode(mode)) = backslash {
            assert!(matches!(mode, EditMode::Emacs));
        } else {
            panic!("failure, parsed to: {:?}", backslash);
        }

        Ok(())
    }
}
