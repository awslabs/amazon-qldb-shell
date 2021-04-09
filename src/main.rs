use amazon_qldb_shell::run;
use anyhow::Result;
use std::error::Error;
use std::{env, process::exit};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = _main().await {
        eprintln!("qldb shell error: {}", e);

        fn recurse_source(source: Option<&dyn Error>) {
            if let Some(err) = source {
                eprintln!(" - caused by: {}", err);
                recurse_source(err.source());
            }
        }

        recurse_source(e.source());

        eprintln!(
            r#"
The QLDB shell has encountered an unhandled error and will now exit.
Please consider reporting this at: https://github.com/awslabs/amazon-qldb-shell/issues/new?template=bug_report.md"#
        );
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
