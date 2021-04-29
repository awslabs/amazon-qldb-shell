use anyhow::Result;
use std::ffi::OsString;
use structopt::clap::AppSettings;
use structopt::StructOpt;

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
    Status,
    Set(SetCommand),
}

#[derive(StructOpt, Debug)]
pub enum SetCommand {
    InputMode(InputMode),
}

#[derive(StructOpt, Debug)]
pub enum InputMode {
    Emacs,
    Vi,
}

#[cfg(test)]
mod command_tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn set_input_mode() -> Result<()> {
        let backslash = super::backslash(&["set", "input-mode", "emacs"])?;
        if let Backslash::Set(SetCommand::InputMode(mode)) = backslash {
            assert!(matches!(mode, InputMode::Emacs));
        } else {
            panic!("failure, parsed to: {:?}", backslash);
        }

        Ok(())
    }
}
