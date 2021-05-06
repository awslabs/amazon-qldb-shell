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
    prelude::*,
    EnvFilter, Registry,
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

    let mut writer = ShellWriter::new(env.log_file().value)?;
    let guard = writer.take_guard();

    let subscriber = Registry::default()
        .with(filter)
        .with(fmt::Layer::default().with_writer(writer));
    subscriber::set_global_default(subscriber)?;

    Ok(guard)
}

enum ShellWriter {
    Stdout(io::Stdout),
    FileWriter {
        non_blocking: NonBlocking,
        guard: Option<WorkerGuard>,
    },
}

impl ShellWriter {
    fn new(p: Option<impl AsRef<Path>>) -> Result<ShellWriter> {
        Ok(match p {
            None => ShellWriter::Stdout(io::stdout()),
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
                ShellWriter::FileWriter {
                    non_blocking,
                    guard: Some(guard),
                }
            }
        })
    }

    fn take_guard(&mut self) -> Option<WorkerGuard> {
        match self {
            ShellWriter::Stdout(_) => None,
            ShellWriter::FileWriter { guard, .. } => guard.take(),
        }
    }
}

impl Write for ShellWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            ShellWriter::Stdout(stdout) => stdout.write(buf),
            ShellWriter::FileWriter {
                non_blocking: inner,
                ..
            } => inner.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            ShellWriter::Stdout(stdout) => stdout.flush(),
            ShellWriter::FileWriter {
                non_blocking: inner,
                ..
            } => inner.flush(),
        }
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            ShellWriter::Stdout(stdout) => stdout.write_all(buf),
            ShellWriter::FileWriter {
                non_blocking: inner,
                ..
            } => inner.write_all(buf),
        }
    }
}

impl Clone for ShellWriter {
    fn clone(&self) -> Self {
        match self {
            ShellWriter::Stdout(_) => ShellWriter::Stdout(io::stdout()),
            ShellWriter::FileWriter { non_blocking, .. } => ShellWriter::FileWriter {
                non_blocking: non_blocking.clone(),
                guard: None,
            },
        }
    }
}

impl MakeWriter for ShellWriter {
    type Writer = ShellWriter;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}
