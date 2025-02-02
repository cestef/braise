use thiserror::Error;

#[derive(Error, Debug)]
pub enum BraiseError {
    #[error("No Braise file found")]
    NoBraiseFileFound,
    #[error("Invalid file format: {0}")]
    InvalidFileFormat(String),
    #[error("Invalid task format: {0}")]
    InvalidTaskFormat(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("TOML error: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("No task or default task found")]
    NoTask,
    #[error("No task found for: {0}")]
    NoTaskFoundFor(String),
    #[error("Invalid argument index: {0}, only {1} arguments provided")]
    InvalidArgIndex(usize, usize),
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
    #[error("An unknown error occurred: {0}")]
    Error(String),
    #[error("Logger error: {0}")]
    LoggerError(#[from] log::SetLoggerError),
    #[error("dotenvy error: {0}")]
    DotenvyError(#[from] dotenvy::Error),
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    #[error("Failed to parse glob pattern: {0}")]
    GlobPatternError(#[from] fnmatch_regex::error::Error),
    #[error("Failed to parse glob pattern: {0}")]
    GlobError(String),
    #[error("Thread error: {0}")]
    ThreadError(#[from] tokio::task::JoinError),
    #[error("Task error: {0}")]
    TaskError(String),
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
    #[error("A task cannot have both a group and a command: {0}")]
    BothGroupAndCommand(String),
    #[error("Missing field '{0}' for: {1}")]
    MissingField(String, String),
    #[error("Dependency cannot be a group: {0}")]
    GroupDependecy(String),
}

pub type Result<T, E = BraiseError> = std::result::Result<T, E>;

macro_rules! bail {
    ($err:expr) => {
        return Err(From::from($err))
    };
}
pub(crate) use bail;
