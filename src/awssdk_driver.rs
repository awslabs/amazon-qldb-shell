use anyhow::Result;
use async_trait::async_trait;
use std::{error::Error, sync::Arc};

use amazon_qldb_driver::QldbSession;
use aws_hyper::{conn::Standard, Client};
use aws_sdk_qldbsession::{
    error::SendCommandError,
    input::SendCommandInput,
    model::{EndSessionRequest, StartSessionRequest, StartSessionResult},
    output::SendCommandOutput,
    Config, Region, SdkError,
};
use rusoto_core::credential::DefaultCredentialsProvider;
use smithy_http::body::SdkBody;
use tower::Service;

use crate::{
    credentials::RusotoCredentialProvider,
    error,
    rusoto_driver::{self, CredentialProvider},
    settings::Environment,
};

type BoxError = Box<dyn Error + Send + Sync>;

pub(crate) struct QldbSessionSdk<S> {
    client: Client<S>,
    conf: Config,
}

impl<S> QldbSessionSdk<S> {
    fn new(client: Client<S>, conf: Config) -> QldbSessionSdk<S> {
        QldbSessionSdk { client, conf }
    }
}

// The trait bounds are imposed by the implementation in aws-hyper.
#[async_trait]
impl<S> QldbSession for QldbSessionSdk<S>
where
    S: Send + Sync,
    S: Service<http::Request<SdkBody>, Response = http::Response<SdkBody>> + Send + Clone + 'static,
    S::Error: Into<BoxError> + Send + Sync + 'static,
    S::Future: Send + 'static,
{
    async fn send_command(
        &self,
        input: SendCommandInput,
    ) -> Result<SendCommandOutput, SdkError<SendCommandError>> {
        let op = input.make_operation(&self.conf).expect("valid operation"); // FIXME: remove potential panic
        self.client.call(op).await
    }
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
) -> Result<QldbSessionSdk<Standard>> {
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
            error::usage_error(
                format!(
                    r#"Unable to connect to ledger `{}`.

Please check the following:

- That you have specified a ledger that exists and is active
- That the AWS region you are targeting is correct
- That your AWS credentials are setup
- That your AWS credentials grant access on this ledger

The following error chain may have more information:
"#,
                    current_ledger.name.clone()
                ),
                e,
            )
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

async fn build_client(env: &Environment) -> Result<QldbSessionSdk<Standard>> {
    let hyper = aws_hyper::conn::Standard::https();
    let client = Client::new(hyper);
    let provider = rusoto_driver::profile_provider(&env)?;
    let rusoto_provider = match provider {
        Some(p) => CredentialProvider::Profile(p),
        None => CredentialProvider::Default(DefaultCredentialsProvider::new()?),
    };
    let creds = RusotoCredentialProvider(Arc::new(rusoto_provider));
    let region = env.current_region().name().to_owned();
    let conf = Config::builder()
        .region(Region::new(region))
        .credentials_provider(creds)
        .build();

    // TODO: Set user-agent: https://github.com/awslabs/aws-sdk-rust/issues/146
    // let mut hyper = HttpClient::new()?;
    // hyper.local_agent(format!(
    //     "QLDB Driver for Rust v{}/QLDB Shell for Rust v{}",
    //     amazon_qldb_driver::version(),
    //     env!("CARGO_PKG_VERSION")
    // ));

    Ok(QldbSessionSdk::new(client, conf))
}
