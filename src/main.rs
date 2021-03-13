use amazon_qldb_shell::run;
use anyhow::Result;
use tokio::runtime;

fn main() -> Result<()> {
    let runtime = runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    run(runtime)
}
