use anyhow::{anyhow, Result};
use command_line::CommandLineOptionParser;

pub use command_line::{ExecuteStatementOpt, FormatMode, Opt};
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
