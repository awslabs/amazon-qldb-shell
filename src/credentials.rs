use std::sync::Arc;

use aws_auth::{Credentials, CredentialsError, ProvideCredentials};
use futures::executor;
use rusoto_core::credential::ProvideAwsCredentials as RusotoProvider;

pub(crate) struct RusotoCredentialProvider<P>(pub(crate) P)
where
    P: RusotoProvider;

impl<P> ProvideCredentials for RusotoCredentialProvider<Arc<P>>
where
    P: RusotoProvider + Send + Sync,
{
    fn provide_credentials(&self) -> Result<Credentials, CredentialsError> {
        match tokio::task::block_in_place(|| {
            executor::block_on(async { self.0.credentials().await })
        }) {
            Ok(credentials) => Ok(Credentials::from_keys(
                credentials.aws_access_key_id(),
                credentials.aws_secret_access_key(),
                credentials.token().to_owned(),
            )),
            Err(err) => Err(CredentialsError::Unhandled(Box::new(err))),
        }
    }
}
