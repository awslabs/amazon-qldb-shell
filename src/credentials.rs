use std::io;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::Receiver as SyncReceiver;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use anyhow::Result;
use aws_auth::{Credentials, CredentialsError, ProvideCredentials};
use rusoto_core::credential::ProvideAwsCredentials as RusotoProvider;
use tokio::runtime::Builder;
use tokio::sync::mpsc::{channel, Sender};

pub(crate) struct RusotoCredentialProvider {
    bridge: AsyncBridge,
}

struct AsyncBridge {
    tx: Sender<()>,
    rx: Arc<Mutex<SyncReceiver<Result<Credentials, CredentialsError>>>>,
    _handle: JoinHandle<io::Result<()>>, // stored for cancellation purposes
}

pub(crate) async fn from_rusoto<P>(rusoto: P) -> Result<RusotoCredentialProvider>
where
    P: RusotoProvider + Send + Sync + 'static,
{
    let (wake, mut req) = channel(1);
    let (res, credentials) = sync_channel(1);

    let handle = thread::spawn(|| {
        let rt = Builder::new_current_thread().build()?;
        rt.block_on(async move {
            loop {
                if let None = req.recv().await {
                    break;
                }
                let credentials = match rusoto.credentials().await {
                    Ok(credentials) => Ok(Credentials::from_keys(
                        credentials.aws_access_key_id(),
                        credentials.aws_secret_access_key(),
                        credentials.token().to_owned(),
                    )),
                    Err(err) => Err(CredentialsError::Unhandled(Box::new(err))),
                };
                if let Err(_) = res.send(credentials) {
                    break;
                }
            }
        });

        Ok(())
    });

    let bridge = AsyncBridge {
        tx: wake,
        rx: Arc::new(Mutex::new(credentials)),
        _handle: handle,
    };

    Ok(RusotoCredentialProvider { bridge })
}

impl ProvideCredentials for RusotoCredentialProvider {
    fn provide_credentials(&self) -> Result<Credentials, CredentialsError> {
        self.bridge
            .tx
            .try_send(())
            .expect("the credentials task should never crash");
        // This doesn't work because (I think) the spawned future never wakes
        // up.
        let res = self.bridge.rx.lock().expect("mutex is never poisoned");
        res.recv()
            .expect("credentials (or an error) should always come back")
    }
}
