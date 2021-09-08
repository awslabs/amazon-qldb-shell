use aws_auth::provider::AsyncProvideCredentials;
use aws_auth::provider::BoxFuture;
use aws_auth::provider::CredentialsError;
use aws_auth::provider::CredentialsResult;
use aws_auth::Credentials;
use rusoto_core::credential::ProvideAwsCredentials as RusotoProvider;

pub(crate) struct RusotoCredentialProvider<P: RusotoProvider + Send + Sync + 'static> {
    rusoto: P,
}

pub(crate) fn from_rusoto<P>(rusoto: P) -> RusotoCredentialProvider<P>
where
    P: RusotoProvider + Send + Sync + 'static,
{
    RusotoCredentialProvider { rusoto }
}

impl<P> AsyncProvideCredentials for RusotoCredentialProvider<P>
where
    P: RusotoProvider + Send + Sync + 'static,
{
    fn provide_credentials<'a>(&'a self) -> BoxFuture<'a, CredentialsResult>
    where
        Self: 'a,
    {
        let fut = async move {
            match self.rusoto.credentials().await {
                Ok(credentials) => Ok(Credentials::from_keys(
                    credentials.aws_access_key_id(),
                    credentials.aws_secret_access_key(),
                    credentials.token().to_owned(),
                )),
                Err(err) => Err(CredentialsError::Unhandled(Box::new(err))),
            }
        };

        Box::pin(fut)
    }
}
