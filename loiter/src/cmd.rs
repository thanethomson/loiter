//! User-oriented functionality for interacting with Loiter stores.

use crate::{
    is_dir, is_file, Duration, DurationFilter, Error, FilterSpec, Log, LogField, LogFilter, LogId,
    Project, ProjectField, ProjectFilter, ProjectId, SortSpec, Store, Task, TaskField, TaskFilter,
    TaskId, TaskPriority, TaskState, Timestamp, TimestampFilter,
};
use log::debug;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    str::FromStr,
};
use structopt::StructOpt;

const GITIGNORE: &str = r#"*.swp
*.bak
"#;

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
            .with_tags(parse_comma_separated(cmd.maybe_tags.clone()))
    }
}

/// Remove a project and all of its related data.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct RemoveProject {
    /// The ID of the project to remove.
    pub id: String,
}

/// Add a new task for a project.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct AddTask {
    /// The ID of the project to which this task belongs.
    pub project_id: ProjectId,

    /// A short, human-readable description of what this task is about.
    pub description: String,

    /// The desired priority of the task (lower values correspond to higher
    /// priority, i.e. 1 is the highest priority, and 10 is the lowest).
    #[structopt(short, long, default_value = "10")]
    pub priority: TaskPriority,

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
            .with_priority(cmd.priority)?
            .with_maybe_state(cmd.maybe_state.clone())
            .with_maybe_deadline(cmd.maybe_deadline)
            .with_tags(parse_comma_separated(cmd.maybe_tags.clone()))
    }
}

/// Update one or more specific tasks.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct UpdateTask {
    /// The ID of the project whose task(s) must be updated.
    pub project_id: ProjectId,

    /// The ID(s) of the task(s) to update (comma-separated).
    pub task_ids: String,

    /// Update the task description.
    #[structopt(name = "description", short, long)]
    #[serde(rename = "description")]
    pub maybe_description: Option<String>,

    /// Update the task priority.
    #[structopt(name = "priority", short, long)]
    #[serde(rename = "priority")]
    pub maybe_priority: Option<TaskPriority>,

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
        if let Some(priority) = &self.maybe_priority {
            task = task.with_priority(*priority)?;
        }
        if let Some(state) = &self.maybe_state {
            task = task.with_state(state);
        }
        if let Some(deadline) = &self.maybe_deadline {
            task = task.with_deadline(*deadline);
        }
        if let Some(tags) = &self.maybe_tags {
            task = task.with_tags(parse_comma_separated(Some(tags.clone())))?;
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
    #[structopt(name = "task")]
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
            .with_tags(parse_comma_separated(cmd.maybe_tags.clone()))
    }
}

/// Start a new work log for a project or task.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct StartLog {
    /// The ID of the project to which to add this work log.
    #[structopt(name = "project")]
    pub project_id: ProjectId,

    /// Optionally, the ID of the task to which this work log relates.
    #[structopt(name = "task")]
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
            .with_tags(parse_comma_separated(cmd.maybe_tags.clone()))
    }
}

/// Stop a work log.
///
/// By default this stops the currently active work log, unless a project and
/// log ID (and possibly task ID) are provided.
#[derive(Debug, Clone, Default, StructOpt, Serialize, Deserialize)]
pub struct StopLog {
    /// For specifying a specific work log to stop.
    #[structopt(name = "project", short, long)]
    pub maybe_project_id: Option<ProjectId>,

    /// For specifying a specific work log to stop.
    #[structopt(name = "task", short, long)]
    pub maybe_task_id: Option<TaskId>,

    /// For specifying a specific work log to stop.
    #[structopt(name = "id", short, long)]
    pub maybe_id: Option<LogId>,

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

/// Cancel a work log.
///
/// By default this cancels the currently active work log, unless a project and
/// log ID (and possibly task ID) are provided.
#[derive(Debug, Clone, Default, StructOpt, Serialize, Deserialize)]
pub struct CancelLog {
    /// For specifying a specific work log to cancel.
    #[structopt(name = "project", short, long)]
    pub maybe_project_id: Option<ProjectId>,

    /// For specifying a specific work log to cancel.
    #[structopt(name = "task", short, long)]
    pub maybe_task_id: Option<TaskId>,

    /// For specifying a specific work log to cancel.
    #[structopt(name = "id", short, long)]
    pub maybe_id: Option<LogId>,
}

/// List all projects.
#[derive(Debug, Clone, Default, StructOpt, Serialize, Deserialize)]
pub struct ListProjects {
    /// Show project details as opposed to just project names and IDs.
    #[structopt(short, long)]
    pub detailed: bool,

    /// Only return projects matching this deadline filter.
    #[structopt(name = "deadline", long)]
    pub maybe_deadline_filter: Option<String>,

    /// Only return projects matching one or more of these tags.
    #[structopt(name = "tags", long)]
    pub maybe_tags_filter: Option<String>,

    /// Optionally sort the projects by specific fields (e.g. "name" will sort
    /// projects in ascending order by name; "name:desc" will sort by name in
    /// descending order; "deadline,name" will first sort by deadline and then
    /// by name).
    #[structopt(short, long, default_value = "id")]
    pub sort: String,
}

/// List all of the tasks for a project.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct ListTasks {
    /// Only return tasks whose project matches these project IDs
    /// (comma-separated).
    #[structopt(name = "project")]
    pub maybe_project_ids: Option<String>,

    /// Only return tasks whose project's deadline matches this filter.
    #[structopt(name = "project-deadline", long)]
    pub maybe_project_deadline_filter: Option<String>,

    /// Only return tasks whose project's tags match one or more of these tags
    /// (comma-separated).
    #[structopt(name = "project-tags", long)]
    pub maybe_project_tags_filter: Option<String>,

    /// Only return tasks whose priority matches one or more of these priorities
    /// (comma-separated).
    #[structopt(name = "priority", long)]
    pub maybe_priority_filter: Option<String>,

    /// Only return tasks whose states match one or more of these states
    /// (comma-separated). By default, we only list tasks that are not done. To
    /// return tasks with any state, use "any".
    #[structopt(name = "state", long, default_value = "!done")]
    pub state_filter: String,

    /// Only return tasks whose deadline matches the given filter.
    #[structopt(name = "deadline", long)]
    pub maybe_deadline_filter: Option<String>,

    /// Only return tasks whose tags match one or more of these states
    /// (comma-separated).
    #[structopt(name = "tags", long)]
    pub maybe_tags_filter: Option<String>,

    /// Optionally sort the tasks by specific fields (e.g. "id" will sort tasks
    /// in ascending order by ID; "id:desc" will sort by ID in descending order;
    /// "deadline,id" will first sort by deadline and then by ID).
    #[structopt(short, long, default_value = "priority,project,id")]
    pub sort: String,
}

/// List all of the logs for a project or task.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct ListLogs {
    /// Only return logs whose project matches these project IDs
    /// (comma-separated).
    #[structopt(name = "projects")]
    pub maybe_project_ids: Option<String>,

    /// Only return logs whose task IDs match one or more of these IDs.
    #[structopt(name = "tasks")]
    pub maybe_task_ids_filter: Option<String>,

    /// Show detailed logs (including comments).
    #[structopt(short, long)]
    pub detailed: bool,

    /// Only return logs whose project's deadline matches this filter.
    #[structopt(name = "project-deadline", long)]
    pub maybe_project_deadline_filter: Option<String>,

    /// Only return logs whose project's tags match one or more of these tags
    /// (comma-separated).
    #[structopt(name = "project-tags", long)]
    pub maybe_project_tags_filter: Option<String>,

    /// Only return logs whose associated task's priority matches one or more of
    /// these priorities (comma-separated).
    #[structopt(name = "task-priority", long)]
    pub maybe_task_priority_filter: Option<String>,

    /// Only return logs whose task's states match one or more of these states
    /// (comma-separated).
    #[structopt(name = "task-state", long)]
    pub maybe_task_state_filter: Option<String>,

    /// Only return logs whose task's deadline matches the given filter.
    #[structopt(name = "task-deadline", long)]
    pub maybe_task_deadline_filter: Option<String>,

    /// Only return logs whose task's tags match one or more of these states
    /// (comma-separated).
    #[structopt(name = "task-tags", long)]
    pub maybe_task_tags_filter: Option<String>,

    /// Only return logs whose start time matches this filter.
    #[structopt(name = "start", long, default_value = "today")]
    pub start_filter: String,

    /// Only return logs whose duration matches this filter.
    #[structopt(name = "duration", long)]
    pub maybe_duration_filter: Option<String>,

    /// Only return logs whose tags match one or more of these tags
    /// (comma-separated).
    #[structopt(name = "tags", long)]
    pub maybe_tags_filter: Option<String>,

    /// Optionally sort the logs by specific fields (e.g. "id" will sort logs in
    /// ascending order by ID; "id:desc" will sort by ID in descending order;
    /// "duration,id" will first sort by duration and then by ID).
    #[structopt(short, long, default_value = "start")]
    pub sort: String,
}

#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct TaskStates {
    #[structopt(name = "project", short, long)]
    pub maybe_project_id: Option<ProjectId>,
}

/// Initialize the local storage so it can be pushed to a remote store.
///
/// Initializes the Loiter home directory as a Git repository and adds a remote
/// as its origin.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct RemoteInit {
    /// The default branch to use when initializing the repository.
    #[structopt(short, long, default_value = "main")]
    pub branch: String,

    /// The remote Git repository to set as origin.
    pub origin: String,
}

/// Add a new project to the given store.
pub fn add_project(store: &Store, params: &AddProject) -> Result<Project, Error> {
    let project = Project::try_from(params)?;
    store.save_project(&project)?;
    debug!("Created new project {}", project.name());
    Ok(project)
}

/// Remove a project and all of its related data from the store.
pub fn remove_project(store: &Store, params: &RemoveProject) -> Result<ProjectId, Error> {
    store.remove_project(&params.id)?;
    debug!("Removed project {}", params.id);
    Ok(params.id.clone())
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

/// Update one or more fields of one or more specific tasks.
pub fn update_tasks(store: &Store, params: &UpdateTask) -> Result<Vec<Task>, Error> {
    let task_ids = parse_comma_separated(Some(params.task_ids.clone()))
        .iter()
        .map(|s| TaskId::from_str(s))
        .collect::<Result<Vec<TaskId>, std::num::ParseIntError>>()
        .map_err(|e| Error::InvalidTaskIds(params.task_ids.clone(), e))?;
    let project_filter = FilterSpec::new(ProjectFilter::Ids(vec![params.project_id.clone()]));
    let task_filter = FilterSpec::new(TaskFilter::Ids(task_ids));
    let tasks = store.tasks(&project_filter, &task_filter)?;
    let updated_tasks = tasks
        .into_iter()
        .map(|task| store.save_task(&params.apply(&task)?))
        .collect::<Result<Vec<Task>, Error>>()?;
    Ok(updated_tasks)
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
    if let Some(task_id) = log.task_id() {
        let project = store.project(log.project_id().unwrap())?;
        let task = store.task(log.project_id().unwrap(), task_id)?;
        let config = store.config()?;
        let task_state_config = project
            .task_state_config()
            .unwrap_or_else(|| config.task_state_config());
        let task = task.with_state(task_state_config.in_progress());
        let task = store.save_task(&task)?;
        debug!("Updated task state for task: {:?}", task);
    }

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
    let invalid_log = params.maybe_project_id.is_some() ^ params.maybe_id.is_some();
    if invalid_log {
        return Err(Error::BothProjectAndLogIdRequired);
    }
    let state = store.state()?;
    let mut selected_active_log = false;
    let (project_id, maybe_task_id, log_id) =
        if let Some(project_id) = params.maybe_project_id.as_ref() {
            (
                project_id.clone(),
                params.maybe_task_id,
                params.maybe_id.unwrap(),
            )
        } else {
            selected_active_log = true;
            state.active_log().ok_or(Error::NoActiveLog)?
        };

    let mut log = store
        .log(&project_id, maybe_task_id, log_id)?
        .with_duration_or_stop_or_now(params.maybe_duration, params.maybe_stop_time)?;

    // Optionally update the comment and tags
    if let Some(comment) = &params.maybe_comment {
        log = log.with_comment(comment);
    }
    if let Some(tags) = &params.maybe_tags {
        log = log.with_tags(parse_comma_separated(Some(tags.clone())))?;
    }

    let log = store.save_log(&log)?;
    if selected_active_log {
        let state = state.with_no_active_log();
        store.save_state(&state)?;
    }
    debug!(
        "Stopped log {} for project {}{} at {} ({})",
        log.id().unwrap(),
        log.project_id().unwrap(),
        log.task_id()
            .map(|task_id| format!(", task {},", task_id))
            .unwrap_or_else(|| "".to_string()),
        log.stop().unwrap(),
        log.duration().unwrap(),
    );
    Ok(log)
}

/// Cancels the active work log, if any.
pub fn cancel_log(store: &Store, params: &CancelLog) -> Result<Option<Log>, Error> {
    let invalid_log = params.maybe_project_id.is_some() ^ params.maybe_id.is_some();
    if invalid_log {
        return Err(Error::BothProjectAndLogIdRequired);
    }
    let state = store.state()?;
    let mut selected_active_log = false;
    let (project_id, maybe_task_id, log_id) =
        if let Some(project_id) = params.maybe_project_id.as_ref() {
            (
                project_id.clone(),
                params.maybe_task_id,
                params.maybe_id.unwrap(),
            )
        } else {
            selected_active_log = true;
            state.active_log().ok_or(Error::NoActiveLog)?
        };

    let log = store.log(&project_id, maybe_task_id, log_id)?;

    store.delete_log(log.project_id().unwrap(), log.task_id(), log.id().unwrap())?;
    if selected_active_log {
        let state = state.with_no_active_log();
        store.save_state(&state)?;
    }
    debug!(
        "Cancelled log {} for project {}{}",
        log.id().unwrap(),
        log.project_id().unwrap(),
        log.task_id()
            .map(|task_id| format!(", task {},", task_id))
            .unwrap_or_else(|| "".to_string()),
    );
    Ok(Some(log))
}

/// List projects, optionally sorting them.
///
/// Returns the rendered table containing the results.
pub fn list_projects(store: &Store, params: &ListProjects) -> Result<Vec<Project>, Error> {
    let filter = build_project_filter(
        None,
        params.maybe_deadline_filter.clone(),
        params.maybe_tags_filter.clone(),
    )?;

    let mut projects = store.projects(&filter)?;
    let sort_spec = SortSpec::<ProjectField>::from_str(&params.sort)?;
    projects = sort_spec.sort(projects);
    Ok(projects)
}

fn build_project_filter(
    maybe_project_ids: Option<String>,
    maybe_deadline: Option<String>,
    maybe_tags: Option<String>,
) -> Result<FilterSpec<ProjectFilter>, Error> {
    let mut filter = FilterSpec::new(ProjectFilter::All);
    if let Some(project_ids) = maybe_project_ids.as_ref() {
        filter = filter.and_then(ProjectFilter::Ids(parse_comma_separated(Some(
            project_ids.clone(),
        ))));
    }
    if let Some(deadline) = maybe_deadline {
        filter = filter.and_then(ProjectFilter::Deadline(TimestampFilter::from_str(
            &deadline,
        )?));
    }
    if let Some(tags) = maybe_tags {
        filter = filter.and_then(ProjectFilter::Tags(parse_comma_separated(Some(tags))));
    }
    Ok(filter)
}

fn build_task_filter(
    maybe_priorities: Option<String>,
    maybe_states: Option<String>,
    maybe_deadline: Option<String>,
    maybe_tags: Option<String>,
) -> Result<FilterSpec<TaskFilter>, Error> {
    let mut filter = FilterSpec::new(TaskFilter::All);
    if let Some(priorities) = maybe_priorities {
        let priorities = parse_comma_separated(Some(priorities.clone()))
            .iter()
            .map(|priority| TaskPriority::from_str(priority))
            .collect::<Result<Vec<TaskPriority>, std::num::ParseIntError>>()
            .map_err(|e| Error::CannotParseTaskPriority(priorities, e))?;
        filter = filter.and_then(TaskFilter::Priority(priorities));
    }
    if let Some(states) = maybe_states {
        let states = parse_comma_separated(Some(states));
        if states.len() == 1 && states[0].starts_with('!') {
            filter = filter.and_then(TaskFilter::StateNot(
                states[0].trim_start_matches('!').to_string(),
            ));
        } else if !states.is_empty() && states[0] != "any" && states[0] != "all" {
            filter = filter.and_then(TaskFilter::State(states));
        }
    }
    if let Some(deadline) = maybe_deadline {
        filter = filter.and_then(TaskFilter::Deadline(TimestampFilter::from_str(&deadline)?));
    }
    if let Some(tags) = maybe_tags {
        filter = filter.and_then(TaskFilter::Tags(parse_comma_separated(Some(tags))));
    }
    Ok(filter)
}

/// List tasks for a particular project, optionally sorting them.
///
/// Returns the rendered table containing the results.
pub fn list_tasks(store: &Store, params: &ListTasks) -> Result<Vec<Task>, Error> {
    let project_filter = build_project_filter(
        params.maybe_project_ids.clone(),
        params.maybe_project_deadline_filter.clone(),
        params.maybe_project_tags_filter.clone(),
    )?;
    let task_filter = build_task_filter(
        params.maybe_priority_filter.clone(),
        Some(params.state_filter.clone()),
        params.maybe_deadline_filter.clone(),
        params.maybe_tags_filter.clone(),
    )?;

    let mut tasks = store.tasks(&project_filter, &task_filter)?;
    let sort_spec = SortSpec::<TaskField>::from_str(&params.sort)?;
    tasks = sort_spec.sort(tasks);
    Ok(tasks)
}

fn build_log_filter(
    task_filter: &FilterSpec<TaskFilter>,
    maybe_task_ids: Option<String>,
    maybe_start: Option<String>,
    maybe_duration: Option<String>,
    maybe_tags: Option<String>,
) -> Result<FilterSpec<LogFilter>, Error> {
    let mut filter = FilterSpec::new(LogFilter::All);
    if let Some(task_ids) = maybe_task_ids {
        let task_ids = parse_comma_separated(Some(task_ids.clone()))
            .into_iter()
            .map(|task_id| TaskId::from_str(&task_id))
            .collect::<Result<Vec<TaskId>, std::num::ParseIntError>>()
            .map_err(|e| Error::InvalidTaskIds(task_ids, e))?;
        filter = filter.and_then(LogFilter::Task(task_ids));
    } else if !task_filter.is_passthrough() {
        filter = filter.and_then(LogFilter::HasTask);
    }
    if let Some(start) = maybe_start {
        filter = filter.and_then(LogFilter::Start(TimestampFilter::from_str(&start)?));
    }
    if let Some(duration) = maybe_duration {
        filter = filter.and_then(LogFilter::Duration(DurationFilter::from_str(&duration)?));
    }
    if let Some(tags) = maybe_tags {
        filter = filter.and_then(LogFilter::Tags(parse_comma_separated(Some(tags))));
    }
    Ok(filter)
}

/// List work logs, filtered and ordered by the given parameters.
pub fn list_logs(store: &Store, params: &ListLogs) -> Result<Vec<Log>, Error> {
    let project_filter = build_project_filter(
        params.maybe_project_ids.clone(),
        params.maybe_project_deadline_filter.clone(),
        params.maybe_project_tags_filter.clone(),
    )?;
    let task_filter = build_task_filter(
        params.maybe_task_priority_filter.clone(),
        params.maybe_task_state_filter.clone(),
        params.maybe_task_deadline_filter.clone(),
        params.maybe_task_tags_filter.clone(),
    )?;
    let log_filter = build_log_filter(
        &task_filter,
        params.maybe_task_ids_filter.clone(),
        Some(params.start_filter.clone()),
        params.maybe_duration_filter.clone(),
        params.maybe_tags_filter.clone(),
    )?;

    let mut logs = store.logs(&project_filter, &task_filter, &log_filter, params.detailed)?;
    let sort_spec = SortSpec::<LogField>::from_str(&params.sort)?;
    logs = sort_spec.sort(logs);
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

/// Initialize the Loiter root path as a Git repository.
pub fn remote_init(store: &Store, params: &RemoteInit) -> Result<PathBuf, Error> {
    let store_path = store.path();
    let git_path = store_path.join(".git");
    if is_dir(&git_path) {
        return Err(Error::RemoteAlreadyInitialized(store_path));
    }
    ensure_gitignore(&store_path)?;

    git_init(&store_path, params.branch.as_str(), params.origin.as_str())?;
    if git_add(&store_path)? {
        git_commit(&store_path)?;
    }
    // Push and set upstream
    git(
        &store_path,
        ["push", "-u", "origin", params.branch.as_str()],
    )?;
    Ok(store_path)
}

pub fn remote_push(store: &Store) -> Result<PathBuf, Error> {
    let store_path = store.path();
    let git_path = store_path.join(".git");
    if !is_dir(&git_path) {
        return Err(Error::NotRemote(store_path));
    }
    if git_add(&store_path)? {
        git_commit(&store_path)?;
    }
    git_push(&store_path)?;
    Ok(store_path)
}

pub fn remote_pull(store: &Store) -> Result<PathBuf, Error> {
    let store_path = store.path();
    let git_path = store_path.join(".git");
    if !is_dir(&git_path) {
        return Err(Error::NotRemote(store_path));
    }
    git_pull(&store_path)?;
    Ok(store_path)
}

fn git_init(path: &Path, branch: &str, origin: &str) -> Result<(), Error> {
    let _ = git(path, ["init", "-b", branch])?;
    let _ = git(path, ["remote", "add", "origin", origin])?;
    Ok(())
}

fn git_add(path: &Path) -> Result<bool, Error> {
    let (_, stdout, _) = git(path, ["add", "."])?;
    Ok(!stdout.contains("nothing to commit"))
}

fn git_commit(path: &Path) -> Result<(), Error> {
    git(path, ["commit", "-m", "Loiter stash"])?;
    Ok(())
}

fn git_push(path: &Path) -> Result<(), Error> {
    git(path, ["push", "origin"])?;
    Ok(())
}

fn git_pull(path: &Path) -> Result<(), Error> {
    git(path, ["pull", "origin"])?;
    Ok(())
}

fn git<I, S>(path: &Path, args: I) -> Result<(ExitStatus, String, String), Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    subprocess(Command::new("git").current_dir(path).args(args))
}

fn ensure_gitignore(path: &Path) -> Result<(), Error> {
    let gitignore_path = path.join(".gitignore");
    if is_file(&gitignore_path) {
        debug!(
            "{} already exists - not overwriting",
            gitignore_path.display()
        );
        return Ok(());
    }
    std::fs::write(&gitignore_path, GITIGNORE)?;
    debug!("Wrote default .gitignore to {}", gitignore_path.display());
    Ok(())
}

fn subprocess(cmd: &mut Command) -> Result<(ExitStatus, String, String), Error> {
    let result = cmd.output()?;
    let stdout = match String::from_utf8(result.stdout) {
        Ok(s) => s,
        Err(e) => format!("Failed to convert stdout data to string: {:?}", e),
    };
    let stderr = match String::from_utf8(result.stderr) {
        Ok(s) => s,
        Err(e) => format!("Failed to convert stderr data to string: {:?}", e),
    };
    debug!("stdout:\n{}", stdout);
    debug!("stderr:\n{}", stderr);
    Ok((result.status, stdout, stderr))
}

fn parse_comma_separated(maybe_str: Option<String>) -> Vec<String> {
    maybe_str
        .map(|s| {
            s.split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<String>>()
        })
        .unwrap_or_else(Vec::new)
}
