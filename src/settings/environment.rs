use super::{config::LedgerConfig, Opt};
use crate::{awssdk_driver, error, settings::ShellConfig};
use anyhow::Result;
use aws_sdk_qldbsession::Region;
use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard},
};

#[derive(Clone)]
pub struct Environment {
    inner: Arc<RwLock<EnvironmentInner>>,
}

pub(crate) struct EnvironmentInner {
    pub(crate) current_ledger: LedgerConfig,
    pub(crate) current_region: Region,
    pub(crate) config: ShellConfig,
}

impl Environment {
    pub fn new(mut config: ShellConfig, cli: Opt) -> Result<Environment> {
        // First, update any config options based off `[cli]`.
        if let Some(format) = cli.format {
            config.ui.format = format;
        }

        // Next, identify the current ledger and region.
        let ledger_name = match (cli.ledger, &config.default_ledger) {
            (None, None) => Err(error::usage_error(
                "`--ledger` was not specified and there is no `default_ledger` in your config",
            ))?,
            (None, Some(default)) => default.clone(),
            (Some(cli), _) => cli,
        };

        let current_ledger = LedgerConfig {
            name: ledger_name,
            profile: cli.profile,
            region: cli.region,
            qldb_session_endpoint: cli.qldb_session_endpoint.map(|url| url.to_string()),
        };

        let current_region = awssdk_driver::determine_region(current_ledger.region.as_ref())?;

        let mut inner = EnvironmentInner {
            current_ledger,
            current_region,
            config,
        };

        let _ = inner.reload_current_ledger_config()?;

        Ok(Environment {
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    pub(crate) fn config(&self) -> ShellConfigGuard {
        let guard = self.inner.read().unwrap();
        ShellConfigGuard { guard }
    }

    pub(crate) fn current_ledger(&self) -> LedgerConfigGuard {
        let guard = self.inner.read().unwrap();
        LedgerConfigGuard { guard }
    }

    pub(crate) fn current_region(&self) -> Region {
        let guard = self.inner.read().unwrap();
        guard.current_region.clone()
    }

    pub(crate) fn update<F, R>(&self, update: F) -> Result<R>
    where
        F: FnOnce(&mut EnvironmentInner) -> Result<R>,
    {
        let mut inner = self.inner.write().unwrap();
        update(&mut inner)
    }

    /// When running in non-iteractive mode (e.g. using unix pipes to process
    /// data), when suppress chrome such as the welcome message.
    pub(crate) fn apply_noninteractive_defaults(&mut self) {
        let mut inner = self.inner.write().unwrap();
        let ui = &mut inner.deref_mut().config.ui;
        ui.display_welcome = false;
        ui.display_ctrl_signals = false;
    }
}

impl EnvironmentInner {
    /// Reconfigures `current_ledger` and `current_region` based on the
    /// currently active ledger name. The main reason to use this function is
    /// when switching ledgers.
    ///
    /// Returns true if a ledger with that name was found in config. false
    /// indicates no changes were made.
    pub(crate) fn reload_current_ledger_config(&mut self) -> Result<bool> {
        if let Some(ref all) = self.config.ledgers {
            if let Some(preconfigured) = all.iter().find(|c| c.name == self.current_ledger.name) {
                let current_ledger = &mut self.current_ledger;

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

                self.current_region =
                    awssdk_driver::determine_region(current_ledger.region.as_ref())?;

                return Ok(true);
            }
        }

        Ok(false)
    }
}

pub(crate) struct ShellConfigGuard<'a> {
    guard: RwLockReadGuard<'a, EnvironmentInner>,
}

impl<'a> Deref for ShellConfigGuard<'a> {
    type Target = ShellConfig;

    fn deref(&self) -> &Self::Target {
        &self.guard.config
    }
}

pub(crate) struct LedgerConfigGuard<'a> {
    guard: RwLockReadGuard<'a, EnvironmentInner>,
}

impl<'a> Deref for LedgerConfigGuard<'a> {
    type Target = LedgerConfig;

    fn deref(&self) -> &Self::Target {
        &self.guard.current_ledger
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.read().unwrap();
        write!(f, "{:?}", inner.config)
    }
}
