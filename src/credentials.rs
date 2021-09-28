use aws_types::credentials::{future, Credentials, CredentialsError, ProvideCredentials};
use rusoto_core::credential::ProvideAwsCredentials as RusotoProvider;

pub(crate) struct RusotoCredentialProvider<P: RusotoProvider + Send + Sync + 'static> {
    rusoto: P,
}

impl<P> std::fmt::Debug for RusotoCredentialProvider<P>
where
    P: RusotoProvider + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RusotoCredentialProvider").finish()
    }
}

pub(crate) fn from_rusoto<P>(rusoto: P) -> RusotoCredentialProvider<P>
where
    P: RusotoProvider + Send + Sync + 'static,
{
    RusotoCredentialProvider { rusoto }
}

impl<P> ProvideCredentials for RusotoCredentialProvider<P>
where
    P: RusotoProvider + Send + Sync + 'static,
{
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials
    where
        Self: 'a,
    {
        future::ProvideCredentials::new(async move {
            match self.rusoto.credentials().await {
                Ok(credentials) => Ok(Credentials::from_keys(
                    credentials.aws_access_key_id(),
                    credentials.aws_secret_access_key(),
                    credentials.token().to_owned(),
                )),
                Err(err) => Err(CredentialsError::Unhandled(Box::new(err))),
            }
        })
    }
}
