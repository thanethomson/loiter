//! User-oriented functionality for interacting with Loiter stores.

use crate::{
    Duration, Error, Log, LogField, Order, Project, ProjectField, ProjectId, SortSpec, Store, Task,
    TaskField, TaskId, TaskState, Timestamp,
};
use comfy_table::{presets, Table};
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

impl TryFrom<AddProject> for Project {
    type Error = Error;

    fn try_from(cmd: AddProject) -> Result<Self, Self::Error> {
        Project::new(&cmd.name)
            .with_maybe_description(cmd.maybe_description)
            .with_maybe_deadline(cmd.maybe_deadline)
            .with_tags(parse_tags(cmd.maybe_tags))
    }
}

/// Add a new task for a project.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct AddTask {
    /// The ID of the project to which this task belongs.
    project_id: ProjectId,

    /// A short, human-readable description of what this task is about.
    description: String,

    /// The desired state of the task once added.
    #[structopt(name = "state", short, long)]
    #[serde(rename = "state")]
    maybe_state: Option<TaskState>,

    /// An optional deadline for this project.
    #[structopt(name = "deadline", short, long)]
    #[serde(rename = "deadline")]
    maybe_deadline: Option<Timestamp>,

    /// Tags to associate with this task, separated by commas (e.g.
    /// "work,engineering,ux").
    #[structopt(name = "tags", long)]
    #[serde(rename = "tags")]
    maybe_tags: Option<String>,
}

impl TryFrom<AddTask> for Task {
    type Error = Error;

    fn try_from(cmd: AddTask) -> Result<Self, Self::Error> {
        Task::new(&cmd.project_id, &cmd.description)
            .with_maybe_state(cmd.maybe_state)
            .with_maybe_deadline(cmd.maybe_deadline)
            .with_tags(parse_tags(cmd.maybe_tags))
    }
}

/// Update a specific task.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct UpdateTask {
    /// The ID of the project whose task must be updated.
    project_id: ProjectId,

    /// The ID of the task to update.
    task_id: TaskId,

    /// Update the task description.
    #[structopt(name = "description", short, long)]
    #[serde(rename = "description")]
    maybe_description: Option<String>,

    /// Update the state of the task.
    #[structopt(name = "state", short, long)]
    #[serde(rename = "state")]
    maybe_state: Option<TaskState>,

    /// Update the deadline for the task.
    #[structopt(name = "deadline", long)]
    #[serde(rename = "deadline")]
    maybe_deadline: Option<Timestamp>,

    /// Update the task's tags.
    #[structopt(name = "tags", long)]
    #[serde(rename = "tags")]
    maybe_tags: Option<String>,
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
    project_id: ProjectId,

    /// Optionally, the ID of the task to which this work log relates.
    #[structopt(name = "task", short, long)]
    #[serde(rename = "task_id")]
    maybe_task_id: Option<TaskId>,

    /// Optionally specify the start time for this log.
    #[structopt(name = "from", long)]
    #[serde(rename = "start")]
    maybe_start: Option<Timestamp>,

    /// Optionally specify the stop time for this log (cannot be used with
    /// duration).
    #[structopt(name = "to", long)]
    #[serde(rename = "stop")]
    maybe_stop: Option<Timestamp>,

    /// Optionally specify the duration of this log, in seconds (cannot be
    /// used with stop time).
    #[structopt(name = "duration", short, long)]
    #[serde(rename = "duration")]
    maybe_duration: Option<Duration>,

    /// An optional comment of what is/was being done in this work log.
    #[structopt(name = "comment", short, long)]
    #[serde(rename = "comment")]
    maybe_comment: Option<String>,

    /// Tags to associate with this work log, separated by commas (e.g.
    /// "work,coding").
    #[structopt(name = "tags", long)]
    #[serde(rename = "tags")]
    maybe_tags: Option<String>,
}

impl TryFrom<AddLog> for Log {
    type Error = Error;

    fn try_from(cmd: AddLog) -> Result<Self, Self::Error> {
        Log::new(&cmd.project_id)
            .with_maybe_task_id(cmd.maybe_task_id)
            .with_maybe_start(cmd.maybe_start)
            .with_maybe_duration_or_stop(cmd.maybe_duration, cmd.maybe_stop)?
            .with_maybe_comment(cmd.maybe_comment)
            .with_tags(parse_tags(cmd.maybe_tags))
    }
}

/// Start a new work log for a project or task.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct StartLog {
    /// The ID of the project to which to add this work log.
    #[structopt(name = "project")]
    project_id: ProjectId,

    /// Optionally, the ID of the task to which this work log relates.
    #[structopt(name = "task", long)]
    #[serde(rename = "task_id")]
    maybe_task_id: Option<TaskId>,

    /// Optionally, the date/time from which to start this log. Defaults to the
    /// current local time.
    #[structopt(name = "from", short, long, default_value = "now")]
    start: Timestamp,

    /// An optional comment describing what is/was being done in this work log.
    #[structopt(name = "comment", short, long)]
    maybe_comment: Option<String>,

    /// Tags to associate with this work log, separated by commas (e.g.
    /// "work,coding").
    #[structopt(name = "tags", long)]
    maybe_tags: Option<String>,
}

impl TryFrom<StartLog> for Log {
    type Error = Error;

    fn try_from(cmd: StartLog) -> Result<Self, Self::Error> {
        Log::new(&cmd.project_id)
            .with_maybe_task_id(cmd.maybe_task_id)
            .with_start(cmd.start)
            .with_maybe_comment(cmd.maybe_comment)
            .with_tags(parse_tags(cmd.maybe_tags))
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
    maybe_stop_time: Option<Timestamp>,

    #[structopt(name = "duration", short, long)]
    #[serde(rename = "duration")]
    maybe_duration: Option<Duration>,

    /// An optional comment of what was done in this work log.
    #[structopt(name = "comment", short, long)]
    #[serde(rename = "comment")]
    maybe_comment: Option<String>,

    /// Tags to associate with this work log, separated by commas (e.g.
    /// "work,coding").
    #[structopt(name = "tags", long)]
    #[serde(rename = "tags")]
    maybe_tags: Option<String>,
}

/// List all projects.
#[derive(Debug, Clone, Default, StructOpt, Serialize, Deserialize)]
pub struct ListProjects {
    /// Show project details as opposed to just project names and IDs.
    #[structopt(short, long)]
    detailed: bool,

    /// Optionally sort the projects by specific fields (e.g. "name" will sort
    /// projects in ascending order by name; "name:desc" will sort by name in
    /// descending order; "deadline,name" will first sort by deadline and then
    /// by name).
    #[structopt(short, long)]
    sort: Option<String>,
}

/// List all of the tasks for a project.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct ListTasks {
    /// The ID of the project whose tasks must be listed.
    #[structopt(name = "project")]
    project_id: ProjectId,

    /// Show task details as opposed to just task IDs and descriptions.
    #[structopt(short, long)]
    detailed: bool,

    /// Sort tasks by the given field.
    #[structopt(short, long, default_value)]
    sort_by: TaskField,

    /// Sort order ("asc" for ascending order, or "desc" for descending order).
    #[structopt(short, long, default_value)]
    order: Order,
}

/// List all of the logs for a project or task.
#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct ListLogs {
    /// The ID of the project whose logs must be listed.
    #[structopt(name = "project")]
    project_id: ProjectId,

    /// The ID of the task whose logs must be listed (if applicable).
    #[structopt(name = "task", short, long)]
    maybe_task_id: Option<TaskId>,

    /// Show logs' details as opposed to summary.
    #[structopt(short, long)]
    detailed: bool,

    /// Sort logs by the given field.
    #[structopt(short, long, default_value)]
    sort_by: LogField,

    /// Sort order ("asc" for ascending order, or "desc" for descending order).
    #[structopt(short, long, default_value)]
    order: Order,
}

#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
pub struct TaskStates {
    #[structopt(name = "project", short, long)]
    maybe_project_id: Option<ProjectId>,
}

/// Add a new project to the given store.
pub fn add_project(store: &Store, params: AddProject) -> Result<Project, Error> {
    let project = Project::try_from(params)?;
    store.save_project(&project)?;
    debug!("Created new project {}", project.name());
    Ok(project)
}

/// Add a new task for a specific project to the store.
pub fn add_task(store: &Store, params: AddTask) -> Result<Task, Error> {
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
pub fn update_task(store: &Store, params: UpdateTask) -> Result<Task, Error> {
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
pub fn add_log(store: &Store, params: AddLog) -> Result<Log, Error> {
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
pub fn start_log(store: &Store, params: StartLog) -> Result<Log, Error> {
    let state = store.state()?;
    // Stop any active log
    if state.active_log().is_some() {
        let _ = stop_log(store, StopLog::default())?;
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
pub fn stop_log(store: &Store, params: StopLog) -> Result<Log, Error> {
    let state = store.state()?;
    let mut active_log = match state.active_log() {
        Some((project_id, maybe_task_id, log_id)) => {
            store.log(&project_id, maybe_task_id, log_id)?
        }
        None => return Err(Error::NoActiveLog),
    }
    .with_duration_or_stop_or_now(params.maybe_duration, params.maybe_stop_time)?;

    // Optionally update the comment and tags
    if let Some(comment) = params.maybe_comment {
        active_log = active_log.with_comment(comment);
    }
    if let Some(tags) = params.maybe_tags {
        active_log = active_log.with_tags(parse_tags(Some(tags)))?;
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
pub fn cancel_log(store: &Store) -> Result<(), Error> {
    let state = store.state()?;
    let active_log = match state.active_log() {
        Some((project_id, maybe_task_id, log_id)) => {
            store.log(&project_id, maybe_task_id, log_id)?
        }
        None => return Err(Error::NoActiveLog),
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
    Ok(())
}

/// List projects, optionally sorting them.
///
/// Returns the rendered table containing the results.
pub fn list_projects(store: &Store, params: ListProjects) -> Result<String, Error> {
    let mut projects = store.projects()?;
    if let Some(sort) = params.sort {
        let sort_spec = SortSpec::<ProjectField>::from_str(&sort)?;
        projects = sort_spec.sort(projects);
    }
    let mut table = Table::new();
    let header = if params.detailed {
        vec!["ID", "Name", "Description", "Deadline", "Tags"]
    } else {
        vec!["ID", "Name", "Tags"]
    };
    table.load_preset(presets::ASCII_FULL).set_header(header);
    for project in projects.iter() {
        let deadline = display_optional(project.deadline());
        let tags = project.tags().collect::<Vec<&str>>().join(", ");
        let row = if params.detailed {
            vec![
                project.id(),
                project.name(),
                project.description().unwrap_or(""),
                &deadline,
                &tags,
            ]
        } else {
            vec![project.id(), project.name(), &tags]
        };
        table.add_row(row);
    }
    Ok(table.to_string())
}

/// List tasks for a particular project, optionally sorting them.
///
/// Returns the rendered table containing the results.
pub fn list_tasks(store: &Store, params: ListTasks) -> Result<String, Error> {
    let tasks = store.tasks(&params.project_id)?;
    let mut table = Table::new();
    table.load_preset(presets::ASCII_FULL).set_header(vec![
        "ID",
        "Project ID",
        "State",
        "Description",
        "Deadline",
        "Tags",
    ]);
    for task in tasks.iter() {
        table.add_row(vec![
            task.id().unwrap().to_string().as_str(),
            task.project_id().unwrap(),
            task.state().unwrap_or(""),
            task.description(),
            &task
                .deadline()
                .map(|d| d.to_string())
                .unwrap_or_else(|| "".to_string()),
            &task.tags().collect::<Vec<&str>>().join(", "),
        ]);
    }
    Ok(table.to_string())
}

/// List work logs, filtered and ordered by the given parameters.
pub fn list_logs(store: &Store, params: ListLogs) -> Result<String, Error> {
    let logs = store.logs(&params.project_id, params.maybe_task_id)?;
    let mut table = Table::new();
    table
        .load_preset(presets::ASCII_FULL)
        .set_header(vec!["ID", "Project", "Task", "Start", "Duration", "Tags"]);

    for log in logs.iter() {
        table.add_row(vec![
            log.id().unwrap().to_string().as_str(),
            log.project_id().unwrap(),
            &log.task_id()
                .map(|i| i.to_string())
                .unwrap_or_else(|| "".to_string()),
            &log.start()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "".to_string()),
            &log.duration()
                .map(|d| d.to_string())
                .unwrap_or_else(|| "".to_string()),
            &log.tags().collect::<Vec<&str>>().join(", "),
        ]);
    }

    Ok(table.to_string())
}

/// Shows the status of the currently active log.
pub fn status(store: &Store) -> Result<(), Error> {
    let state = store.state()?;
    match state.active_log() {
        Some((project_id, maybe_task_id, log_id)) => {
            let active_log = store.log(&project_id, maybe_task_id, log_id)?;
            let start = active_log.start().unwrap();
            let duration = Timestamp::now()? - start;
            debug!(
                "Log {} from project {}{} has been active since {} ({})",
                active_log.id().unwrap(),
                active_log.project_id().unwrap(),
                active_log
                    .task_id()
                    .map(|task_id| format!(", task {},", task_id))
                    .unwrap_or_else(|| "".to_string()),
                start,
                duration,
            );
        }
        None => debug!("No active log"),
    }
    Ok(())
}

/// Shows a list of task states. If no project is supplied, the default
/// configuration will be shown.
pub fn task_states(store: &Store, params: TaskStates) -> Result<String, Error> {
    let config = store.config()?;
    let default_tsc = config.task_state_config();
    let maybe_project = match params.maybe_project_id {
        Some(id) => Some(store.project(&id)?),
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
    let mut table = Table::new();
    table.load_preset(presets::ASCII_FULL);
    for state in states.iter() {
        table.add_row(vec![state]);
    }
    Ok(table.to_string())
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

fn display_optional<S: std::fmt::Display>(maybe_value: Option<S>) -> String {
    maybe_value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "".to_string())
}
