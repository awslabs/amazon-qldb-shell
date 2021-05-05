use std::{
    io::{self, Write},
    path::Path,
};

use anyhow::{anyhow, Result};
use tracing::subscriber;
use tracing_appender::{
    non_blocking::{NonBlocking, WorkerGuard},
    rolling,
};
use tracing_subscriber::{
    fmt::{self, MakeWriter},
    layer::SubscriberExt,
    EnvFilter,
};

use crate::{
    error,
    settings::{Environment, Opt},
};

pub(crate) fn configure(opt: &Opt, env: &Environment) -> Result<Option<WorkerGuard>> {
    let level = match opt.verbose {
        0 => "error",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let filter = EnvFilter::from_default_env()
        .add_directive("rustyline=off".parse()?)
        .add_directive(level.parse()?);

    let mut file_writer = FileWriter::new(env.log_file().value)?;
    let guard = file_writer.take_guard();

    let subscriber = fmt::Subscriber::builder().with_env_filter(filter);

    // FIXME: Remove duplication caused by generics.
    if opt.verbose == 3 {
        let subscriber = subscriber
            .pretty()
            .finish()
            .with(fmt::Layer::default().with_writer(std::io::stdout))
            .with(fmt::Layer::default().with_writer(file_writer));
        subscriber::set_global_default(subscriber)?;
    } else {
        let subscriber = subscriber
            .compact()
            .finish()
            .with(fmt::Layer::default().with_writer(std::io::stdout))
            .with(fmt::Layer::default().with_writer(file_writer));
        subscriber::set_global_default(subscriber)?;
    };

    Ok(guard)
}

enum FileWriter {
    Disabled,
    Enabled {
        non_blocking: NonBlocking,
        guard: Option<WorkerGuard>,
    },
}

impl FileWriter {
    fn new(p: Option<impl AsRef<Path>>) -> Result<FileWriter> {
        Ok(match p {
            None => FileWriter::Disabled,
            Some(p) => {
                let p = p.as_ref();
                let dirname = p.parent().ok_or(error::usage_error(
                    format!("{} is not in a directory", p.display()),
                    anyhow!("file logging was enabled"),
                ))?;
                let prefix = p.file_name().ok_or(error::usage_error(
                    format!("{} does not have a filename", p.display()),
                    anyhow!("file logging was enabled"),
                ))?;
                let file_appender = rolling::hourly(dirname, prefix);
                let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
                FileWriter::Enabled {
                    non_blocking,
                    guard: Some(guard),
                }
            }
        })
    }

    fn take_guard(&mut self) -> Option<WorkerGuard> {
        match self {
            FileWriter::Disabled => None,
            FileWriter::Enabled { guard, .. } => guard.take(),
        }
    }
}

impl Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            FileWriter::Disabled => Ok(buf.len()),
            FileWriter::Enabled {
                non_blocking: inner,
                ..
            } => inner.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            FileWriter::Disabled => Ok(()),
            FileWriter::Enabled {
                non_blocking: inner,
                ..
            } => inner.flush(),
        }
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            FileWriter::Disabled => Ok(()),
            FileWriter::Enabled {
                non_blocking: inner,
                ..
            } => inner.write_all(buf),
        }
    }
}

impl Clone for FileWriter {
    fn clone(&self) -> Self {
        match self {
            FileWriter::Disabled => FileWriter::Disabled,
            FileWriter::Enabled { non_blocking, .. } => FileWriter::Enabled {
                non_blocking: non_blocking.clone(),
                guard: None,
            },
        }
    }
}

impl MakeWriter for FileWriter {
    type Writer = FileWriter;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}
