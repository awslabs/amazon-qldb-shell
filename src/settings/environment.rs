use anyhow::{anyhow, Result};
use std::{
    fmt,
    sync::{Arc, Mutex},
};

use crate::settings::{command_line::CommandLineOptionParser, FormatMode};
use crate::settings::{Config, Setter, Setting};

use super::{config::EditMode, Opt};

#[derive(Clone)]
pub struct Environment {
    inner: Arc<Mutex<EnvironmentInner>>,
}

#[derive(Debug)]
struct EnvironmentInner {
    display_welcome: Setting<bool>,
    display_ctrl_signals: Setting<bool>,
    auto_commit: Setting<bool>,
    format: Setting<FormatMode>,
    ledger: Setting<String>,
    prompt: Setting<String>,
    profile: Setting<Option<String>>,
    qldb_session_endpoint: Setting<Option<String>>,
    region: Setting<Option<String>>,
    show_query_metrics: Setting<bool>,
    terminator_required: Setting<bool>,
    edit_mode: Setting<EditMode>,
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            inner: Arc::new(Mutex::new(EnvironmentInner {
                display_welcome: Setting {
                    name: "display_welcome".to_string(),
                    modified: false,
                    setter: Setter::Environment,
                    value: atty::is(atty::Stream::Stdin),
                },
                display_ctrl_signals: Setting {
                    name: "display_ctrl_signals".to_string(),
                    modified: false,
                    setter: Setter::Environment,
                    value: atty::is(atty::Stream::Stdin),
                },
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
                edit_mode: Setting {
                    name: "edit_mode".to_string(),
                    modified: false,
                    setter: Setter::Environment,
                    value: EditMode::Emacs,
                },
            })),
        }
    }

    pub(crate) fn apply_config(&mut self, config: &Config) {
        let mut inner = self.inner.lock().unwrap();
        inner.apply_config(config)
    }

    pub(crate) fn apply_cli(&mut self, opt: &Opt) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.apply_cli(opt)
    }

    pub(crate) fn display_welcome(&self) -> Setting<bool> {
        let inner = self.inner.lock().unwrap();
        inner.display_welcome.clone()
    }

    pub(crate) fn display_ctrl_signals(&self) -> Setting<bool> {
        let inner = self.inner.lock().unwrap();
        inner.display_ctrl_signals.clone()
    }

    pub(crate) fn auto_commit(&self) -> Setting<bool> {
        let inner = self.inner.lock().unwrap();
        inner.auto_commit.clone()
    }

    pub(crate) fn format(&self) -> Setting<FormatMode> {
        let inner = self.inner.lock().unwrap();
        inner.format.clone()
    }

    pub(crate) fn ledger(&self) -> Setting<String> {
        let inner = self.inner.lock().unwrap();
        inner.ledger.clone()
    }

    pub(crate) fn prompt(&self) -> Setting<String> {
        let inner = self.inner.lock().unwrap();
        inner.prompt.clone()
    }

    pub(crate) fn profile(&self) -> Setting<Option<String>> {
        let inner = self.inner.lock().unwrap();
        inner.profile.clone()
    }

    pub(crate) fn qldb_session_endpoint(&self) -> Setting<Option<String>> {
        let inner = self.inner.lock().unwrap();
        inner.qldb_session_endpoint.clone()
    }

    pub(crate) fn region(&self) -> Setting<Option<String>> {
        let inner = self.inner.lock().unwrap();
        inner.region.clone()
    }

    pub(crate) fn show_query_metrics(&self) -> Setting<bool> {
        let inner = self.inner.lock().unwrap();
        inner.show_query_metrics.clone()
    }

    pub(crate) fn terminator_required(&self) -> Setting<bool> {
        let inner = self.inner.lock().unwrap();
        inner.terminator_required.clone()
    }

    pub(crate) fn edit_mode(&self) -> Setting<EditMode> {
        let inner = self.inner.lock().unwrap();
        inner.edit_mode.clone()
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.lock().unwrap();
        write!(f, "{:?}", inner)
    }
}

impl EnvironmentInner {
    fn apply_config(&mut self, config: &Config) {
        self.auto_commit
            .apply_value_opt(&config.auto_commit, Setter::Config);
        if let Some(ref ui) = config.ui {
            self.prompt.apply_value_opt(&ui.prompt, Setter::Config);
            self.edit_mode
                .apply_value_opt(&ui.edit_mode, Setter::Config);
        }
    }

    fn apply_cli(&mut self, opt: &Opt) -> Result<()> {
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
}
