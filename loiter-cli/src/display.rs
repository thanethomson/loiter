//! Utilities for displaying data via the CLI.

use comfy_table::{presets, Attribute, Cell, Color, Table};
use crossterm::style::Stylize;
use loiter::{
    cmd::{ListLogs, ListProjects, LogStatus},
    Duration, Log, Project, Task, TaskState,
};

pub const COLOR_STATES: Color = Color::DarkCyan;
pub const COLOR_PROJECT: Color = Color::Blue;
pub const COLOR_DEADLINE: Color = Color::Red;
pub const COLOR_TAGS: Color = Color::Green;
pub const COLOR_TIME: Color = Color::Cyan;

/// List the given task states.
pub fn task_states(states: Vec<TaskState>) {
    let mut table = Table::new();
    table.load_preset(presets::NOTHING);
    for state in states.iter() {
        table.add_row(vec![Cell::new(state).fg(COLOR_STATES)]);
    }
    println!("{}", table);
}

/// Render the given list of projects with the specified parameters.
pub fn projects(projects: Vec<Project>, params: &ListProjects) {
    let mut table = Table::new();
    table.load_preset(presets::NOTHING);
    if params.detailed {
        table.set_header(header_cells(vec![
            "ID",
            "Name",
            "Description",
            "Deadline",
            "Tags",
        ]));
    }
    for project in projects {
        if params.detailed {
            table.add_row(vec![
                Cell::new(project.id()).fg(COLOR_PROJECT),
                Cell::new(project.name()),
                Cell::new(display_optional(project.description())),
                Cell::new(display_optional(project.deadline())).fg(COLOR_DEADLINE),
                Cell::new(join(project.tags(), ",")).fg(COLOR_TAGS),
            ]);
        } else {
            table.add_row(vec![Cell::new(project.id()).fg(Color::Blue)]);
        }
    }
    println!("{}", table);
}

pub fn project_added(project: &Project) {
    println!("Added project with ID {}", project.id().with(COLOR_PROJECT));
}

pub fn project_removed<S: AsRef<str>>(id: S) {
    println!("Removed project {}", id.as_ref().with(COLOR_PROJECT));
}

pub fn tasks(tasks: Vec<Task>) {
    let mut table = Table::new();
    table
        .load_preset(presets::NOTHING)
        .set_header(header_cells(vec![
            "Project",
            "ID",
            "Description",
            "State",
            "Deadline",
            "Tags",
        ]));
    for task in tasks {
        table.add_row(vec![
            Cell::new(task.project_id().unwrap()).fg(COLOR_PROJECT),
            Cell::new(task.id().unwrap()),
            Cell::new(task.description()),
            Cell::new(display_optional(task.state())).fg(COLOR_STATES),
            Cell::new(display_optional(task.deadline())).fg(COLOR_DEADLINE),
            Cell::new(join(task.tags(), ",")).fg(COLOR_TAGS),
        ]);
    }
    println!("{}", table);
}

pub fn task_added(task: &Task) {
    println!(
        "Added task {} for project {}{}",
        task.id().unwrap(),
        task.project_id().unwrap().with(COLOR_PROJECT),
        task.deadline()
            .map(|deadline| format!(
                " with deadline {}",
                deadline.to_string().with(COLOR_DEADLINE)
            ))
            .unwrap_or_else(|| "".to_string())
    );
}

pub fn task_updated(task: &Task) {
    println!(
        "Task {} updated for project {}",
        task.id().unwrap(),
        task.project_id().unwrap().with(COLOR_PROJECT),
    );
}

pub fn logs(logs: Vec<Log>, params: &ListLogs) {
    let mut table = Table::new();
    table.load_preset(presets::NOTHING);
    if params.detailed {
        table.set_header(header_cells(vec![
            "Project", "Task", "ID", "Start", "Duration", "Comment", "Tags",
        ]));
    } else {
        // Same as above, but without the duration.
        table.set_header(header_cells(vec![
            "Project", "Task", "ID", "Start", "Duration", "Tags",
        ]));
    }
    let mut total_duration = Duration::zero();
    let log_count = logs.len();
    for log in logs {
        if params.detailed {
            table.add_row(vec![
                Cell::new(log.project_id().unwrap()).fg(COLOR_PROJECT),
                Cell::new(display_optional(log.task_id())),
                Cell::new(log.id().unwrap()),
                Cell::new(display_optional(log.start())).fg(COLOR_TIME),
                Cell::new(display_optional(log.duration())).fg(COLOR_TIME),
                Cell::new(display_optional(log.comment())),
                Cell::new(join(log.tags(), ",")).fg(COLOR_TAGS),
            ]);
        } else {
            table.add_row(vec![
                Cell::new(log.project_id().unwrap()).fg(COLOR_PROJECT),
                Cell::new(display_optional(log.task_id())),
                Cell::new(log.id().unwrap()),
                Cell::new(display_optional(log.start())).fg(COLOR_TIME),
                Cell::new(display_optional(log.duration())).fg(COLOR_TIME),
                Cell::new(join(log.tags(), ",")).fg(COLOR_TAGS),
            ]);
        }
        total_duration += log.duration().unwrap_or_else(Duration::zero);
    }
    println!("{}", table);
    print!(
        "{} {}",
        log_count,
        if log_count == 1 { "log" } else { "logs" }
    );
    if total_duration > Duration::zero() {
        println!(", {}", total_duration.to_string().with(COLOR_TIME));
    } else {
        println!();
    }
}

pub fn log_added(log: &Log) {
    println!(
        "Log {} added for {}{}",
        log.id().unwrap(),
        log.project_id().unwrap().with(COLOR_PROJECT),
        display_optional(log.task_id().map(|task_id| format!(", task {}", task_id))),
    );
}

pub fn log_started(log: &Log) {
    println!(
        "Log {} for {}{} started at {}",
        log.id().unwrap(),
        log.project_id().unwrap().with(COLOR_PROJECT),
        display_optional(log.task_id().map(|task_id| format!(", task {}", task_id))),
        log.start().unwrap().to_string().with(COLOR_TIME),
    );
}

pub fn log_stopped(log: &Log) {
    println!(
        "Log {} for {}{} stopped at {} ({})",
        log.id().unwrap(),
        log.project_id().unwrap().with(COLOR_PROJECT),
        display_optional(log.task_id().map(|task_id| format!(", task {}", task_id))),
        log.stop().unwrap().to_string().with(COLOR_TIME),
        log.duration().unwrap().to_string().with(COLOR_TIME),
    );
}

pub fn log_cancelled(maybe_log: Option<&Log>) {
    match maybe_log {
        Some(log) => {
            println!(
                "Log {} for {}{} cancelled",
                log.id().unwrap(),
                log.project_id().unwrap().with(COLOR_PROJECT),
                display_optional(log.task_id().map(|task_id| format!(", task {}", task_id))),
            );
        }
        None => println!("No active log"),
    }
}

pub fn log_status(maybe_log_status: Option<LogStatus>) {
    match maybe_log_status {
        Some(status) => {
            println!(
                "Log {} for {}{} active since {} ({})",
                status.log.id().unwrap(),
                status.log.project_id().unwrap().with(COLOR_PROJECT),
                display_optional(
                    status
                        .log
                        .task_id()
                        .map(|task_id| format!(", task {}", task_id))
                ),
                status.log.start().unwrap().to_string().with(COLOR_TIME),
                status.active_for.to_string().with(COLOR_TIME),
            );
        }
        None => println!("No active log"),
    }
}

fn display_optional<D: std::fmt::Display>(v: Option<D>) -> String {
    v.map(|inner| inner.to_string())
        .unwrap_or_else(|| "".to_string())
}

fn join<D, I>(items: I, sep: &str) -> String
where
    D: std::fmt::Display,
    I: Iterator<Item = D>,
{
    items
        .map(|i| i.to_string())
        .collect::<Vec<String>>()
        .join(sep)
}

fn header_cell<S: ToString>(s: S) -> Cell {
    Cell::new(s).add_attribute(Attribute::Bold)
}

fn header_cells<S: ToString, I: IntoIterator<Item = S>>(headings: I) -> Vec<Cell> {
    headings.into_iter().map(header_cell).collect()
}
