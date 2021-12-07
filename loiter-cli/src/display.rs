//! Utilities for displaying data via the CLI.

use std::path::Path;

use comfy_table::{presets, Attribute, Cell, Color, ContentArrangement, Table};
use crossterm::style::Stylize;
use loiter::{
    cmd::{ListLogs, ListProjects, LogStatus},
    Duration, Log, Project, ProjectId, Task, TaskId, TaskState, MAX_TASK_PRIORITY,
};

pub const COLOR_STATES: Color = Color::DarkCyan;
pub const COLOR_PROJECT: Color = Color::Blue;
pub const COLOR_DEADLINE: Color = Color::Red;
pub const COLOR_TAGS: Color = Color::Green;
pub const COLOR_TIME: Color = Color::Cyan;
pub const COLOR_PRIORITY_HIGH: Color = Color::Red;
pub const COLOR_PRIORITY_MEDIUM: Color = Color::Yellow;
pub const COLOR_PRIORITY_LOW: Color = Color::Green;

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

pub fn tasks(tasks: Vec<Task>, maybe_active_task: Option<(ProjectId, TaskId)>) {
    let mut table = Table::new();
    table
        .load_preset(presets::NOTHING)
        .set_header(header_cells(vec![
            "Project",
            "ID",
            "Priority",
            "Description",
            "State",
            "Deadline",
            "Tags",
        ]))
        .set_content_arrangement(ContentArrangement::Dynamic);
    for task in tasks {
        let is_active = if let Some((project_id, task_id)) = &maybe_active_task {
            if task.project_id().unwrap() == project_id && task.id().unwrap() == *task_id {
                true
            } else {
                false
            }
        } else {
            false
        };

        let priority = task.priority();
        let mut cells = vec![
            Cell::new(task.project_id().unwrap()).fg(COLOR_PROJECT),
            Cell::new(task.id().unwrap()),
            Cell::new(priority.to_string()).fg(priority_color(priority)),
            Cell::new(task.description()),
            Cell::new(format!(
                "{}{}",
                display_optional(task.state()),
                if is_active { " ⏲️" } else { "" }
            ))
            .fg(COLOR_STATES),
            Cell::new(display_optional(task.deadline())).fg(COLOR_DEADLINE),
            Cell::new(join(task.tags(), ",")).fg(COLOR_TAGS),
        ];
        if is_active {
            cells = cells
                .into_iter()
                .map(|cell| cell.add_attribute(Attribute::Bold))
                .collect::<Vec<Cell>>();
        }

        table.add_row(cells);
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

pub fn tasks_updated(tasks: Vec<Task>) {
    let task_count = tasks.len();
    for task in tasks {
        println!(
            "Task {} {} updated for project {}",
            task.id().unwrap(),
            task.description().attribute(Attribute::Bold),
            task.project_id().unwrap().with(COLOR_PROJECT),
        );
    }
    println!(
        "{} {} updated",
        task_count,
        if task_count == 1 { "task" } else { "tasks" }
    );
}

pub fn logs(logs: Vec<Log>, params: &ListLogs) {
    let mut table = Table::new();
    table.load_preset(presets::NOTHING);
    if params.detailed {
        table
            .set_header(header_cells(vec![
                "Project",
                "Task",
                "ID",
                "Start",
                "Duration",
                "Comment",
                "Tags",
                "Task Description",
                "Task Tags",
            ]))
            .set_content_arrangement(ContentArrangement::Dynamic);
    } else {
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
                Cell::new(display_optional(log.task().map(|task| task.description()))),
                Cell::new(display_optional(
                    log.task().map(|task| join(task.tags(), ",")),
                ))
                .fg(COLOR_TAGS),
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
    println!();
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

pub fn remote_initialized(path: &Path) {
    println!("{} initialized as a Git repository", path.display());
}

pub fn remote_pushed(path: &Path) {
    println!("{} committed and pushed to remote", path.display());
}

pub fn remote_pulled(path: &Path) {
    println!("{} pulled from remote", path.display());
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

fn priority_color(priority: u8) -> Color {
    let prio_thresh = MAX_TASK_PRIORITY / 3;
    if priority <= prio_thresh {
        COLOR_PRIORITY_HIGH
    } else if priority <= 2 * prio_thresh {
        COLOR_PRIORITY_MEDIUM
    } else {
        COLOR_PRIORITY_LOW
    }
}
