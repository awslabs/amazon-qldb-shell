use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShellError {
    #[error("Usage error: {}", .message)]
    UsageError { message: String },
    #[error("Bug: {0}")]
    Bug(String),
}

pub fn usage_error(message: impl AsRef<str>) -> ShellError {
    ShellError::UsageError {
        message: message.as_ref().into(),
    }
}

pub fn bug(message: impl AsRef<str>) -> ShellError {
    ShellError::Bug(message.as_ref().into())
}
