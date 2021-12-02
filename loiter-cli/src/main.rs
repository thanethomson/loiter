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
    /// Start a work log.
    Start(cmd::StartLog),
    /// Stop the currently active work log.
    Stop(cmd::StopLog),
    /// Show the status of the currently active work log (if any).
    Status,
    //    Find,
    /// List projects, tasks or work logs.
    List(ListCommand),
    //    Remove,
    //    Update,
}

#[derive(Debug, StructOpt)]
enum AddCommand {
    Project(cmd::AddProject),
    Task(cmd::AddTask),
    Log(cmd::AddLog),
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
        Command::Start(params) => {
            let _ = cmd::start_log(&store, params)?;
        }
        Command::Stop(params) => {
            let _ = cmd::stop_log(&store, params)?;
        }
        Command::Status => cmd::status(&store)?,
        Command::List(list_cmd) => list(&store, list_cmd)?,
    }
    Ok(())
}

fn add(store: &Store, cmd: AddCommand) -> Result<(), Box<dyn Error>> {
    match cmd {
        AddCommand::Project(params) => {
            let _ = cmd::add_project(store, params)?;
        }
        AddCommand::Task(params) => {
            let _ = cmd::add_task(store, params)?;
        }
        AddCommand::Log(params) => {
            let _ = cmd::add_log(store, params)?;
        }
    }
    Ok(())
}

fn list(store: &Store, cmd: ListCommand) -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        match cmd {
            ListCommand::Projects(params) => cmd::list_projects(store, params)?,
            ListCommand::Tasks(params) => cmd::list_tasks(store, params)?,
            ListCommand::Logs(params) => cmd::list_logs(store, params)?,
        }
    );

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
