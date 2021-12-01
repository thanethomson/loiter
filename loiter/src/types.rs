//! Data types used by Loiter.

use std::{collections::HashSet, str::FromStr};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{strings::slugify, Duration, Error, Timestamp};

pub type ProjectId = String;
pub type TaskId = u32;
pub type LogId = u32;
pub type TaskState = String;

const DEFAULT_INITIAL_TASK_STATE: &str = "inbox";
const DEFAULT_IN_PROGRESS_TASK_STATE: &str = "doing";
const DEFAULT_DONE_TASK_STATE: &str = "done";
const DEFAULT_TASK_STATES: &[&str] = &[
    DEFAULT_INITIAL_TASK_STATE,
    "todo",
    DEFAULT_IN_PROGRESS_TASK_STATE,
    DEFAULT_DONE_TASK_STATE,
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskStateConfig {
    states: HashSet<TaskState>,
    initial: TaskState,
    in_progress: TaskState,
    done: TaskState,
}

impl Default for TaskStateConfig {
    fn default() -> Self {
        Self {
            states: HashSet::from_iter(DEFAULT_TASK_STATES.iter().map(|s| s.to_string())),
            initial: DEFAULT_INITIAL_TASK_STATE.to_string(),
            in_progress: DEFAULT_IN_PROGRESS_TASK_STATE.to_string(),
            done: DEFAULT_DONE_TASK_STATE.to_string(),
        }
    }
}

impl TaskStateConfig {
    pub fn validate_or_initial(&self, maybe_state: Option<TaskState>) -> Result<TaskState, Error> {
        match maybe_state {
            Some(state) => {
                if self.states.contains(&state) {
                    Ok(state)
                } else {
                    Err(Error::InvalidTaskState(state, self.states.clone()))
                }
            }
            None => Ok(self.initial().to_string()),
        }
    }

    pub fn states(&self) -> impl Iterator<Item = &str> {
        self.states.iter().map(|s| s.as_str())
    }

    pub fn initial(&self) -> &str {
        self.initial.as_str()
    }

    pub fn in_progress(&self) -> &str {
        self.in_progress.as_str()
    }

    pub fn done(&self) -> &str {
        self.done.as_str()
    }
}

/// Loiter global configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    task_state_config: TaskStateConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            task_state_config: TaskStateConfig::default(),
        }
    }
}

impl Config {
    pub fn with_task_state_config(mut self, config: &TaskStateConfig) -> Self {
        self.task_state_config = config.clone();
        self
    }

    pub fn task_state_config(&self) -> &TaskStateConfig {
        &self.task_state_config
    }
}

/// For keeping track of the current global time tracking state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct State {
    active_log: Option<(ProjectId, Option<TaskId>, LogId)>,
}

impl Default for State {
    fn default() -> Self {
        Self { active_log: None }
    }
}

impl State {
    pub fn with_active_log<S: AsRef<str>>(
        mut self,
        project_id: S,
        maybe_task_id: Option<TaskId>,
        log_id: LogId,
    ) -> Self {
        self.active_log = Some((project_id.as_ref().to_string(), maybe_task_id, log_id));
        self
    }

    pub fn with_no_active_log(mut self) -> Self {
        self.active_log = None;
        self
    }

    pub fn active_log(&self) -> Option<(ProjectId, Option<TaskId>, LogId)> {
        self.active_log.clone()
    }
}

/// The direction in which to sort items.
#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq)]
pub enum SortDir {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

impl Default for SortDir {
    fn default() -> Self {
        Self::Asc
    }
}

impl FromStr for SortDir {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "asc" | "a" => Self::Asc,
            "desc" | "d" => Self::Desc,
            _ => return Err(Error::UnrecognizedSortDir(s.to_string())),
        })
    }
}

impl std::fmt::Display for SortDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Asc => "asc",
                Self::Desc => "desc",
            }
        )
    }
}

/// The fields on which project listings can be sorted.
#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq)]
pub enum ProjectField {
    Id,
    Name,
    Description,
    Deadline,
}

impl ProjectField {
    pub fn sort(&self, a: &Project, b: &Project) -> std::cmp::Ordering {
        match self {
            Self::Id => a.id().cmp(b.id()),
            Self::Name => a.name().cmp(b.name()),
            Self::Description => a.description().cmp(&b.description()),
            Self::Deadline => a.deadline().cmp(&b.deadline()),
        }
    }
}

impl Default for ProjectField {
    fn default() -> Self {
        Self::Id
    }
}

impl FromStr for ProjectField {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "id" => Self::Id,
            "name" => Self::Name,
            "description" | "desc" => Self::Description,
            "deadline" => Self::Deadline,
            _ => return Err(Error::UnrecognizedProjectField(s.to_string())),
        })
    }
}

impl std::fmt::Display for ProjectField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Id => "id",
                Self::Name => "name",
                Self::Description => "description",
                Self::Deadline => "deadline",
            }
        )
    }
}

/// The central unit of organization in Loiter that groups together tasks and
/// work logs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    #[serde(skip)]
    id: ProjectId,
    name: String,
    #[serde(rename = "description")]
    maybe_description: Option<String>,
    #[serde(rename = "deadline")]
    maybe_deadline: Option<Timestamp>,
    tags: HashSet<String>,
    #[serde(rename = "task_state_config")]
    maybe_task_state_config: Option<TaskStateConfig>,
}

impl Project {
    /// Minimal constructor.
    pub fn new<S: AsRef<str>>(name: S) -> Self {
        let name = name.as_ref().to_string();
        Self {
            id: slugify(&name),
            name,
            maybe_description: None,
            maybe_deadline: None,
            tags: HashSet::new(),
            maybe_task_state_config: None,
        }
    }

    pub fn with_name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.name = name.as_ref().to_string();
        self.id = slugify(&self.name);
        self
    }

    pub fn with_description<S: AsRef<str>>(mut self, description: S) -> Self {
        self.maybe_description = Some(description.as_ref().to_string());
        self
    }

    pub fn with_maybe_description(mut self, maybe_description: Option<String>) -> Self {
        self.maybe_description = maybe_description;
        self
    }

    pub fn with_deadline(mut self, deadline: Timestamp) -> Self {
        self.maybe_deadline = Some(deadline);
        self
    }

    pub fn with_maybe_deadline(mut self, maybe_deadline: Option<Timestamp>) -> Self {
        self.maybe_deadline = maybe_deadline;
        self
    }

    pub fn with_tags<S, T>(mut self, tags: T) -> Result<Self, Error>
    where
        S: AsRef<str>,
        T: IntoIterator<Item = S>,
    {
        self.tags = tags
            .into_iter()
            .map(|t| validate_tag(&t))
            .collect::<Result<HashSet<String>, Error>>()?;
        Ok(self)
    }

    pub fn with_task_state_config(mut self, config: &TaskStateConfig) -> Self {
        self.maybe_task_state_config = Some(config.clone());
        self
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.maybe_description.as_deref()
    }

    pub fn deadline(&self) -> Option<&Timestamp> {
        self.maybe_deadline.as_ref()
    }

    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.tags.iter().map(|t| t.as_str())
    }

    pub fn task_state_config(&self) -> Option<&TaskStateConfig> {
        self.maybe_task_state_config.as_ref()
    }
}

/// The fields on which task listings can be sorted.
#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq)]
pub enum TaskField {
    Id,
    ProjectId,
    Description,
    State,
    Deadline,
}

impl TaskField {
    pub fn sort(&self, a: &Task, b: &Task) -> std::cmp::Ordering {
        match self {
            Self::Id => a.id().cmp(&b.id()),
            Self::ProjectId => a.project_id().cmp(&b.project_id()),
            Self::Description => a.description().cmp(b.description()),
            Self::State => a.state().cmp(b.state()),
            Self::Deadline => a.deadline().cmp(&b.deadline()),
        }
    }
}

impl Default for TaskField {
    fn default() -> Self {
        Self::Id
    }
}

impl FromStr for TaskField {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "id" => Self::Id,
            "project_id" | "project-id" | "project" => Self::ProjectId,
            "description" | "desc" => Self::Description,
            "state" => Self::State,
            "deadline" => Self::Deadline,
            _ => return Err(Error::UnrecognizedTaskField(s.to_string())),
        })
    }
}

impl std::fmt::Display for TaskField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Id => "id",
                Self::ProjectId => "project-id",
                Self::Description => "description",
                Self::State => "state",
                Self::Deadline => "deadline",
            }
        )
    }
}

/// A discrete unit of work related to a project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    #[serde(skip)]
    maybe_project_id: Option<ProjectId>,
    #[serde(skip)]
    maybe_id: Option<TaskId>,
    description: String,
    state: TaskState,
    #[serde(rename = "deadline")]
    maybe_deadline: Option<Timestamp>,
    tags: HashSet<String>,
}

impl Task {
    /// Constructor.
    pub fn new<S1, S2, S3>(project_id: S1, description: S2, state: S3) -> Self
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
        S3: AsRef<str>,
    {
        Self {
            maybe_project_id: Some(project_id.as_ref().to_string()),
            maybe_id: None,
            description: description.as_ref().to_string(),
            state: state.as_ref().to_string(),
            maybe_deadline: None,
            tags: HashSet::new(),
        }
    }

    pub fn with_project_id<S: AsRef<str>>(mut self, project_id: S) -> Self {
        self.maybe_project_id = Some(project_id.as_ref().to_string());
        self
    }

    pub fn with_id(mut self, id: TaskId) -> Self {
        self.maybe_id = Some(id);
        self
    }

    pub fn with_description<S: AsRef<str>>(mut self, description: S) -> Self {
        self.description = description.as_ref().to_string();
        self
    }

    pub fn with_state<S: AsRef<str>>(mut self, state: S) -> Self {
        self.state = state.as_ref().to_string();
        self
    }

    pub fn with_deadline(mut self, deadline: Timestamp) -> Self {
        self.maybe_deadline = Some(deadline);
        self
    }

    pub fn with_maybe_deadline(mut self, maybe_deadline: Option<Timestamp>) -> Self {
        self.maybe_deadline = maybe_deadline;
        self
    }

    pub fn with_tags<S, T>(mut self, tags: T) -> Result<Self, Error>
    where
        S: AsRef<str>,
        T: IntoIterator<Item = S>,
    {
        self.tags = tags
            .into_iter()
            .map(|t| validate_tag(&t))
            .collect::<Result<HashSet<String>, Error>>()?;
        Ok(self)
    }

    pub fn project_id(&self) -> Option<&str> {
        self.maybe_project_id.as_deref()
    }

    pub fn id(&self) -> Option<TaskId> {
        self.maybe_id
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn state(&self) -> &str {
        self.state.as_str()
    }

    pub fn deadline(&self) -> Option<&Timestamp> {
        self.maybe_deadline.as_ref()
    }

    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.tags.iter().map(|t| t.as_str())
    }
}

/// The fields on which work log listings can be sorted.
#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq)]
pub enum LogField {
    Id,
    ProjectId,
    TaskId,
    Start,
    Duration,
    Comment,
}

impl LogField {
    pub fn sort(&self, a: &Log, b: &Log) -> std::cmp::Ordering {
        match self {
            Self::Id => a.id().cmp(&b.id()),
            Self::ProjectId => a.project_id().cmp(&b.project_id()),
            Self::TaskId => a.task_id().cmp(&b.task_id()),
            Self::Start => a.start().cmp(&b.start()),
            Self::Duration => a.duration().cmp(&b.duration()),
            Self::Comment => a.comment().cmp(&b.comment()),
        }
    }
}

impl Default for LogField {
    fn default() -> Self {
        Self::Id
    }
}

impl FromStr for LogField {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "id" => Self::Id,
            "project_id" | "project-id" | "project" => Self::ProjectId,
            "task_id" | "task-id" | "task" => Self::TaskId,
            "start" => Self::Start,
            "duration" => Self::Duration,
            "comment" => Self::Comment,
            _ => return Err(Error::UnrecognizedLogField(s.to_string())),
        })
    }
}

impl std::fmt::Display for LogField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Id => "id",
                Self::ProjectId => "project-id",
                Self::TaskId => "task-id",
                Self::Start => "start",
                Self::Duration => "duration",
                Self::Comment => "comment",
            }
        )
    }
}

/// A log of work done or currently underway.
///
/// Always associated with a project, but optionally associated with a specific
/// task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Log {
    #[serde(skip)]
    maybe_project_id: Option<ProjectId>,
    #[serde(skip)]
    maybe_task_id: Option<TaskId>,
    #[serde(skip)]
    maybe_id: Option<LogId>,
    #[serde(rename = "start")]
    maybe_start: Option<Timestamp>,
    #[serde(rename = "duration")]
    maybe_duration: Option<Duration>,
    #[serde(rename = "description")]
    maybe_comment: Option<String>,
    tags: HashSet<String>,
}

impl Log {
    /// Constructor with required fields.
    pub fn new<S: AsRef<str>>(project_id: S) -> Self {
        Self {
            maybe_project_id: Some(project_id.as_ref().to_string()),
            maybe_task_id: None,
            maybe_id: None,
            maybe_start: None,
            maybe_duration: None,
            maybe_comment: None,
            tags: HashSet::new(),
        }
    }

    pub fn with_project_id<S: AsRef<str>>(mut self, project_id: S) -> Self {
        self.maybe_project_id = Some(project_id.as_ref().to_string());
        self
    }

    pub fn with_maybe_project_id(mut self, maybe_project_id: Option<ProjectId>) -> Self {
        self.maybe_project_id = maybe_project_id;
        self
    }

    pub fn with_task_id(mut self, task_id: TaskId) -> Self {
        self.maybe_task_id = Some(task_id);
        self
    }

    pub fn with_maybe_task_id(mut self, maybe_task_id: Option<TaskId>) -> Self {
        self.maybe_task_id = maybe_task_id;
        self
    }

    pub fn with_id(mut self, id: LogId) -> Self {
        self.maybe_id = Some(id);
        self
    }

    pub fn with_start(mut self, start: Timestamp) -> Self {
        self.maybe_start = Some(start);
        self
    }

    pub fn with_maybe_start(mut self, maybe_start: Option<Timestamp>) -> Self {
        self.maybe_start = maybe_start;
        self
    }

    pub fn with_stop(mut self, stop: Timestamp) -> Result<Self, Error> {
        let start_time: OffsetDateTime = self
            .maybe_start
            .clone()
            .ok_or(Error::LogWithoutStartCannotStop)?
            .into();
        let stop_time: OffsetDateTime = stop.into();
        if stop_time < start_time {
            return Err(Error::LogCannotStopBeforeStart);
        }
        let duration = stop_time - start_time;
        self.maybe_duration = Some(duration.into());
        Ok(self)
    }

    pub fn with_maybe_stop(self, maybe_stop: Option<Timestamp>) -> Result<Self, Error> {
        match maybe_stop {
            Some(stop) => self.with_stop(stop),
            None => Ok(self),
        }
    }

    pub fn with_duration(self, duration: Duration) -> Self {
        self.with_maybe_duration(Some(duration))
    }

    pub fn with_maybe_duration(mut self, maybe_duration: Option<Duration>) -> Self {
        self.maybe_duration = maybe_duration;
        self
    }

    pub fn with_maybe_duration_or_stop(
        self,
        maybe_duration: Option<Duration>,
        maybe_stop: Option<Timestamp>,
    ) -> Result<Self, Error> {
        if maybe_duration.is_some() && maybe_stop.is_some() {
            Err(Error::CannotAcceptDurationAndStop)
        } else if let Some(duration) = maybe_duration {
            Ok(self.with_duration(duration))
        } else if let Some(stop) = maybe_stop {
            self.with_stop(stop)
        } else {
            Ok(self)
        }
    }

    pub fn with_comment<S: AsRef<str>>(mut self, comment: S) -> Self {
        self.maybe_comment = Some(comment.as_ref().to_string());
        self
    }

    pub fn with_maybe_comment(mut self, maybe_comment: Option<String>) -> Self {
        self.maybe_comment = maybe_comment;
        self
    }

    pub fn with_tags<S, T>(mut self, tags: T) -> Result<Self, Error>
    where
        S: AsRef<str>,
        T: IntoIterator<Item = S>,
    {
        self.tags = tags
            .into_iter()
            .map(|t| validate_tag(&t))
            .collect::<Result<HashSet<String>, Error>>()?;
        Ok(self)
    }

    pub fn project_id(&self) -> Option<&str> {
        self.maybe_project_id.as_deref()
    }

    pub fn task_id(&self) -> Option<TaskId> {
        self.maybe_task_id
    }

    pub fn id(&self) -> Option<LogId> {
        self.maybe_id
    }

    pub fn start(&self) -> Option<&Timestamp> {
        self.maybe_start.as_ref()
    }

    pub fn duration(&self) -> Option<&Duration> {
        self.maybe_duration.as_ref()
    }

    pub fn comment(&self) -> Option<&str> {
        self.maybe_comment.as_deref()
    }

    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.tags.iter().map(|t| t.as_str())
    }
}

fn validate_tag<S: AsRef<str>>(tag: S) -> Result<String, Error> {
    let tag = tag.as_ref().to_lowercase();
    let has_invalid_chars = tag
        .chars()
        .filter(|c| !matches!(c, 'a'..='z' | '0'..='9' | '-' | '_'))
        .count()
        > 0;
    if has_invalid_chars {
        Err(Error::TagHasInvalidChars(tag))
    } else {
        Ok(tag)
    }
}
