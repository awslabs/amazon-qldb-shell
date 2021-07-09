use amazon_qldb_shell::{error::ShellError, run};
use anyhow::Result;
use std::{env, process::exit};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = _main().await {
        if let Some(shell) = e.downcast_ref::<ShellError>() {
            match shell {
                ShellError::UsageError { message } => handle_usage_error(message),
                ShellError::Bug(message) => handle_bug(message),
            }
        } else {
            eprintln!("qldb shell error: {:#}", e);
        }

        exit(1);
    }
}

async fn _main() -> Result<()> {
    // If a crash happens we want the backtrace to be printed by default. This
    // makes bug reporting easier!
    if let Err(env::VarError::NotPresent) = env::var("RUST_BACKTRACE") {
        env::set_var("RUST_BACKTRACE", "1");
    }

    run().await
}

fn handle_usage_error(message: impl AsRef<str>) {
    eprintln!("usage error: {}", message.as_ref());
}

fn handle_bug(message: impl AsRef<str>) {
    eprintln!("qldb shell bug: {}", message.as_ref());
    eprintln!(
        r#"
The QLDB shell has encountered an unhandled error and will now exit.
Please consider reporting this at: https://github.com/awslabs/amazon-qldb-shell/issues/new?template=bug_report.md"#
    );
}
