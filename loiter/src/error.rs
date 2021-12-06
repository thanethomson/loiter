//! Errors handling for Loiter.

use std::path::PathBuf;

use thiserror::Error;

use crate::{Log, LogId, ProjectId, Task, TaskId, TaskState};

#[derive(Debug, Error)]
pub enum Error {
    #[error("file not found: {0}")]
    FileNotFound(PathBuf),
    #[error("project \"{0}\" not found")]
    ProjectNotFound(String),
    #[error("project \"{0}\" already exists")]
    ProjectAlreadyExists(String),
    #[error("task for project \"{0}\" with ID {1} does not exist")]
    TaskNotFound(String, TaskId),
    #[error("multiple tasks found for project \"{0}\" with ID {1} - please fix your local Loiter storage directory")]
    MultipleTasks(String, TaskId, Vec<Task>),
    #[error("task is missing its project ID: {0:?}")]
    TaskMissingProjectId(Task),
    #[error("task is missing its state: {0:?}")]
    TaskMissingState(Task),
    #[error("a log without a start time cannot be stopped")]
    LogWithoutStartCannotStop,
    #[error("cannot stop a log before it starts")]
    LogCannotStopBeforeStart,
    #[error("failed to calculate log duration from stop time: {0}")]
    LogDurationCalculationFailed(std::num::TryFromIntError),
    #[error("log for project \"{0}\"{} with ID {2} does not exist", .1.map(|task_id| format!(", task ID {}, ", task_id)).unwrap_or_else(|| "".to_string()))]
    LogNotFound(ProjectId, Option<TaskId>, LogId),
    #[error("log is missing its project ID: {0:?}")]
    LogMissingProjectId(Log),
    #[error("log is missing its ID: {0:?}")]
    LogMissingId(Log),
    #[error("there is currently no active log")]
    NoActiveLog,
    #[error("invalid path: {0}")]
    InvalidPath(PathBuf),
    #[error("invalid task file name: \"{0}\"")]
    InvalidTaskFilename(PathBuf),
    #[error("failed to parse task number from filename \"{0}\": {1}")]
    InvalidTaskNumber(PathBuf, std::num::ParseIntError),
    #[error("invalid task state: \"{0}\" (supported values: {})", .1.iter().map(|s| s.to_string()).collect::<Vec<String>>().join(", "))]
    InvalidTaskState(TaskState, Vec<TaskState>),
    #[error("task states must be unique; duplicate found in \"{}\"", .0.join(", "))]
    DuplicateTaskStates(Vec<TaskState>),
    #[error("too few task states ({0}) - there must be at least {1}")]
    TooFewTaskStates(usize, usize),
    #[error("invalid task ID(s) \"{0}\": {1}")]
    InvalidTaskIds(String, std::num::ParseIntError),
    #[error("failed to parse log number from filename \"{0}\": {1}")]
    InvalidLogNumber(PathBuf, std::num::ParseIntError),
    #[error("I/O failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization failed: {0}\n{1}")]
    Serialize(serde_json::Error, String),
    #[error("invalid date/time format: {0}")]
    InvalidDateTimeFormat(#[from] time::error::InvalidFormatDescription),
    #[error("invalid date/time: {0}")]
    InvalidDateTime(String),
    #[error("cannot determine local time zone: {0}")]
    CannotDetermineTimeZone(#[from] time::error::IndeterminateOffset),
    #[error("durations must start with a number: {0}")]
    DurationMustStartWithNumber(String),
    #[error("invalid amount for duration: \"{0}\" ({1})")]
    InvalidDurationAmount(String, std::num::ParseIntError),
    #[error("invalid units for duration: \"{0}\"")]
    InvalidDurationUnit(String),
    #[error("invalid duration: \"{0}{1}\"")]
    InvalidDuration(String, String),
    #[error("tag contains invalid characters (can only be alphanumeric, '-' or '_'): \"{0}\"")]
    TagHasInvalidChars(String),
    #[error("unrecognized project field: {0}")]
    UnrecognizedProjectField(String),
    #[error("unrecognized task field: {0}")]
    UnrecognizedTaskField(String),
    #[error("unrecognized work log field: {0}")]
    UnrecognizedLogField(String),
    #[error("unrecognized sort order: {0}")]
    UnrecognizedSortOrder(String),
    #[error("cannot accept both duration and stop time - please supply only one of these")]
    CannotAcceptDurationAndStop,
    #[error("sort specification cannot have empty components: {0}")]
    SortSpecHasEmptyComponent(String),
    #[error("sort specification \"{0}\" has too many parts in \"{1}\" (only a single colon is allowed for each field)")]
    TooManyComponentsInSortSpec(String, String),
    #[error("invalid timestamp filter: \"{0}\"")]
    InvalidTimestampFilter(String),
    #[error("failed to parse timestamp filter: {0}")]
    TimestampFilterParsingFailed(String),
    #[error("invalid duration filter: \"{0}\"")]
    InvalidDurationFilter(String),
    #[error("invalid duration filter operator \"{0}\" in filter: \"{1}\"")]
    InvalidDurationFilterOp(String, String),
    #[error("both project and log ID are required in order to reference a specific log")]
    BothProjectAndLogIdRequired,
    #[error("path already initialized as a Git repository: {}", .0.display())]
    RemoteAlreadyInitialized(PathBuf),
    #[error("remote initialization failed - see debug logs for details")]
    RemoteInitFailed,
    #[error("remote is not initialized for path: {}", .0.display())]
    NotRemote(PathBuf),
    #[error("remote push failed: {0}")]
    RemotePushFailed(String),
}
