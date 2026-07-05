use std::fmt::{Display, Formatter};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Http(String),
    Zip(zip::result::ZipError),
    Parse(String),
    Process(String),
    Platform(String),
    Validation(String),
    State(String),
    Other(String),
}

impl AppError {
    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse(message.into())
    }

    pub fn process(message: impl Into<String>) -> Self {
        Self::Process(message.into())
    }

    pub fn platform(message: impl Into<String>) -> Self {
        Self::Platform(message.into())
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn state(message: impl Into<String>) -> Self {
        Self::State(message.into())
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Http(e) => write!(f, "HTTP error: {e}"),
            Self::Zip(e) => write!(f, "Zip error: {e}"),
            Self::Parse(e) => write!(f, "Parse error: {e}"),
            Self::Process(e) => write!(f, "Process error: {e}"),
            Self::Platform(e) => write!(f, "Platform error: {e}"),
            Self::Validation(e) => write!(f, "Validation error: {e}"),
            Self::State(e) => write!(f, "State error: {e}"),
            Self::Other(e) => write!(f, "Error: {e}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        Self::Http(value.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(value: zip::result::ZipError) -> Self {
        Self::Zip(value)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::Parse(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn display_smoke() {
        let err = AppError::validation("invalid");
        assert!(err.to_string().contains("Validation error"));
    }
}
