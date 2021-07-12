use crate::error;
use crate::settings::Environment;
use anyhow::Result;
use async_trait::async_trait;
use rusoto_core::credential::{DefaultCredentialsProvider, ProfileProvider, ProvideAwsCredentials};

/// Required for static dispatch of [`QldbSessionClient::new_with`].
pub(crate) enum CredentialProvider {
    Profile(ProfileProvider),
    Default(DefaultCredentialsProvider),
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
            Default(c) => c.credentials().await,
        }
    }
}

pub(crate) fn profile_provider(env: &Environment) -> Result<Option<ProfileProvider>> {
    let it = match env.current_ledger().profile {
        Some(ref p) => {
            let mut prof = ProfileProvider::new().map_err(|e| {
                error::usage_error(format!("Unable to create profile provider: {}", e))
            })?;
            prof.set_profile(p);
            Some(prof)
        }
        None => None,
    };

    Ok(it)
}
