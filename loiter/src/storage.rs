//! File system storage management for Loiter.
//!
//! Each object is stored as its own file in the file system. The following
//! directory structure is used to organize data:
//!
//! ```ignore
//! |_ state.json          - Current global time tracking state
//! |_ project1/           - All files relating to "Project 1"
//! |  |_ project.json     - The project's metadata.
//! |  |_ logs/            - Work logs related to "Project 1".
//! |  |  |_ 00001.json    - Work log 1 for "Project 1" (no task).
//! |  |_ tasks/           - Tasks related to "Project 1".
//! |     |_ 0001/         - Task 1 of "Project 1".
//! |     |  |_ task.json  - Task 1's metadata.
//! |     |  |_ 00001.json - Work log 1 for task 0 of "Project 1".
//! |     |  |_ 00002.json - Work log 2 for task 0 of "Project 1".
//! |     |
//! |     |_ 0002/         - Task 2 of "Project 1".
//! |        |_ task.json  - Task 2's metadata.
//! |
//! |_ project2/
//! |_ some-other-project/
//! ```

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use log::{debug, info};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::strings::slugify;
use crate::{
    Config, Duration, Error, Log, LogField, LogId, Order, Project, ProjectField, State, Task,
    TaskField, TaskId, Timestamp,
};

const STARTING_TASK_ID: TaskId = 1;
const STARTING_LOG_ID: LogId = 1;

/// A file system-based data store for Loiter.
///
/// This struct provides a minimal interface for retrieving and
/// creating/updating objects in the store.
#[derive(Debug)]
pub struct Store {
    // Absolute path to the root of the store.
    path: PathBuf,
}

impl Store {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref();
        ensure_dir_exists(path)?;
        Ok(Self {
            path: path.canonicalize()?,
        })
    }

    fn state_path(&self) -> PathBuf {
        self.path.join("state.json")
    }

    /// Get the current Loiter state.
    ///
    /// If no state file exists, one will be created.
    pub fn state(&self) -> Result<State, Error> {
        let state_path = self.state_path();
        if !is_file(&state_path) {
            let state = State::default();
            save_to_json_file(&state_path, &state)?;
            Ok(state)
        } else {
            load_from_json_file(self.state_path())
        }
    }

    /// Save the current global time tracking state.
    pub fn save_state(&self, state: &State) -> Result<(), Error> {
        save_to_json_file(self.state_path(), state)
    }

    fn config_path(&self) -> PathBuf {
        self.path.join("config.json")
    }

    /// Get the current Loiter configuration.
    ///
    /// If no configuration file exists, one will be created with default
    /// values.
    pub fn config(&self) -> Result<Config, Error> {
        let config_path = self.config_path();
        if !is_file(&config_path) {
            let config = Config::default();
            save_to_json_file(&config_path, &config)?;
            Ok(config)
        } else {
            load_from_json_file(&config_path)
        }
    }

    /// Save the given configuration to our configuration file.
    pub fn save_config(&self, config: &Config) -> Result<(), Error> {
        save_to_json_file(self.config_path(), &config)
    }

    fn project_path(&self, id: &str) -> PathBuf {
        self.path.join(id)
    }

    fn project_meta_path(&self, id: &str) -> PathBuf {
        self.project_path(id).join("project.json")
    }

    /// Get a list of all of the projects in the store.
    pub fn projects(&self, sort_by: ProjectField, order: Order) -> Result<Vec<Project>, Error> {
        let mut projects = fs::read_dir(&self.path)?
            .into_iter()
            .filter_map(|r| {
                if let Ok(e) = r {
                    let path = e.path();
                    if is_dir(&path) {
                        let project_id = path.file_name().unwrap().to_str().unwrap();
                        let project_meta_path = self.project_meta_path(project_id);
                        // We're only interested in this folder if it contains a
                        // project metadata file.
                        if is_file(project_meta_path) {
                            return Some(self.project(project_id));
                        }
                    }
                }
                None
            })
            .collect::<Result<Vec<Project>, Error>>()?;
        projects.sort_by(|a, b| sort_by.sort(a, b));
        if let Order::Desc = order {
            projects.reverse();
        }
        Ok(projects)
    }

    /// Attempt to get a specific project by its ID.
    pub fn project(&self, id: &str) -> Result<Project, Error> {
        let p: Project =
            load_from_json_file(self.project_meta_path(&slugify(id))).map_err(|e| match e {
                Error::FileNotFound(_) => Error::ProjectNotFound(id.to_string()),
                e => e,
            })?;
        // Ensure that the ID is updated
        let project_name = p.name().to_string();
        Ok(p.with_name(&project_name))
    }

    /// Create or update a project.
    pub fn save_project(&self, project: &Project) -> Result<(), Error> {
        save_to_json_file(self.project_meta_path(project.id()), &project)
    }

    /// Attempts to rename the given project from its old ID to the one
    /// supplied.
    pub fn rename_project<S: AsRef<str>>(&self, old_id: S, project: &Project) -> Result<(), Error> {
        let old_path = self.project_path(old_id.as_ref());
        if !is_dir(&old_path) {
            return Err(Error::ProjectNotFound(old_id.as_ref().to_string()));
        }
        let new_path = self.project_path(project.id());
        if is_dir(&new_path) {
            return Err(Error::ProjectAlreadyExists(project.id().to_string()));
        }
        fs::rename(&old_path, &new_path)?;
        debug!(
            "Renamed directory {} to {}",
            old_path.display(),
            new_path.display()
        );
        self.save_project(project)
    }

    fn tasks_path(&self, project_id: &str) -> PathBuf {
        self.project_path(project_id).join("tasks")
    }

    /// Get all of the tasks for the project with the specified ID.
    pub fn tasks(
        &self,
        project_id: &str,
        sort_by: TaskField,
        order: Order,
    ) -> Result<Vec<Task>, Error> {
        let tasks_path = self.tasks_path(project_id);
        if !is_dir(&tasks_path) {
            return Ok(Vec::new());
        }
        let mut tasks = fs::read_dir(&tasks_path)?
            .into_iter()
            .filter_map(|r| {
                if let Ok(e) = r {
                    let path = tasks_path.join(e.path());
                    if is_dir(&path) {
                        let task_id = match task_id_from_path(&path) {
                            Ok(task_id) => task_id,
                            Err(_) => return None,
                        };
                        return Some(self.task(project_id, task_id));
                    }
                }
                None
            })
            .collect::<Result<Vec<Task>, Error>>()?;
        tasks.sort_by(|a, b| sort_by.sort(a, b));
        if let Order::Desc = order {
            tasks.reverse();
        }
        Ok(tasks)
    }

    fn task_path(&self, project_id: &str, task_id: TaskId) -> PathBuf {
        self.tasks_path(project_id).join(format!("{:04}", task_id))
    }

    fn task_meta_path(&self, project_id: &str, task_id: TaskId) -> PathBuf {
        self.task_path(project_id, task_id).join("task.json")
    }

    /// Attempt to get a task by its ID and its project's ID.
    pub fn task(&self, project_id: &str, task_id: TaskId) -> Result<Task, Error> {
        let task_path = self.task_path(project_id, task_id);
        let task_meta_path = self.task_meta_path(project_id, task_id);
        if !is_dir(&task_path) || !is_file(&task_meta_path) {
            return Err(Error::TaskNotFound(project_id.to_string(), task_id));
        }
        Ok(load_from_json_file::<&PathBuf, Task>(&task_meta_path)?
            .with_project_id(project_id)
            .with_id(task_id))
    }

    fn next_task_id(&self, project_id: &str) -> Result<TaskId, Error> {
        Ok(self
            .tasks(project_id, TaskField::default(), Order::default())?
            .into_iter()
            .map(|task| task.id().unwrap())
            .max()
            .map(|highest_task_id| highest_task_id + 1)
            .unwrap_or(STARTING_TASK_ID))
    }

    /// Create or update a task.
    pub fn save_task(&self, task: &Task) -> Result<Task, Error> {
        let project_id = task
            .project_id()
            .ok_or_else(|| Error::TaskMissingProjectId(task.clone()))?;
        let project = self.project(project_id)?;
        let config = self.config()?;
        let task_state_config = project
            .task_state_config()
            .unwrap_or_else(|| config.task_state_config());
        let state = task_state_config.validate_or_initial(task.state())?;
        let task_id = match task.id() {
            Some(id) => id,
            None => self.next_task_id(project_id)?,
        };
        let task = task.clone().with_id(task_id).with_state(state);
        let task_path = self.task_path(project_id, task_id);
        ensure_dir_exists(&task_path)?;

        let task_meta_path = self.task_meta_path(project_id, task_id);
        save_to_json_file(&task_meta_path, &task)?;
        Ok(task)
    }

    fn logs_path(&self, project_id: &str, maybe_task_id: Option<TaskId>) -> PathBuf {
        match maybe_task_id {
            Some(task_id) => self.task_path(project_id, task_id),
            None => self.project_path(project_id).join("logs"),
        }
    }

    fn log_path(&self, project_id: &str, maybe_task_id: Option<TaskId>, id: LogId) -> PathBuf {
        self.logs_path(project_id, maybe_task_id)
            .join(format!("{:05}.json", id))
    }

    fn next_log_id(&self, project_id: &str, maybe_task_id: Option<TaskId>) -> Result<LogId, Error> {
        Ok(self
            .logs(
                project_id,
                maybe_task_id,
                LogField::default(),
                Order::default(),
            )?
            .into_iter()
            .map(|log| log.id().unwrap())
            .max()
            .map(|highest_log_id| highest_log_id + 1)
            .unwrap_or(STARTING_LOG_ID))
    }

    /// Get all of the logs associated with the given project, and optionally
    /// with the given task.
    pub fn logs(
        &self,
        project_id: &str,
        maybe_task_id: Option<TaskId>,
        sort_by: LogField,
        order: Order,
    ) -> Result<Vec<Log>, Error> {
        let logs_path = self.logs_path(project_id, maybe_task_id);
        if !is_dir(&logs_path) {
            return Ok(Vec::new());
        }
        let mut logs = fs::read_dir(&logs_path)?
            .into_iter()
            .filter_map(|r| {
                if let Ok(e) = r {
                    let path = logs_path.join(e.path());
                    if is_file(&path) {
                        let log_id = match log_id_from_path(&path) {
                            Ok(log_id) => log_id,
                            Err(_) => return None,
                        };
                        return Some(self.log(project_id, maybe_task_id, log_id));
                    }
                }
                None
            })
            .collect::<Result<Vec<Log>, Error>>()?;
        logs.sort_by(|a, b| sort_by.sort(a, b));
        if let Order::Desc = order {
            logs.reverse();
        }
        Ok(logs)
    }

    /// Attempt to get the specific log belonging to the given project, and
    /// optionally the given task, with the specified log ID.
    pub fn log(
        &self,
        project_id: &str,
        maybe_task_id: Option<TaskId>,
        id: LogId,
    ) -> Result<Log, Error> {
        let log_path = self.log_path(project_id, maybe_task_id, id);
        if !is_file(&log_path) {
            return Err(Error::LogNotFound(
                project_id.to_string(),
                maybe_task_id,
                id,
            ));
        }
        Ok(load_from_json_file::<&PathBuf, Log>(&log_path)?
            .with_id(id)
            .with_project_id(project_id)
            .with_maybe_task_id(maybe_task_id))
    }

    /// Create or update a work log.
    pub fn save_log(&self, log: &Log) -> Result<Log, Error> {
        let project_id = log
            .project_id()
            .ok_or_else(|| Error::LogMissingProjectId(log.clone()))?;
        let project_path = self.project_path(project_id);
        if !is_dir(&project_path) {
            return Err(Error::ProjectNotFound(project_id.to_string()));
        }
        if let Some(task_id) = log.task_id() {
            let task_path = self.task_path(project_id, task_id);
            if !is_dir(&task_path) {
                return Err(Error::TaskNotFound(project_id.to_string(), task_id));
            }
        }
        let log_id = match log.id() {
            Some(id) => id,
            None => self.next_log_id(project_id, log.task_id())?,
        };
        let log_path = self.log_path(project_id, log.task_id(), log_id);
        let log = log.clone().with_id(log_id);
        save_to_json_file(&log_path, &log)?;
        Ok(log)
    }

    /// Start the given work log.
    pub fn start_log(&self, log: &Log) -> Result<Log, Error> {
        let state = self.state()?;
        let now = Timestamp::now()?;
        // Stop any other log that's already active
        if let Some((project_id, maybe_task_id, log_id)) = state.active_log() {
            info!(
                "Stopping active log {} for project {}{}",
                log_id,
                project_id,
                maybe_task_id
                    .map(|task_id| format!(", task {}", task_id))
                    .unwrap_or_else(|| "".to_string())
            );
            let active_log = self
                .log(&project_id, maybe_task_id, log_id)?
                .with_stop(now)?;
            let _ = self.save_log(&active_log)?;
        }
        // Give it a start time if it doesn't have one yet
        let log = log.clone().with_maybe_start(log.start().or(Some(now)));
        self.save_log(&log)
    }

    /// Stop the currently active log, if any.
    ///
    /// If no stop time or duration is provided, the current local time will be
    /// used as the stop time to determine the duration of the log.
    pub fn stop_log(
        &self,
        maybe_stop_time: Option<Timestamp>,
        maybe_duration: Option<Duration>,
        maybe_comment: Option<String>,
        tags: Vec<String>,
    ) -> Result<Log, Error> {
        let state = self.state()?;
        let (project_id, maybe_task_id, log_id) = state.active_log().ok_or(Error::NoActiveLog)?;
        let now = Timestamp::now()?;
        let mut active_log = self
            .log(&project_id, maybe_task_id, log_id)?
            .with_maybe_duration_or_stop(maybe_duration, maybe_stop_time.or(Some(now)))?;
        if let Some(comment) = maybe_comment {
            active_log = active_log.with_comment(&comment);
        }
        if !tags.is_empty() {
            active_log = active_log.with_tags(tags)?;
        }
        self.save_log(&active_log)
    }
}

fn load_from_json_file<P, O>(path: P) -> Result<O, Error>
where
    P: AsRef<Path>,
    O: DeserializeOwned,
{
    let path = path.as_ref();
    let meta = fs::metadata(path);
    if meta.is_err() || !meta?.is_file() {
        return Err(Error::FileNotFound(path.to_path_buf()));
    }
    let s = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&s)?)
}

fn save_to_json_file<P, O>(path: P, obj: &O) -> Result<(), Error>
where
    P: AsRef<Path>,
    O: Serialize,
{
    let path = path.as_ref();
    let parent_path = path
        .parent()
        .ok_or_else(|| Error::InvalidPath(path.to_path_buf()))?;
    // Ensure the parent path exists
    if !is_dir(&parent_path) {
        fs::create_dir_all(parent_path)?;
        debug!("Created path: {}", parent_path.display());
    }
    let s = serde_json::to_string_pretty(obj)?;
    Ok(fs::write(path, &s)?)
}

fn is_file<P: AsRef<Path>>(path: P) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_file())
        .unwrap_or(false)
}

fn is_dir<P: AsRef<Path>>(path: P) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_dir())
        .unwrap_or(false)
}

fn task_id_from_path<P: AsRef<Path>>(path: P) -> Result<TaskId, Error> {
    let path = path.as_ref();
    let dir_name = path
        .file_name()
        .map(OsStr::to_str)
        .flatten()
        .ok_or_else(|| Error::InvalidPath(path.to_path_buf()))?;
    TaskId::from_str(dir_name).map_err(|e| Error::InvalidTaskNumber(path.to_path_buf(), e))
}

fn log_id_from_path<P: AsRef<Path>>(path: P) -> Result<LogId, Error> {
    let path = path.as_ref();
    let file_name = path
        .file_stem()
        .map(OsStr::to_str)
        .flatten()
        .ok_or_else(|| Error::InvalidPath(path.to_path_buf()))?;
    LogId::from_str(file_name).map_err(|e| Error::InvalidLogNumber(path.to_path_buf(), e))
}

fn ensure_dir_exists<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    let path = path.as_ref();
    if !is_dir(path) {
        fs::create_dir_all(path)?;
        debug!("Created path: {}", path.display());
    }
    Ok(())
}