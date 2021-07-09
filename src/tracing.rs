use anyhow::Result;
use tracing::subscriber;
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

use crate::{error, settings::Environment};

/// Configures tracing.
///
/// By default, tracing writes to stdout using the `fmt` subscriber which
/// produces log-like output. The CLI supports a repeting `--verbose` flag to
/// change the filter level from error .. trace (achieved with `-vvv`).
///
/// If logging is enabled (via `debug.log` in config files), all events (no
/// filter) will be written to an hourly file in [bunyan][bunyan] format. You
/// can then use the CLI to work with the data.
///
/// When logging is enabled, no events are written to stdout. This is because to
/// get useful logs we need a debug/trace level filter. At that level, stdout is
/// far too noisy. It would be great if we could decouple filtering in the
/// stdout vs file writer, but this is not yet supported in tracing
/// (https://github.com/tokio-rs/tracing/issues/302).
///
/// [bunyan]: https://www.npmjs.com/package/bunyan#cli-usage
pub(crate) fn configure(verbose: u8, env: &Environment) -> Result<Option<WorkerGuard>> {
    let (non_blocking, guard) = match env.config().debug.log {
        Some(ref path) => {
            let dirname = path.parent().ok_or(error::usage_error(format!(
                "{} is not in a directory",
                path.display()
            )))?;
            let prefix = path.file_name().ok_or(error::usage_error(format!(
                "{} does not have a filename",
                path.display()
            )))?;
            let file_appender = rolling::hourly(dirname, prefix);
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            (Some(non_blocking), Some(guard))
        }
        None => (None, None),
    };

    match non_blocking {
        Some(non_blocking) => {
            let formatting_layer = BunyanFormattingLayer::new("qldb".into(), non_blocking);
            let subscriber = Registry::default()
                .with(JsonStorageLayer)
                .with(formatting_layer);

            subscriber::set_global_default(subscriber)?;
        }
        None => {
            let level = match verbose {
                0 => "error",
                1 => "info",
                2 => "debug",
                _ => "trace",
            };

            let filter = EnvFilter::from_default_env()
                .add_directive("rustyline=off".parse()?)
                .add_directive(level.parse()?);

            let subscriber = fmt::Subscriber::builder().with_env_filter(filter).finish();

            subscriber::set_global_default(subscriber)?;
        }
    };

    Ok(guard)
}
