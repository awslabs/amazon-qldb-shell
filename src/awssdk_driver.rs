use anyhow::Result;
use aws_config::{
    meta::{credentials::LazyCachingCredentialsProvider, region::RegionProviderChain},
    profile::ProfileFileCredentialsProvider,
};
use futures::channel::mpsc::channel;
use http::Uri;
use std::{str::FromStr, time::Duration};
use tracing::debug;

use amazon_qldb_driver::awssdk::{config, Client, Endpoint, Region};
use amazon_qldb_driver::{retry, QldbDriver, QldbDriverBuilder};

use crate::{error, settings::Environment};

// TODO: re-add metadata
// use aws_http::user_agent::{AdditionalMetadata, AwsUserAgent};
// let mut op = input
//     .make_operation(&self.inner.conf)
//     .await
//     .expect("valid operation"); // FIXME: remove potential panic
// op.properties_mut()
//     .get_mut::<AwsUserAgent>()
//     .unwrap()
//     .add_metadata(AdditionalMetadata::new(
//         format!("QLDB Driver for Rust v{}", amazon_qldb_driver::version()),
//         format!("QLDB Shell for Rust v{}", env!("CARGO_PKG_VERSION")),
//     ));
// self.inner.client.call(op).await
pub(crate) async fn build_driver(client: Client, ledger: String) -> Result<QldbDriver> {
    // We disable transaction retries because they don't make sense. Users
    // are entering statements, so if the tx fails they actually have to
    // enter them again! We can't simply remember their inputs and try
    // again, as individual statements may be derived from values seen from
    // yet other statements.
    Ok(QldbDriverBuilder::new()
        .ledger_name(ledger)
        .transaction_retry_policy(retry::never())
        .build_with_client(client)
        .await?)
}

/// Tries to start a session on the given ledger (via `env`). Fails with a
/// `usage_error` otherwise.
///
/// If a connection is formed, the new session is discarded and the client is
/// returned. The cleanup is just good manners, but the client is important
/// because it means future commands can reuse that same initial connection,
/// credentials, etc.
pub(crate) async fn health_check_start_session(env: &Environment) -> Result<Client> {
    let session_client = build_client(&env).await?;

    let current_ledger = env.current_ledger();

    let (mut sender, receiver) = channel(1);

    debug!("testing connectivity");
    let connect_fut = session_client
        .send_command()
        .ledger_name(&current_ledger.name[..])
        .command_stream(receiver.into())
        .send();

    let mut output = tokio::time::timeout(Duration::from_secs(5), connect_fut)
        .await
        .map_err(|_| error::usage_error(format!("timed out connecting after 5 seconds")))?
        .map_err(|e| {
            error::usage_error(format!(
                r#"Unable to connect to ledger `{}`.

Please check the following:

- That you have specified a ledger that exists and is active
- That the AWS region you are targeting is correct
- That your AWS credentials are setup
- That your AWS credentials grant access on this ledger

The following error may have more information: {}
"#,
                current_ledger.name.clone(),
                e
            ))
        })?;
    debug!("connected!");
    sender.close_channel();
    output.result_stream.recv().await?;

    // Dropping the ch pair ends the connection.

    Ok(session_client)
}

async fn build_client(env: &Environment) -> Result<Client> {
    let aws_config = aws_config::from_env();
    let aws_config = match env.current_ledger().profile {
        Some(ref name) => aws_config.credentials_provider(
            LazyCachingCredentialsProvider::builder()
                .load(
                    ProfileFileCredentialsProvider::builder()
                        .profile_name(name)
                        .build(),
                )
                .build(),
        ),
        None => aws_config,
    };
    let aws_config = aws_config.load().await;

    let conf = config::Builder::from(&aws_config).region(env.current_region());
    let conf = match env.current_ledger().qldb_session_endpoint {
        Some(ref endpoint) => {
            // Strip a trailing slash, otherwise things go wrong in hyper. Specifically,
            // it makes a POST request that looks like this:
            //
            //     POST // HTTP/1.1
            let clean = endpoint.trim_matches(|c| c == '/');
            let endpoint = Uri::from_str(clean)?;
            let resolver = Endpoint::immutable(endpoint);
            conf.endpoint_resolver(resolver)
        }
        _ => conf,
    };

    Ok(Client::from_conf(conf.build()))
}

// Note: infallible, but potentially fallible in the future (e.g. if we want to
// check that the region is valid).
pub(crate) async fn determine_region<S>(user_specified: Option<S>) -> Result<Region>
where
    S: Into<String>,
{
    let user_specified = user_specified.map(|it| Region::new(it.into()));
    let region = RegionProviderChain::first_try(user_specified)
        .or_default_provider()
        .region()
        .await;

    Ok(region.ok_or(error::usage_error(
        "no region provided, and none could be automatically determined",
    ))?)
}
