use super::{config::LedgerConfig, Opt};
use crate::{error, rusoto_driver, settings::ShellConfig};
use anyhow::{anyhow, Result};
use rusoto_core::Region;
use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(Clone)]
pub struct Environment {
    inner: Arc<Mutex<EnvironmentInner>>,
}

struct EnvironmentInner {
    current_ledger: LedgerConfig,
    current_region: Region,
    config: ShellConfig,
}

impl Environment {
    pub fn new(mut config: ShellConfig, cli: Opt) -> Result<Environment> {
        // First, update any config options based off `[cli]`.
        config.ui.format = cli.format;

        // Next, identify the current ledger and region.
        let ledger_name = match (cli.ledger, &config.default_ledger) {
            (None, None) => Err(error::usage_error(
                "`--ledger` was not specified and there is no `default_ledger` in your config",
                anyhow!("user error"),
            ))?,
            (None, Some(default)) => default.clone(),
            (Some(cli), _) => cli,
        };

        let mut current_ledger = LedgerConfig {
            name: ledger_name,
            profile: cli.profile,
            region: cli.region,
            qldb_session_endpoint: cli.qldb_session_endpoint.map(|url| url.to_string()),
        };

        // If there is a preconfigured ledger by this name, copy over any
        // default configuration not specified on the command line.
        if let Some(ref all) = config.ledgers {
            if let Some(preconfigured) = all.iter().find(|c| c.name == current_ledger.name) {
                if current_ledger.profile.is_none() {
                    current_ledger.profile = preconfigured.profile.clone();
                }

                if current_ledger.region.is_none() {
                    current_ledger.region = preconfigured.region.clone();
                }

                if current_ledger.qldb_session_endpoint.is_none() {
                    current_ledger.qldb_session_endpoint =
                        preconfigured.qldb_session_endpoint.clone();
                }
            }
        }

        let current_region = rusoto_driver::rusoto_region(
            current_ledger.region.as_ref(),
            current_ledger.qldb_session_endpoint.as_ref(),
        )?;

        let inner = EnvironmentInner {
            current_ledger,
            current_region,
            config,
        };

        Ok(Environment {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    pub(crate) fn config(&self) -> ShellConfigGuard {
        let guard = self.inner.lock().unwrap();
        ShellConfigGuard { guard }
    }

    pub(crate) fn current_ledger(&self) -> LedgerConfigGuard {
        let guard = self.inner.lock().unwrap();
        LedgerConfigGuard { guard }
    }

    pub(crate) fn current_region(&self) -> Region {
        let guard = self.inner.lock().unwrap();
        guard.current_region.clone()
    }

    pub(crate) fn update<F>(&self, update: F)
    where
        F: Fn(&mut LedgerConfig, &mut ShellConfig) -> (),
    {
        let mut inner = self.inner.lock().unwrap();
        let EnvironmentInner {
            current_ledger,
            config,
            ..
        } = inner.deref_mut();
        update(current_ledger, config)
    }

    /// When running in non-iteractive mode (e.g. using unix pipes to process
    /// data), when suppress chrome such as the welcome message.
    pub(crate) fn apply_noninteractive_defaults(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        let ui = &mut inner.deref_mut().config.ui;
        ui.display_welcome = false;
        ui.display_ctrl_signals = false;
    }
}

pub(crate) struct ShellConfigGuard<'a> {
    guard: MutexGuard<'a, EnvironmentInner>,
}

impl<'a> Deref for ShellConfigGuard<'a> {
    type Target = ShellConfig;

    fn deref(&self) -> &Self::Target {
        &self.guard.config
    }
}

pub(crate) struct LedgerConfigGuard<'a> {
    guard: MutexGuard<'a, EnvironmentInner>,
}

impl<'a> Deref for LedgerConfigGuard<'a> {
    type Target = LedgerConfig;

    fn deref(&self) -> &Self::Target {
        &self.guard.current_ledger
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.lock().unwrap();
        write!(f, "{:?}", inner.config)
    }
}
