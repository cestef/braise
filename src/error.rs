use thiserror::Error;

#[derive(Debug, Error)]
pub enum BraiseError {
    #[error("No Braise file found")]
    NoBraiseFileFound,
    #[error("Invalid file format: {0}")]
    InvalidFileFormat(String),
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    #[error("Invalid task: {0}")]
    InvalidTask(String),
    #[error("No task or default task found")]
    NoTask,
    #[error("Invalid argument index: {0}, only {1} arguments provided")]
    InvalidArgIndex(usize, usize),
    #[error("Error: {0}")]
    Error(String),
    #[error("Invalid shell: {0}")]
    InvalidShell(String),
    #[error("Invalid dependency: {0}")]
    InvalidDependency(String),
    #[error("No shell found")]
    NoShell,
    #[error("No valid task found for {0}")]
    NoValidTask(String),
}
