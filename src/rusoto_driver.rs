use crate::settings::Environment;
use amazon_qldb_driver::QldbDriverBuilder;
use amazon_qldb_driver::{retry, QldbDriver};
use anyhow::Result;
use async_trait::async_trait;
use rusoto_core::{
    credential::{ChainProvider, ProfileProvider, ProvideAwsCredentials},
    Client, HttpClient, Region,
};
use rusoto_qldb_session::QldbSessionClient;
use std::str::FromStr;
use tracing::warn;

pub async fn build_driver(env: &Environment) -> Result<QldbDriver<QldbSessionClient>> {
    let client = build_rusoto_client(env).await?;

    // We disable transaction retries because they don't make sense. Users
    // are entering statements, so if the tx fails they actually have to
    // enter them again! We can't simply remember their inputs and try
    // again, as individual statements may be derived from values seen from
    // yet other statements.
    QldbDriverBuilder::new()
        .ledger_name(env.ledger().value)
        .transaction_retry_policy(retry::never())
        .build_with_client(client)
        .await
}

pub(crate) async fn health_check_start_session(env: &Environment) -> Result<()> {
    use rusoto_qldb_session::*;
    let session_client = build_rusoto_client(&env).await?;

    session_client
        .send_command(SendCommandRequest {
            start_session: Some(StartSessionRequest {
                ledger_name: env.ledger().value,
            }),
            ..Default::default()
        })
        .await?;

    Ok(())
}

async fn build_rusoto_client(env: &Environment) -> Result<QldbSessionClient> {
    let provider = profile_provider(&env)?;
    let region = rusoto_region(&env)?;
    let creds = match provider {
        Some(p) => CredentialProvider::Profile(p),
        None => CredentialProvider::Chain(ChainProvider::new()),
    };

    let mut hyper = HttpClient::new()?;
    hyper.local_agent(format!(
        "QLDB Driver for Rust v{}/QLDB Shell for Rust v{}",
        amazon_qldb_driver::version(),
        env!("CARGO_PKG_VERSION")
    ));

    let client = Client::new_with(creds, hyper);
    Ok(QldbSessionClient::new_with_client(client, region))
}

/// Required for static dispatch of [`QldbSessionClient::new_with`].
enum CredentialProvider {
    Profile(ProfileProvider),
    Chain(ChainProvider),
}

#[async_trait]
impl ProvideAwsCredentials for CredentialProvider {
    async fn credentials(
        &self,
    ) -> Result<rusoto_core::credential::AwsCredentials, rusoto_core::credential::CredentialsError>
    {
        use CredentialProvider::*;
        match self {
            Profile(p) => p.credentials().await,
            Chain(c) => c.credentials().await,
        }
    }
}

fn profile_provider(env: &Environment) -> Result<Option<ProfileProvider>> {
    let it = match env.profile().value {
        Some(p) => {
            let mut prof = ProfileProvider::new()?;
            prof.set_profile(p);
            Some(prof)
        }
        None => None,
    };

    Ok(it)
}

// FIXME: Default region should consider what is set in the Profile.
fn rusoto_region(env: &Environment) -> Result<Region> {
    let it = match (env.region().value, env.qldb_session_endpoint().value) {
        (Some(r), Some(e)) => Region::Custom {
            name: r,
            endpoint: e,
        },
        (Some(r), None) => match Region::from_str(&r) {
            Ok(it) => it,
            Err(e) => {
                warn!("Unknown region {}: {}. If you know the endpoint, you can specify it and try again.", r, e);
                return Err(e)?;
            }
        },
        (None, Some(e)) => Region::Custom {
            name: Region::default().name().to_owned(),
            endpoint: e,
        },
        (None, None) => Region::default(),
    };

    Ok(it)
}
