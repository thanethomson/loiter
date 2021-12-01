use std::error::Error;
use std::{convert::Infallible, path::PathBuf, str::FromStr};

use log::Level;
use loiter::{
    Duration, Log, LogField, Project, ProjectField, ProjectId, SortDir, Store, Task, TaskField,
    TaskId, TaskState, Timestamp,
};
use structopt::StructOpt;

// Defaults to ~/.loiter
#[derive(Debug, Clone)]
struct StorePath(PathBuf);

impl Default for StorePath {
    fn default() -> Self {
        Self(home::home_dir().unwrap().join(".loiter"))
    }
}

impl FromStr for StorePath {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(PathBuf::from_str(s)?))
    }
}

impl std::fmt::Display for StorePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "loiter")]
struct Opt {
    /// Where to find your local Loiter data store. Defaults to `~/.loiter`.
    #[structopt(short, long, default_value)]
    path: StorePath,
    /// Increase output logging verbosity to DEBUG level.
    #[structopt(short, long)]
    verbose: bool,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Add a project, task or work log.
    Add(AddCommand),
    /// Start a work log.
    Start(StartCommand),
    /// Stop the currently active work log.
    Stop(StopCommand),
    //    Find,
    /// List projects, tasks or work logs.
    List(ListCommand),
    //    Remove,
    //    Update,
}

#[derive(Debug, StructOpt)]
enum AddCommand {
    /// Add a new project.
    Project(AddProjectCommand),
    /// Add a task to a project.
    Task(AddTaskCommand),
    /// Add a work log for a task or project.
    Log(AddLogCommand),
}

#[derive(Debug, StructOpt)]
struct AddProjectCommand {
    /// The name of the project to add. Its ID will automatically be
    /// generated from this name.
    #[structopt(short, long)]
    name: String,
    /// An optional description of the project.
    #[structopt(name = "description", short, long)]
    maybe_description: Option<String>,
    /// An optional deadline for this project.
    #[structopt(name = "deadline", short = "e", long)]
    maybe_deadline: Option<Timestamp>,
    /// Tags to associate with this project, separated by commas (e.g.
    /// "work,engineering,ux").
    #[structopt(name = "tags", long)]
    maybe_tags: Option<String>,
}

#[derive(Debug, StructOpt)]
struct AddTaskCommand {
    #[structopt(short, long)]
    /// The ID of the project to which this task belongs.
    project_id: String,
    #[structopt(short, long)]
    /// A short, human-readable description of what this task is about.
    description: String,
    /// The desired state of the task once added.
    #[structopt(name = "state", short, long)]
    maybe_state: Option<TaskState>,
    /// An optional deadline for this project.
    #[structopt(name = "deadline", short = "e", long)]
    maybe_deadline: Option<Timestamp>,
    /// Tags to associate with this task, separated by commas (e.g.
    /// "work,engineering,ux").
    #[structopt(name = "tags", long)]
    maybe_tags: Option<String>,
}

#[derive(Debug, StructOpt)]
struct AddLogCommand {
    #[structopt(short, long)]
    /// The ID of the project to which to add this work log.
    project_id: String,
    /// Optionally, the ID of the task to which this work log relates.
    #[structopt(name = "task-id", short = "i", long)]
    maybe_task_id: Option<TaskId>,
    /// Optionally specify the start time for this log.
    #[structopt(name = "from", short, long)]
    maybe_start: Option<Timestamp>,
    /// Optionally specify the stop time for this log (cannot be used with
    /// duration).
    #[structopt(name = "to", short, long)]
    maybe_stop: Option<Timestamp>,
    /// Optionally specify the duration of this log, in seconds (cannot be
    /// used with stop time).
    #[structopt(name = "duration", short, long)]
    maybe_duration: Option<Duration>,
    /// An optional comment of what is/was being done in this work log.
    #[structopt(name = "comment", short, long)]
    maybe_comment: Option<String>,
    /// Tags to associate with this work log, separated by commas (e.g.
    /// "work,coding").
    #[structopt(name = "tags", long)]
    maybe_tags: Option<String>,
}

#[derive(Debug, StructOpt)]
struct StartCommand {
    #[structopt(short, long)]
    /// The ID of the project to which to add this work log.
    project_id: String,
    /// Optionally, the ID of the task to which this work log relates.
    #[structopt(name = "task", short, long)]
    maybe_task_id: Option<TaskId>,
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

#[derive(Debug, StructOpt)]
struct StopCommand {
    /// Optionally specify the time at which the current work log should be
    /// stopped. If not given, and no duration is given, the current
    /// date/time will be used.
    #[structopt(name = "at", short, long)]
    maybe_stop_time: Option<Timestamp>,
    #[structopt(name = "duration", short, long)]
    maybe_duration: Option<Duration>,
    /// An optional comment of what was done in this work log.
    #[structopt(name = "comment", short, long)]
    maybe_comment: Option<String>,
    /// Tags to associate with this work log, separated by commas (e.g.
    /// "work,coding").
    #[structopt(name = "tags", long)]
    maybe_tags: Option<String>,
}

#[derive(Debug, StructOpt)]
enum ListCommand {
    /// List all projects.
    Projects(ListProjectsCommand),
    /// List tasks.
    Tasks(ListTasksCommand),
    /// List work logs.
    Logs(ListLogsCommand),
}

#[derive(Debug, StructOpt)]
struct ListProjectsCommand {
    /// Show project details as opposed to just project names and IDs.
    #[structopt(short, long)]
    detailed: bool,
    /// Sort projects by the given field.
    #[structopt(short, long, default_value)]
    sort_by: ProjectField,
    /// Sort direction ("asc" for ascending order, or "desc" for descending
    /// order).
    #[structopt(short = "r", long, default_value)]
    sort_dir: SortDir,
}

#[derive(Debug, StructOpt)]
struct ListTasksCommand {
    /// The ID of the project whose tasks must be listed.
    #[structopt(short, long)]
    project_id: ProjectId,
    /// Show task details as opposed to just task IDs and descriptions.
    #[structopt(short, long)]
    detailed: bool,
    /// Sort tasks by the given field.
    #[structopt(short, long, default_value)]
    sort_by: TaskField,
    /// Sort direction ("asc" for ascending order, or "desc" for descending
    /// order).
    #[structopt(short = "r", long, default_value)]
    sort_dir: SortDir,
}

#[derive(Debug, StructOpt)]
struct ListLogsCommand {
    /// The ID of the project whose logs must be listed.
    #[structopt(short, long)]
    project_id: ProjectId,
    /// The ID of the task whose logs must be listed (if applicable).
    #[structopt(name = "task-id", short, long)]
    maybe_task_id: Option<TaskId>,
    /// Show logs' details as opposed to summary.
    #[structopt(short, long)]
    detailed: bool,
    /// Sort logs by the given field.
    #[structopt(short, long, default_value)]
    sort_by: LogField,
    /// Sort direction ("asc" for ascending order, or "desc" for descending
    /// order).
    #[structopt(short = "r", long, default_value)]
    sort_dir: SortDir,
}

fn execute(opt: Opt) -> Result<(), Box<dyn Error>> {
    let store = Store::new(&opt.path.0)?;
    match opt.command {
        Command::Add(add_cmd) => add(&store, add_cmd),
        Command::Start(start_cmd) => start(&store, start_cmd),
        Command::Stop(stop_cmd) => stop(&store, stop_cmd),
        Command::List(list_cmd) => list(&store, list_cmd),
    }
}

fn add(store: &Store, cmd: AddCommand) -> Result<(), Box<dyn Error>> {
    match cmd {
        AddCommand::Project(sub_cmd) => add_project(store, sub_cmd),
        AddCommand::Task(sub_cmd) => add_task(store, sub_cmd),
        AddCommand::Log(sub_cmd) => add_log(store, sub_cmd),
    }
}

fn add_project(store: &Store, cmd: AddProjectCommand) -> Result<(), Box<dyn Error>> {
    let project = Project::new(&cmd.name)
        .with_maybe_description(cmd.maybe_description.clone())
        .with_maybe_deadline(cmd.maybe_deadline.clone())
        .with_tags(parse_tags(cmd.maybe_tags))?;
    store.save_project(&project)?;
    log::info!("Added project {}", project.name());
    Ok(())
}

fn add_task(store: &Store, cmd: AddTaskCommand) -> Result<(), Box<dyn Error>> {
    let config = store.config()?;
    let project = store.project(&cmd.project_id)?;
    let task_state = project
        .task_state_config()
        .unwrap_or_else(|| config.task_state_config())
        .validate_or_initial(cmd.maybe_state.clone())?;
    let task = Task::new(&cmd.project_id, &cmd.description, task_state)
        .with_maybe_deadline(cmd.maybe_deadline.clone())
        .with_tags(parse_tags(cmd.maybe_tags))?;
    let task = store.save_task(&task)?;
    log::info!(
        "Added task {} for project {}",
        task.id().unwrap(),
        project.name()
    );
    Ok(())
}

fn add_log(store: &Store, cmd: AddLogCommand) -> Result<(), Box<dyn Error>> {
    let work_log = Log::new(&cmd.project_id)
        .with_maybe_task_id(cmd.maybe_task_id)
        .with_maybe_start(cmd.maybe_start)
        .with_maybe_duration_or_stop(cmd.maybe_duration, cmd.maybe_stop)?
        .with_maybe_comment(cmd.maybe_comment)
        .with_tags(parse_tags(cmd.maybe_tags))?;
    let work_log = store.save_log(&work_log)?;
    log::info!(
        "Added log {} for project {}{}",
        work_log.id().unwrap(),
        work_log.project_id().unwrap(),
        work_log
            .task_id()
            .map(|task_id| format!(", task {}", task_id))
            .unwrap_or_else(|| "".to_string())
    );
    Ok(())
}

fn start(store: &Store, cmd: StartCommand) -> Result<(), Box<dyn Error>> {
    Ok(())
}

fn stop(store: &Store, cmd: StopCommand) -> Result<(), Box<dyn Error>> {
    Ok(())
}

fn list(store: &Store, cmd: ListCommand) -> Result<(), Box<dyn Error>> {
    match cmd {
        ListCommand::Projects(sub_cmd) => list_projects(store, sub_cmd),
        ListCommand::Tasks(sub_cmd) => list_tasks(store, sub_cmd),
        ListCommand::Logs(sub_cmd) => list_logs(store, sub_cmd),
    }
}

fn list_projects(store: &Store, cmd: ListProjectsCommand) -> Result<(), Box<dyn Error>> {
    let projects = store.projects(cmd.sort_by, cmd.sort_dir)?;
    for project in projects.iter() {
        println!("{} ({})", project.name(), project.id());
    }
    Ok(())
}

fn list_tasks(store: &Store, cmd: ListTasksCommand) -> Result<(), Box<dyn Error>> {
    let tasks = store.tasks(&cmd.project_id, cmd.sort_by, cmd.sort_dir)?;
    for task in tasks.iter() {
        println!(
            "{}) {} ({}{})",
            task.id().unwrap(),
            task.description(),
            task.state(),
            task.deadline()
                .map(|deadline| format!(", deadline: {}", deadline))
                .unwrap_or_else(|| "".to_string())
        );
    }
    Ok(())
}

fn list_logs(store: &Store, cmd: ListLogsCommand) -> Result<(), Box<dyn Error>> {
    Ok(())
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

fn main() {
    let opt = Opt::from_args();
    simple_logger::init_with_level(if opt.verbose {
        Level::Debug
    } else {
        Level::Info
    })
    .unwrap();

    if let Err(e) = execute(opt) {
        log::error!("{}", e);
    }
}
