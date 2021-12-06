mod display;

use std::error::Error;
use std::{convert::Infallible, path::PathBuf, str::FromStr};

use log::{error, Level};
use loiter::{cmd, Store};
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
    /// Update a project, task or work log.
    Update(UpdateCommand),
    /// Start a work log.
    Start(cmd::StartLog),
    /// Stop the currently active work log.
    Stop(cmd::StopLog),
    /// Cancel the currently active work log.
    Cancel,
    /// Show the status of the currently active work log (if any).
    Status,
    /// Show a list of valid task states.
    States(cmd::TaskStates),
    //    Find,
    /// List projects, tasks or work logs.
    List(ListCommand),
    /// Remove projects, tasks or work logs.
    Remove(RemoveCommand),
    //    Update,
}

#[derive(Debug, StructOpt)]
enum AddCommand {
    Project(cmd::AddProject),
    Task(cmd::AddTask),
    Log(cmd::AddLog),
}

#[derive(Debug, StructOpt)]
enum RemoveCommand {
    Project(cmd::RemoveProject),
}

#[derive(Debug, StructOpt)]
enum UpdateCommand {
    Task(cmd::UpdateTask),
}

#[derive(Debug, StructOpt)]
enum ListCommand {
    Projects(cmd::ListProjects),
    Tasks(cmd::ListTasks),
    Logs(cmd::ListLogs),
}

fn execute(opt: Opt) -> Result<(), Box<dyn Error>> {
    let store = Store::new(&opt.path.0)?;
    match opt.command {
        Command::Add(sub_cmd) => add(&store, sub_cmd)?,
        Command::Remove(sub_cmd) => remove(&store, sub_cmd)?,
        Command::Update(sub_cmd) => update(&store, sub_cmd)?,
        Command::Start(params) => display::log_started(&cmd::start_log(&store, &params)?),
        Command::Stop(params) => display::log_stopped(&cmd::stop_log(&store, &params)?),
        Command::Cancel => display::log_cancelled(cmd::cancel_log(&store)?.as_ref()),
        Command::Status => display::log_status(cmd::active_log_status(&store)?),
        Command::States(params) => display::task_states(cmd::task_states(&store, &params)?),
        Command::List(list_cmd) => list(&store, list_cmd)?,
    }
    Ok(())
}

fn add(store: &Store, cmd: AddCommand) -> Result<(), Box<dyn Error>> {
    match cmd {
        AddCommand::Project(params) => display::project_added(&cmd::add_project(store, &params)?),
        AddCommand::Task(params) => display::task_added(&cmd::add_task(store, &params)?),
        AddCommand::Log(params) => display::log_added(&cmd::add_log(store, &params)?),
    }
    Ok(())
}

fn remove(store: &Store, cmd: RemoveCommand) -> Result<(), Box<dyn Error>> {
    match cmd {
        RemoveCommand::Project(params) => {
            display::project_removed(&cmd::remove_project(store, &params)?)
        }
    }
    Ok(())
}

fn update(store: &Store, cmd: UpdateCommand) -> Result<(), Box<dyn Error>> {
    match cmd {
        UpdateCommand::Task(params) => display::task_updated(&cmd::update_task(store, &params)?),
    }
    Ok(())
}

fn list(store: &Store, cmd: ListCommand) -> Result<(), Box<dyn Error>> {
    match cmd {
        ListCommand::Projects(params) => {
            display::projects(cmd::list_projects(store, &params)?, &params)
        }
        ListCommand::Tasks(params) => display::tasks(cmd::list_tasks(store, &params)?),
        ListCommand::Logs(params) => display::logs(cmd::list_logs(store, &params)?, &params),
    }
    Ok(())
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
        error!("{}", e);
    }
}
