//! User-oriented functionality for interacting with Loiter stores.

use crate::{
    Duration, Error, Log, LogField, Project, ProjectField, ProjectId, SortSpec, Store, Task,
    TaskField, TaskId, TaskState, Timestamp,
};
use log::debug;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use structopt::StructOpt;

/// Add a new project.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct AddProject {
    /// The name of the project to add.
    pub name: String,

    /// Optionally add a description for the project.
    #[structopt(name = "description", short, long)]
    #[serde(rename = "description")]
    pub maybe_description: Option<String>,

    /// Optionally add a deadline for the project.
    #[structopt(name = "deadline", short = "e", long)]
    #[serde(rename = "deadline")]
    pub maybe_deadline: Option<Timestamp>,

    /// Optionally add tags for the project (a comma-separated string, e.g.
    /// "work,coding,ux").
    #[structopt(name = "tags", long)]
    #[serde(rename = "tags")]
    pub maybe_tags: Option<String>,
}

impl TryFrom<&AddProject> for Project {
    type Error = Error;

    fn try_from(cmd: &AddProject) -> Result<Self, Self::Error> {
        Project::new(&cmd.name)
            .with_maybe_description(cmd.maybe_description.clone())
            .with_maybe_deadline(cmd.maybe_deadline)
            .with_tags(parse_tags(cmd.maybe_tags.clone()))
    }
}

/// Add a new task for a project.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct AddTask {
    /// The ID of the project to which this task belongs.
    pub project_id: ProjectId,

    /// A short, human-readable description of what this task is about.
    pub description: String,

    /// The desired state of the task once added.
    #[structopt(name = "state", short, long)]
    #[serde(rename = "state")]
    pub maybe_state: Option<TaskState>,

    /// An optional deadline for this project.
    #[structopt(name = "deadline", short, long)]
    #[serde(rename = "deadline")]
    pub maybe_deadline: Option<Timestamp>,

    /// Tags to associate with this task, separated by commas (e.g.
    /// "work,engineering,ux").
    #[structopt(name = "tags", long)]
    #[serde(rename = "tags")]
    pub maybe_tags: Option<String>,
}

impl TryFrom<&AddTask> for Task {
    type Error = Error;

    fn try_from(cmd: &AddTask) -> Result<Self, Self::Error> {
        Task::new(&cmd.project_id, &cmd.description)
            .with_maybe_state(cmd.maybe_state.clone())
            .with_maybe_deadline(cmd.maybe_deadline)
            .with_tags(parse_tags(cmd.maybe_tags.clone()))
    }
}

/// Update a specific task.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct UpdateTask {
    /// The ID of the project whose task must be updated.
    pub project_id: ProjectId,

    /// The ID of the task to update.
    pub task_id: TaskId,

    /// Update the task description.
    #[structopt(name = "description", short, long)]
    #[serde(rename = "description")]
    pub maybe_description: Option<String>,

    /// Update the state of the task.
    #[structopt(name = "state", short, long)]
    #[serde(rename = "state")]
    pub maybe_state: Option<TaskState>,

    /// Update the deadline for the task.
    #[structopt(name = "deadline", long)]
    #[serde(rename = "deadline")]
    pub maybe_deadline: Option<Timestamp>,

    /// Update the task's tags.
    #[structopt(name = "tags", long)]
    #[serde(rename = "tags")]
    pub maybe_tags: Option<String>,
}

impl UpdateTask {
    /// Apply this update to the given task.
    pub fn apply(&self, task: &Task) -> Result<Task, Error> {
        let mut task = task.clone();
        if let Some(description) = &self.maybe_description {
            task = task.with_description(description);
        }
        if let Some(state) = &self.maybe_state {
            task = task.with_state(state);
        }
        if let Some(deadline) = &self.maybe_deadline {
            task = task.with_deadline(*deadline);
        }
        if let Some(tags) = &self.maybe_tags {
            task = task.with_tags(parse_tags(Some(tags.clone())))?;
        }
        Ok(task)
    }
}

/// Add a completed work log for a project or task.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct AddLog {
    /// The ID of the project to which to add this work log.
    #[structopt(name = "project")]
    pub project_id: ProjectId,

    /// Optionally, the ID of the task to which this work log relates.
    #[structopt(name = "task", short, long)]
    #[serde(rename = "task_id")]
    pub maybe_task_id: Option<TaskId>,

    /// Optionally specify the start time for this log.
    #[structopt(name = "from", long)]
    #[serde(rename = "start")]
    pub maybe_start: Option<Timestamp>,

    /// Optionally specify the stop time for this log (cannot be used with
    /// duration).
    #[structopt(name = "to", long)]
    #[serde(rename = "stop")]
    pub maybe_stop: Option<Timestamp>,

    /// Optionally specify the duration of this log, in seconds (cannot be
    /// used with stop time).
    #[structopt(name = "duration", short, long)]
    #[serde(rename = "duration")]
    pub maybe_duration: Option<Duration>,

    /// An optional comment of what is/was being done in this work log.
    #[structopt(name = "comment", short, long)]
    #[serde(rename = "comment")]
    pub maybe_comment: Option<String>,

    /// Tags to associate with this work log, separated by commas (e.g.
    /// "work,coding").
    #[structopt(name = "tags", long)]
    #[serde(rename = "tags")]
    pub maybe_tags: Option<String>,
}

impl TryFrom<&AddLog> for Log {
    type Error = Error;

    fn try_from(cmd: &AddLog) -> Result<Self, Self::Error> {
        Log::new(&cmd.project_id)
            .with_maybe_task_id(cmd.maybe_task_id)
            .with_maybe_start(cmd.maybe_start)
            .with_maybe_duration_or_stop(cmd.maybe_duration, cmd.maybe_stop)?
            .with_maybe_comment(cmd.maybe_comment.clone())
            .with_tags(parse_tags(cmd.maybe_tags.clone()))
    }
}

/// Start a new work log for a project or task.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct StartLog {
    /// The ID of the project to which to add this work log.
    #[structopt(name = "project")]
    pub project_id: ProjectId,

    /// Optionally, the ID of the task to which this work log relates.
    #[structopt(name = "task", long)]
    #[serde(rename = "task_id")]
    pub maybe_task_id: Option<TaskId>,

    /// Optionally, the date/time from which to start this log. Defaults to the
    /// current local time.
    #[structopt(name = "from", short, long, default_value = "now")]
    pub start: Timestamp,

    /// An optional comment describing what is/was being done in this work log.
    #[structopt(name = "comment", short, long)]
    pub maybe_comment: Option<String>,

    /// Tags to associate with this work log, separated by commas (e.g.
    /// "work,coding").
    #[structopt(name = "tags", long)]
    pub maybe_tags: Option<String>,
}

impl TryFrom<&StartLog> for Log {
    type Error = Error;

    fn try_from(cmd: &StartLog) -> Result<Self, Self::Error> {
        Log::new(&cmd.project_id)
            .with_maybe_task_id(cmd.maybe_task_id)
            .with_start(cmd.start)
            .with_maybe_comment(cmd.maybe_comment.clone())
            .with_tags(parse_tags(cmd.maybe_tags.clone()))
    }
}

/// Stop the currently active work log.
#[derive(Debug, Clone, Default, StructOpt, Serialize, Deserialize)]
pub struct StopLog {
    /// Optionally specify the time at which the current work log should be
    /// stopped. If not given, and no duration is given, the current
    /// date/time will be used.
    #[structopt(name = "at", long)]
    #[serde(rename = "stop")]
    pub maybe_stop_time: Option<Timestamp>,

    #[structopt(name = "duration", short, long)]
    #[serde(rename = "duration")]
    pub maybe_duration: Option<Duration>,

    /// An optional comment of what was done in this work log.
    #[structopt(name = "comment", short, long)]
    #[serde(rename = "comment")]
    pub maybe_comment: Option<String>,

    /// Tags to associate with this work log, separated by commas (e.g.
    /// "work,coding").
    #[structopt(name = "tags", long)]
    #[serde(rename = "tags")]
    pub maybe_tags: Option<String>,
}

/// List all projects.
#[derive(Debug, Clone, Default, StructOpt, Serialize, Deserialize)]
pub struct ListProjects {
    /// Show project details as opposed to just project names and IDs.
    #[structopt(short, long)]
    pub detailed: bool,

    /// Optionally sort the projects by specific fields (e.g. "name" will sort
    /// projects in ascending order by name; "name:desc" will sort by name in
    /// descending order; "deadline,name" will first sort by deadline and then
    /// by name).
    #[structopt(short, long)]
    pub sort: Option<String>,
}

/// List all of the tasks for a project.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct ListTasks {
    /// The ID of the project whose tasks must be listed.
    #[structopt(name = "project")]
    pub project_id: ProjectId,

    /// Optionally sort the tasks by specific fields (e.g. "id" will sort tasks
    /// in ascending order by ID; "id:desc" will sort by ID in descending order;
    /// "deadline,id" will first sort by deadline and then by ID).
    #[structopt(short, long)]
    pub sort: Option<String>,
}

/// List all of the logs for a project or task.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct ListLogs {
    /// The ID of the project whose logs must be listed.
    #[structopt(name = "project")]
    pub project_id: ProjectId,

    /// The ID of the task whose logs must be listed (if applicable).
    #[structopt(name = "task", short, long)]
    pub maybe_task_id: Option<TaskId>,

    /// Show logs' details as opposed to summary.
    #[structopt(short, long)]
    pub detailed: bool,

    /// Optionally sort the logs by specific fields (e.g. "id" will sort logs in
    /// ascending order by ID; "id:desc" will sort by ID in descending order;
    /// "duration,id" will first sort by duration and then by ID).
    #[structopt(short, long)]
    pub sort: Option<String>,
}

#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct TaskStates {
    #[structopt(name = "project", short, long)]
    pub maybe_project_id: Option<ProjectId>,
}

/// Add a new project to the given store.
pub fn add_project(store: &Store, params: &AddProject) -> Result<Project, Error> {
    let project = Project::try_from(params)?;
    store.save_project(&project)?;
    debug!("Created new project {}", project.name());
    Ok(project)
}

/// Add a new task for a specific project to the store.
pub fn add_task(store: &Store, params: &AddTask) -> Result<Task, Error> {
    let task = Task::try_from(params)?;
    let task = store.save_task(&task)?;
    debug!(
        "Added task {} for project {}",
        task.id().unwrap(),
        task.project_id().unwrap()
    );
    Ok(task)
}

/// Update one or more fields of a specific task.
pub fn update_task(store: &Store, params: &UpdateTask) -> Result<Task, Error> {
    let task = store.task(&params.project_id, params.task_id)?;
    let task = store.save_task(&params.apply(&task)?)?;
    debug!(
        "Updated task {} for project {}",
        task.id().unwrap(),
        task.project_id().unwrap(),
    );
    Ok(task)
}

/// Add a new log for a project or task.
pub fn add_log(store: &Store, params: &AddLog) -> Result<Log, Error> {
    let log = Log::try_from(params)?;
    let log = store.save_log(&log)?;
    debug!(
        "Added log {} for project {}{}",
        log.id().unwrap(),
        log.project_id().unwrap(),
        log.task_id()
            .map(|task_id| format!(", task {}", task_id))
            .unwrap_or_else(|| "".to_string())
    );
    Ok(log)
}

/// Start tracking time for a new log.
pub fn start_log(store: &Store, params: &StartLog) -> Result<Log, Error> {
    let state = store.state()?;
    // Stop any active log
    if state.active_log().is_some() {
        let _ = stop_log(store, &StopLog::default())?;
    }
    let log = Log::try_from(params)?;
    let log = store.save_log(&log)?;
    let state = state.with_active_log(log.project_id().unwrap(), log.task_id(), log.id().unwrap());
    store.save_state(&state)?;
    // TODO: If this is associated with a task, change the task's status to in
    // progress.
    debug!(
        "Started log {} for project {}{} at {}",
        log.id().unwrap(),
        log.project_id().unwrap(),
        log.task_id()
            .map(|task_id| format!(", task {},", task_id))
            .unwrap_or_else(|| "".to_string()),
        log.start().unwrap(),
    );
    Ok(log)
}

/// Stop tracking time for the currently active log.
pub fn stop_log(store: &Store, params: &StopLog) -> Result<Log, Error> {
    let state = store.state()?;
    let mut active_log = match state.active_log() {
        Some((project_id, maybe_task_id, log_id)) => {
            store.log(&project_id, maybe_task_id, log_id)?
        }
        None => return Err(Error::NoActiveLog),
    }
    .with_duration_or_stop_or_now(params.maybe_duration, params.maybe_stop_time)?;

    // Optionally update the comment and tags
    if let Some(comment) = &params.maybe_comment {
        active_log = active_log.with_comment(comment);
    }
    if let Some(tags) = &params.maybe_tags {
        active_log = active_log.with_tags(parse_tags(Some(tags.clone())))?;
    }

    let active_log = store.save_log(&active_log)?;
    let state = state.with_no_active_log();
    store.save_state(&state)?;
    debug!(
        "Stopped log {} for project {}{} at {} ({})",
        active_log.id().unwrap(),
        active_log.project_id().unwrap(),
        active_log
            .task_id()
            .map(|task_id| format!(", task {},", task_id))
            .unwrap_or_else(|| "".to_string()),
        active_log.stop().unwrap(),
        active_log.duration().unwrap(),
    );
    Ok(active_log)
}

/// Cancels the active work log, if any.
pub fn cancel_log(store: &Store) -> Result<Option<Log>, Error> {
    let state = store.state()?;
    let active_log = match state.active_log() {
        Some((project_id, maybe_task_id, log_id)) => {
            store.log(&project_id, maybe_task_id, log_id)?
        }
        None => return Ok(None),
    };
    store.delete_log(
        active_log.project_id().unwrap(),
        active_log.task_id(),
        active_log.id().unwrap(),
    )?;
    let state = state.with_no_active_log();
    store.save_state(&state)?;
    debug!(
        "Cancelled log {} for project {}{}",
        active_log.id().unwrap(),
        active_log.project_id().unwrap(),
        active_log
            .task_id()
            .map(|task_id| format!(", task {},", task_id))
            .unwrap_or_else(|| "".to_string()),
    );
    Ok(Some(active_log))
}

/// List projects, optionally sorting them.
///
/// Returns the rendered table containing the results.
pub fn list_projects(store: &Store, params: &ListProjects) -> Result<Vec<Project>, Error> {
    let mut projects = store.projects()?;
    if let Some(sort) = &params.sort {
        let sort_spec = SortSpec::<ProjectField>::from_str(sort)?;
        projects = sort_spec.sort(projects);
    }
    Ok(projects)
}

/// List tasks for a particular project, optionally sorting them.
///
/// Returns the rendered table containing the results.
pub fn list_tasks(store: &Store, params: &ListTasks) -> Result<Vec<Task>, Error> {
    let mut tasks = store.tasks(&params.project_id)?;
    if let Some(sort) = &params.sort {
        let sort_spec = SortSpec::<TaskField>::from_str(sort)?;
        tasks = sort_spec.sort(tasks);
    }
    Ok(tasks)
}

/// List work logs, filtered and ordered by the given parameters.
pub fn list_logs(store: &Store, params: &ListLogs) -> Result<Vec<Log>, Error> {
    let mut logs = store.logs(&params.project_id, params.maybe_task_id)?;
    if let Some(sort) = &params.sort {
        let sort_spec = SortSpec::<LogField>::from_str(sort)?;
        logs = sort_spec.sort(logs);
    }
    Ok(logs)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStatus {
    pub log: Log,
    pub active_for: Duration,
}

/// Returns the status of the currently active log.
pub fn active_log_status(store: &Store) -> Result<Option<LogStatus>, Error> {
    let state = store.state()?;
    match state.active_log() {
        Some((project_id, maybe_task_id, log_id)) => {
            let log = store.log(&project_id, maybe_task_id, log_id)?;
            let start = log.start().unwrap();
            let active_for = Timestamp::now()? - start;
            Ok(Some(LogStatus { log, active_for }))
        }
        None => Ok(None),
    }
}

/// Shows a list of task states. If no project is supplied, the default
/// configuration will be shown.
pub fn task_states(store: &Store, params: &TaskStates) -> Result<Vec<TaskState>, Error> {
    let config = store.config()?;
    let default_tsc = config.task_state_config();
    let maybe_project = match &params.maybe_project_id {
        Some(id) => Some(store.project(id)?),
        None => None,
    };
    let states = maybe_project
        .map(|p| p.task_states(default_tsc))
        .unwrap_or_else(|| {
            default_tsc
                .states()
                .map(|s| s.to_string())
                .collect::<Vec<TaskState>>()
        });
    Ok(states)
}

fn parse_tags(maybe_tags: Option<String>) -> Vec<String> {
    maybe_tags
        .map(|tags| {
            tags.split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<String>>()
        })
        .unwrap_or_else(Vec::new)
}
