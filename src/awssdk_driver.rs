use std::error::Error;
use std::{str::FromStr, sync::Arc};

use amazon_qldb_driver::{retry, QldbDriver, QldbDriverBuilder, QldbResult, QldbSession};
use anyhow::Result;
use async_trait::async_trait;
use aws_config::{
    meta::{credentials::LazyCachingCredentialsProvider, region::RegionProviderChain},
    profile::ProfileFileCredentialsProvider,
};
use aws_sdk_qldbsession::{
    config,
    error::SendCommandError,
    input::SendCommandInput,
    model::{EndSessionRequest, StartSessionRequest},
    output::SendCommandOutput,
    Config, Endpoint, Region, SdkError,
};
use aws_smithy_client::bounds::SmithyConnector;
use aws_smithy_client::erase::DynConnector;
use aws_smithy_client::Client;
use aws_smithy_http::{middleware::MapRequest, operation};
use aws_smithy_http_tower::map_request::MapRequestLayer;
use http::header::HeaderName;
use http::{HeaderValue, Uri};
use tower::ServiceBuilder;

use crate::{error, settings::Environment};

#[derive(Clone, Debug)]
struct UserAgent;

impl MapRequest for UserAgent {
    type Error = Box<dyn Error + Send + Sync + 'static>;

    fn apply(&self, request: operation::Request) -> Result<operation::Request, Self::Error> {
        request.augment(|mut req, _conf| {
            req.headers_mut().append(
                HeaderName::from_static("x-amz-qldb-driver-version"),
                HeaderValue::from_static(amazon_qldb_driver::version())
            );
            req.headers_mut().append(
                HeaderName::from_static("x-amz-qldb-shell-version"),
                HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
            );
            Ok(req)
        })
    }
}

#[derive(Clone)]
pub(crate) struct QldbSessionSdk<C = DynConnector> {
    inner: Arc<QldbSessionSdkInner<C>>,
}

struct QldbSessionSdkInner<C = DynConnector> {
    client: Client<C>,
    conf: Config,
}

impl<C> QldbSessionSdk<C> {
    fn new(client: Client<C>, conf: Config) -> QldbSessionSdk<C> {
        let inner = QldbSessionSdkInner { client, conf };
        QldbSessionSdk {
            inner: Arc::new(inner),
        }
    }
}

#[async_trait]
impl<C> QldbSession for QldbSessionSdk<C>
where
    C: SmithyConnector,
{
    async fn send_command(
        &self,
        input: SendCommandInput,
    ) -> Result<SendCommandOutput, SdkError<SendCommandError>> {
        let op = input
            .make_operation(&self.inner.conf)
            .await
            .expect("valid operation"); // FIXME: remove potential panic
        self.inner.client.call(op).await
    }
}

pub(crate) async fn build_driver(
    client: QldbSessionSdk,
    ledger: String,
) -> QldbResult<QldbDriver<QldbSessionSdk>> {
    // We disable transaction retries because they don't make sense. Users
    // are entering statements, so if the tx fails they actually have to
    // enter them again! We can't simply remember their inputs and try
    // again, as individual statements may be derived from values seen from
    // yet other statements.
    QldbDriverBuilder::new()
        .ledger_name(ledger)
        .transaction_retry_policy(retry::never())
        .build_with_client(client)
        .await
}

/// Tries to start a session on the given ledger (via `env`). Fails with a
/// `usage_error` otherwise.
///
/// If a connection is formed, the new session is discarded and the client is
/// returned. The cleanup is just good manners, but the client is important
/// because it means future commands can reuse that same initial connection,
/// credentials, etc.
pub(crate) async fn health_check_start_session(
    env: &Environment,
) -> Result<QldbSessionSdk<DynConnector>> {
    let session_client = build_client(&env).await?;

    let current_ledger = env.current_ledger();
    let resp = session_client
        .send_command(
            SendCommandInput::builder()
                .start_session(
                    StartSessionRequest::builder()
                        .ledger_name(current_ledger.name.clone())
                        .build(),
                )
                .build()?,
        )
        .await
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

    let session_token = match resp.start_session.and_then(|r| r.session_token) {
        Some(session_token) => session_token,
        None => Err(error::bug("start session did not return a session token"))?,
    };

    // Try be a good citizen, but don't fail if the new session can't be
    // released.
    let _ = session_client
        .send_command(
            SendCommandInput::builder()
                .session_token(session_token)
                .end_session(EndSessionRequest::builder().build())
                .build()?,
        )
        .await;

    Ok(session_client)
}

async fn build_client(env: &Environment) -> Result<QldbSessionSdk<DynConnector>> {
    let builder: aws_smithy_client::Builder<
        _,
        aws_sdk_qldbsession::middleware::DefaultMiddleware,
        _,
    > = aws_smithy_client::Builder::new();
    let client = builder
        .rustls()
        .map_middleware(|middleware| {
            ServiceBuilder::new()
                .layer(MapRequestLayer::for_mapper(UserAgent))
                .layer(middleware)
        })
        .build_dyn();

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

    Ok(QldbSessionSdk::new(client, conf.build()))
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
