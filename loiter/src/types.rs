//! Data types used by Loiter.

use std::{collections::HashSet, str::FromStr};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{strings::slugify, Duration, Error, Timestamp};

pub type ProjectId = String;
pub type TaskId = u32;
pub type LogId = u32;
pub type TaskState = String;

/// Task priority is a numeric value where lower values indicate higher
/// priority.
pub type TaskPriority = u8;

/// The minimum number of task states.
pub const MIN_TASK_STATES: usize = 3;

const DEFAULT_INITIAL_TASK_STATE: &str = "inbox";
const DEFAULT_IN_PROGRESS_TASK_STATE: &str = "doing";
const DEFAULT_DONE_TASK_STATE: &str = "done";
const DEFAULT_TASK_STATES: &[&str] = &[
    DEFAULT_INITIAL_TASK_STATE,
    "todo",
    "blocked",
    DEFAULT_IN_PROGRESS_TASK_STATE,
    DEFAULT_DONE_TASK_STATE,
];

/// The minimum value for task priority (corresponding to the highest priority).
pub const MIN_TASK_PRIORITY: TaskPriority = 1;

/// The highest value for task priority (corresponding to the lowest priority).
pub const MAX_TASK_PRIORITY: TaskPriority = 10;

/// By default, tasks have the lowest priority possible.
pub const DEFAULT_TASK_PRIORITY: TaskPriority = MAX_TASK_PRIORITY;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskStateConfig {
    states: Vec<TaskState>,
    initial: TaskState,
    in_progress: TaskState,
    done: TaskState,
}

impl Default for TaskStateConfig {
    fn default() -> Self {
        Self {
            states: DEFAULT_TASK_STATES.iter().map(|s| s.to_string()).collect(),
            initial: DEFAULT_INITIAL_TASK_STATE.to_string(),
            in_progress: DEFAULT_IN_PROGRESS_TASK_STATE.to_string(),
            done: DEFAULT_DONE_TASK_STATE.to_string(),
        }
    }
}

impl TaskStateConfig {
    /// Constructor.
    pub fn new(
        states: Vec<TaskState>,
        initial: TaskState,
        in_progress: TaskState,
        done: TaskState,
    ) -> Result<Self, Error> {
        if states.len() < MIN_TASK_STATES {
            return Err(Error::TooFewTaskStates(states.len(), MIN_TASK_STATES));
        }
        let states_set = states.clone().into_iter().collect::<HashSet<String>>();
        if states_set.len() != states.len() {
            return Err(Error::DuplicateTaskStates(states));
        }
        for state in [&initial, &in_progress, &done] {
            if !states_set.contains(state) {
                return Err(Error::InvalidTaskState(state.clone(), states));
            }
        }
        Ok(Self {
            states,
            initial,
            in_progress,
            done,
        })
    }

    /// If specified, checks whether the given string is a valid task state.
    ///
    /// If `maybe_state` is `None`, this returns the default initial task state
    /// for this configuration.
    pub fn validate_or_initial<S: AsRef<str>>(
        &self,
        maybe_state: Option<S>,
    ) -> Result<TaskState, Error> {
        match maybe_state {
            Some(state) => {
                let state = state.as_ref().to_string();
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    task_state_config: TaskStateConfig,
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct State {
    active_log: Option<(ProjectId, Option<TaskId>, LogId)>,
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

/// A type that facilitates comparisons of another type to order instances of
/// that type.
pub trait Comparator {
    type Type;

    fn cmp(&self, a: &Self::Type, b: &Self::Type) -> std::cmp::Ordering;
}

/// A specification as to how to sort a list of items using comparators.
///
/// Each comparator/ordering pair will be applied sequentially until the sorting
/// algorithm determines that two items are not equal, at which point they will
/// be reordered.
#[derive(Debug, Clone, PartialEq)]
pub struct SortSpec<C>(Vec<(C, Order)>);

impl<C> SortSpec<C>
where
    C: Comparator,
{
    /// Constructor.
    pub fn new(comparator: C, order: Order) -> Self {
        Self(vec![(comparator, order)])
    }

    /// Builder.
    pub fn and_then(mut self, comparator: C, order: Order) -> Self {
        self.0.push((comparator, order));
        self
    }

    /// Sort the given list of items by this sort specification.
    pub fn sort(&self, mut items: Vec<C::Type>) -> Vec<C::Type> {
        items.sort_by(|a, b| {
            for (comparator, order) in self.0.iter() {
                let cmp = match order {
                    Order::Asc => comparator.cmp(a, b),
                    Order::Desc => comparator.cmp(b, a),
                };
                match cmp {
                    std::cmp::Ordering::Less | std::cmp::Ordering::Greater => return cmp,
                    _ => continue,
                }
            }
            std::cmp::Ordering::Equal
        });
        items
    }
}

impl<C> FromStr for SortSpec<C>
where
    C: Comparator + FromStr<Err = Error>,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO: Add validation to prevent specifying the same field multiple
        // times.
        let specs = s
            .split(',')
            .map(|spec| {
                let spec_parts = spec.trim().split(':').collect::<Vec<&str>>();
                if spec_parts.is_empty() {
                    return Err(Error::SortSpecHasEmptyComponent(s.to_string()));
                }
                if spec_parts.len() > 2 {
                    return Err(Error::TooManyComponentsInSortSpec(
                        s.to_string(),
                        spec.to_string(),
                    ));
                }
                let comparator = C::from_str(spec_parts[0])?;
                let order = spec_parts
                    .get(1)
                    .map(|s| Order::from_str(s))
                    .unwrap_or_else(|| Ok(Order::default()))?;
                Ok((comparator, order))
            })
            .collect::<Result<Vec<(C, Order)>, Error>>()?;
        Ok(Self(specs))
    }
}

// Renders a sort specification. For example, the following specification:
// [
//     (TaskField::ProjectId, Order::Asc),
//     (TaskField::Id, Order::Desc),
// ]
//
// would be rendered as: "project-id,id:desc" (if the order is the default
// value, then it doesn't get rendered).
impl<C> std::fmt::Display for SortSpec<C>
where
    C: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|(comparator, order)| format!(
                    "{}{}",
                    comparator,
                    if *order == Order::default() {
                        "".to_string()
                    } else {
                        format!(":{}", *order)
                    }
                ))
                .collect::<Vec<String>>()
                .join(",")
        )
    }
}

/// The direction in which to sort items.
#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq)]
pub enum Order {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

impl Default for Order {
    fn default() -> Self {
        Self::Asc
    }
}

impl FromStr for Order {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "asc" | "a" => Self::Asc,
            "desc" | "d" => Self::Desc,
            _ => return Err(Error::UnrecognizedSortOrder(s.to_string())),
        })
    }
}

impl std::fmt::Display for Order {
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

/// A filter for specific types of objects.
pub trait Filter {
    type Type;

    /// Returns whether or not the given item matches this filter.
    ///
    /// Time needs to be supplied to facilitate testing.
    fn matches(&self, item: &Self::Type, now: Timestamp) -> bool;
}

/// A specification as to how to filter a list of objects.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterSpec<F>(Vec<F>);

impl<F> FilterSpec<F>
where
    F: Filter + Default + PartialEq,
{
    /// Constructor.
    ///
    /// A filter specification contains at least one filter.
    pub fn new(filter: F) -> Self {
        Self(vec![filter])
    }

    /// Builder.
    pub fn and_then(mut self, filter: F) -> Self {
        if self.is_passthrough() {
            self.0.clear();
        }
        self.0.push(filter);
        self
    }

    /// Returns whether this filter spec just contains a single default filter.
    pub fn is_passthrough(&self) -> bool {
        self.0.len() == 1 && self.0[0] == F::default()
    }
}

impl<F: Filter> Filter for FilterSpec<F> {
    type Type = F::Type;

    fn matches(&self, item: &Self::Type, now: Timestamp) -> bool {
        !self.0.iter().any(|filter| !filter.matches(item, now))
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

impl Default for SortSpec<ProjectField> {
    fn default() -> Self {
        Self(vec![(ProjectField::Name, Order::Asc)])
    }
}

impl Comparator for ProjectField {
    type Type = Project;

    fn cmp(&self, a: &Self::Type, b: &Self::Type) -> std::cmp::Ordering {
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

/// A general, human-friendly filter for timestamps.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimestampFilter {
    /// All entries today.
    Today,
    /// All entries tomorrow.
    Tomorrow,
    /// All entries yesterday.
    Yesterday,
    /// All entries this week (starting on a Monday).
    ThisWeek,
    /// All entries within the last given number of days.
    Days(u16),
    /// All entries this calendar month.
    ThisMonth,
    /// All entries this calendar year.
    ThisYear,
    /// All entries starting from the given timestamp (inclusive).
    Starting(Timestamp),
    /// All entries up to the given timestamp (exclusive).
    Before(Timestamp),
}

impl TimestampFilter {
    /// Attempt to parse the given string as a timestamp filter, using the given
    /// `now` as a reference.
    pub fn parse(s: &str, now: Timestamp) -> Result<Self, Error> {
        Ok(match s.trim().to_lowercase().as_str() {
            "today" => Self::Today,
            "tomorrow" | "tmrw" => Self::Tomorrow,
            "yesterday" | "yst" => Self::Yesterday,
            "week" | "this-week" => Self::ThisWeek,
            "month" | "this-month" => Self::ThisMonth,
            "year" | "this-year" => Self::ThisYear,
            filter => Self::try_parse_complex(filter, now)?,
        })
    }

    /// Given the current `now` value, does the specified timestamp `ts` match
    /// according to the timestamp filter?
    pub fn matches(&self, now: Timestamp, ts: Timestamp) -> bool {
        match self {
            Self::Today => ts >= now.today() && ts < now.tomorrow(),
            Self::Tomorrow => ts >= now.tomorrow() && ts < now.days_forward(2),
            Self::Yesterday => ts >= now.yesterday() && ts < now.today(),
            Self::ThisWeek => ts >= now.this_week() && ts < now.next_week(),
            Self::Days(days) => ts >= now.days_back(*days) && ts < now,
            Self::ThisMonth => ts >= now.this_month() && ts < now.next_month(),
            Self::ThisYear => ts >= now.this_year() && ts < now.next_year(),
            Self::Starting(starting) => ts >= *starting,
            Self::Before(before) => ts < *before,
        }
    }

    // Try to parse a complex timestamp filter.
    fn try_parse_complex(s: &str, now: Timestamp) -> Result<Self, Error> {
        let parts = s.split(' ').map(|p| p.trim()).collect::<Vec<&str>>();
        if parts.len() < 2 {
            return Err(Error::InvalidTimestampFilter(s.to_string()));
        }
        if parts[1] == "days" {
            let days = parts[0]
                .parse::<u16>()
                .map_err(|e| Error::TimestampFilterParsingFailed(e.to_string()))?;
            return Ok(Self::Days(days));
        }
        match parts[0] {
            "from" | "starting" | "start" => Ok(Self::Starting(Timestamp::parse(
                &parts[1..].join(" "),
                now,
            )?)),
            "to" | "before" | "ending" => {
                Ok(Self::Before(Timestamp::parse(&parts[1..].join(" "), now)?))
            }
            _ => Err(Error::InvalidTimestampFilter(s.to_string())),
        }
    }
}

impl FromStr for TimestampFilter {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let now = Timestamp::now()?;
        Self::parse(s, now)
    }
}

/// A filter for durations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum DurationFilter {
    LessThan(Duration),
    LessThanOrEqual(Duration),
    GreaterThan(Duration),
    GreaterThanOrEqual(Duration),
    Equal(Duration),
}

impl DurationFilter {
    pub fn matches(&self, duration: Duration) -> bool {
        match self {
            Self::LessThan(d) => duration < *d,
            Self::LessThanOrEqual(d) => duration <= *d,
            Self::GreaterThan(d) => duration > *d,
            Self::GreaterThanOrEqual(d) => duration >= *d,
            Self::Equal(d) => duration == *d,
        }
    }
}

impl FromStr for DurationFilter {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        // Find where the first digit starts
        let first_digit_idx = s
            .find(|c: char| c.is_ascii_digit())
            .ok_or_else(|| Error::InvalidDurationFilter(s.to_string()))?;
        let duration = Duration::from_str(&s[first_digit_idx..])?;
        Ok(match s[..first_digit_idx].trim() {
            "<=" => DurationFilter::LessThanOrEqual(duration),
            "<" => DurationFilter::LessThan(duration),
            ">=" => DurationFilter::GreaterThanOrEqual(duration),
            ">" => DurationFilter::GreaterThan(duration),
            "" | "=" | "==" => DurationFilter::Equal(duration),
            op => {
                return Err(Error::InvalidDurationFilterOp(
                    op.to_string(),
                    s.to_string(),
                ))
            }
        })
    }
}

/// For filtering projects by the contents of specific fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ProjectFilter {
    /// Include all projects.
    All,
    /// Projects whose IDs match these IDs.
    Ids(Vec<ProjectId>),
    /// Include all projects whose deadline is present and matches the given
    /// timestamp filter.
    Deadline(TimestampFilter),
    /// Include all projects whose tags match at least one of the given tags.
    Tags(Vec<String>),
}

impl Default for ProjectFilter {
    fn default() -> Self {
        Self::All
    }
}

impl Filter for ProjectFilter {
    type Type = Project;

    /// Returns whether the given project matches this filter.
    fn matches(&self, project: &Project, now: Timestamp) -> bool {
        match self {
            Self::All => true,
            Self::Ids(project_ids) => project_ids
                .iter()
                .any(|project_id| project_id == project.id()),
            Self::Deadline(ts_filter) => project
                .deadline()
                .map(|deadline| ts_filter.matches(now, deadline))
                .unwrap_or(false),
            Self::Tags(tags) => {
                project
                    .tags()
                    .collect::<HashSet<&str>>()
                    .intersection(&HashSet::from_iter(tags.iter().map(|t| t.as_str())))
                    .count()
                    > 0
            }
        }
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

    pub fn deadline(&self) -> Option<Timestamp> {
        self.maybe_deadline
    }

    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.tags.iter().map(|t| t.as_str())
    }

    pub fn task_state_config(&self) -> Option<&TaskStateConfig> {
        self.maybe_task_state_config.as_ref()
    }

    /// Convenience method to return an ordered list of task states supported by
    /// this project, using the given default config if this project has no
    /// custom task states.
    pub fn task_states(&self, default_config: &TaskStateConfig) -> Vec<TaskState> {
        self.maybe_task_state_config
            .as_ref()
            .map(|tsc| tsc.states())
            .unwrap_or_else(|| default_config.states())
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
    }
}

/// The fields on which task listings can be sorted.
#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq)]
pub enum TaskField {
    Id,
    ProjectId,
    Description,
    Priority,
    State,
    Deadline,
}

impl Default for SortSpec<TaskField> {
    fn default() -> Self {
        Self(vec![
            (TaskField::Priority, Order::Asc),
            (TaskField::ProjectId, Order::Asc),
            (TaskField::Id, Order::Asc),
        ])
    }
}

impl Comparator for TaskField {
    type Type = Task;

    fn cmp(&self, a: &Task, b: &Task) -> std::cmp::Ordering {
        match self {
            Self::Id => a.id().cmp(&b.id()),
            Self::ProjectId => a.project_id().cmp(&b.project_id()),
            Self::Description => a.description().cmp(b.description()),
            Self::Priority => a.priority().cmp(&b.priority()),
            Self::State => a.state().cmp(&b.state()),
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
            "priority" => Self::Priority,
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
                Self::Priority => "priority",
                Self::State => "state",
                Self::Deadline => "deadline",
            }
        )
    }
}

/// For filtering tasks by the contents of specific fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum TaskFilter {
    /// All tasks.
    All,
    /// Tasks belonging to the given project.
    Project(ProjectId),
    /// Tasks whose IDs match one or more of the given IDs.
    Ids(Vec<TaskId>),
    /// Tasks matching at least one of the given priorities.
    Priority(Vec<TaskPriority>),
    /// Tasks matching the given states.
    State(Vec<TaskState>),
    /// Tasks whose states do *not* include the given state.
    StateNot(TaskState),
    /// Tasks whose deadline matches the given timestamp filter.
    Deadline(TimestampFilter),
    /// Tasks whose tags match one or more of the given tags.
    Tags(Vec<String>),
}

impl Default for TaskFilter {
    fn default() -> Self {
        Self::All
    }
}

impl Filter for TaskFilter {
    type Type = Task;

    /// Returns whether the given task matches this filter.
    fn matches(&self, task: &Task, now: Timestamp) -> bool {
        match self {
            Self::All => true,
            Self::Project(project_id) => task
                .project_id()
                .map(|id| id == project_id)
                .unwrap_or(false),
            Self::Ids(ids) => task
                .id()
                .map(|task_id| ids.iter().any(|id| *id == task_id))
                .unwrap_or(false),
            Self::Priority(priorities) => priorities
                .iter()
                .any(|priority| task.priority() == *priority),
            Self::State(states) => states
                .iter()
                .any(|state| task.state().map(|ts| ts == state).unwrap_or(false)),
            Self::StateNot(state) => task.state().map(|ts| ts != state).unwrap_or(true),
            Self::Deadline(ts_filter) => task
                .deadline()
                .map(|deadline| ts_filter.matches(now, deadline))
                .unwrap_or(false),
            Self::Tags(tags) => {
                task.tags()
                    .collect::<HashSet<&str>>()
                    .intersection(&HashSet::from_iter(tags.iter().map(|t| t.as_str())))
                    .count()
                    > 0
            }
        }
    }
}

/// A discrete unit of work related to a project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    #[serde(skip)]
    maybe_project_id: Option<ProjectId>,
    #[serde(skip)]
    maybe_id: Option<TaskId>,
    #[serde(rename = "priority")]
    maybe_priority: Option<TaskPriority>,
    description: String,
    #[serde(rename = "state")]
    maybe_state: Option<TaskState>,
    #[serde(rename = "deadline")]
    maybe_deadline: Option<Timestamp>,
    tags: HashSet<String>,
}

impl Task {
    /// Constructor.
    pub fn new<S1, S2>(project_id: S1, description: S2) -> Self
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        Self {
            maybe_project_id: Some(project_id.as_ref().to_string()),
            maybe_id: None,
            maybe_priority: None,
            description: description.as_ref().to_string(),
            maybe_state: None,
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

    pub fn with_priority(mut self, priority: TaskPriority) -> Result<Self, Error> {
        if !(MIN_TASK_PRIORITY..=MAX_TASK_PRIORITY).contains(&priority) {
            return Err(Error::InvalidTaskPriority(
                priority,
                MIN_TASK_PRIORITY,
                MAX_TASK_PRIORITY,
            ));
        }
        self.maybe_priority = Some(priority);
        Ok(self)
    }

    pub fn with_description<S: AsRef<str>>(mut self, description: S) -> Self {
        self.description = description.as_ref().to_string();
        self
    }

    pub fn with_state<S: AsRef<str>>(mut self, state: S) -> Self {
        self.maybe_state = Some(state.as_ref().to_string());
        self
    }

    pub fn with_maybe_state(mut self, maybe_state: Option<TaskState>) -> Self {
        self.maybe_state = maybe_state;
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

    pub fn priority(&self) -> TaskPriority {
        self.maybe_priority.unwrap_or(DEFAULT_TASK_PRIORITY)
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn state(&self) -> Option<&str> {
        self.maybe_state.as_deref()
    }

    pub fn deadline(&self) -> Option<Timestamp> {
        self.maybe_deadline
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

impl Default for SortSpec<LogField> {
    fn default() -> Self {
        Self(vec![
            (LogField::ProjectId, Order::Asc),
            (LogField::TaskId, Order::Asc),
            (LogField::Id, Order::Asc),
        ])
    }
}

impl Comparator for LogField {
    type Type = Log;

    fn cmp(&self, a: &Log, b: &Log) -> std::cmp::Ordering {
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

/// For filtering work logs by the contents of specific fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum LogFilter {
    /// All logs.
    All,
    /// All logs belonging to the given project.
    Project(ProjectId),
    /// All logs that are associated with a task.
    HasTask,
    /// All logs belonging to the given task(s).
    Task(Vec<TaskId>),
    /// All logs whose start date/time matches the given timestamp filter.
    Start(TimestampFilter),
    /// All logs whose duration matches the given duration filter.
    Duration(DurationFilter),
    /// All logs whose tags match one or more of the given tags.
    Tags(Vec<String>),
}

impl Default for LogFilter {
    fn default() -> Self {
        Self::All
    }
}

impl Filter for LogFilter {
    type Type = Log;

    /// Returns whether the given work log matches this filter.
    fn matches(&self, log: &Log, now: Timestamp) -> bool {
        match self {
            Self::All => true,
            Self::Project(project_id) => {
                log.project_id().map(|id| id == project_id).unwrap_or(false)
            }
            Self::HasTask => log.task_id().is_some(),
            Self::Task(task_ids) => task_ids
                .iter()
                .any(|task_id| log.task_id().map(|id| id == *task_id).unwrap_or(false)),
            Self::Start(ts_filter) => log
                .start()
                .map(|start| ts_filter.matches(now, start))
                .unwrap_or(false),
            Self::Duration(dur_filter) => log
                .duration()
                .map(|duration| dur_filter.matches(duration))
                .unwrap_or(false),
            Self::Tags(tags) => {
                log.tags()
                    .collect::<HashSet<&str>>()
                    .intersection(&HashSet::from_iter(tags.iter().map(|t| t.as_str())))
                    .count()
                    > 0
            }
        }
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
    #[serde(rename = "comment")]
    maybe_comment: Option<String>,
    tags: HashSet<String>,
    // Allows us to optionally load the task associated with this log for
    // detailed information on the log.
    #[serde(skip)]
    maybe_task: Option<Task>,
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
            maybe_task: None,
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

    pub fn with_duration_or_stop_or_now(
        self,
        maybe_duration: Option<Duration>,
        maybe_stop: Option<Timestamp>,
    ) -> Result<Self, Error> {
        if maybe_duration.is_none() && maybe_stop.is_none() {
            self.with_stop(Timestamp::now()?)
        } else {
            self.with_maybe_duration_or_stop(maybe_duration, maybe_stop)
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

    pub fn with_maybe_task(mut self, maybe_task: Option<Task>) -> Self {
        self.maybe_task_id = maybe_task.as_ref().map(|task| task.id()).flatten();
        self.maybe_task = maybe_task;
        self
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

    pub fn start(&self) -> Option<Timestamp> {
        self.maybe_start
    }

    /// Computes the stop time from the start time and duration.
    ///
    /// If either the start time or the duration are not available, this returns
    /// `None`.
    pub fn stop(&self) -> Option<Timestamp> {
        let start = OffsetDateTime::from(self.maybe_start?);
        let duration = time::Duration::from(self.maybe_duration?);
        Some((start + duration).into())
    }

    pub fn duration(&self) -> Option<Duration> {
        self.maybe_duration
    }

    pub fn comment(&self) -> Option<&str> {
        self.maybe_comment.as_deref()
    }

    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.tags.iter().map(|t| t.as_str())
    }

    pub fn task(&self) -> Option<&Task> {
        self.maybe_task.as_ref()
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

#[cfg(test)]
mod test {
    use super::{
        Duration, DurationFilter, Order, ProjectField, SortSpec, Timestamp, TimestampFilter,
    };
    use lazy_static::lazy_static;
    use std::str::FromStr;
    use time::macros::datetime;

    lazy_static! {
        static ref TEST_NOW: Timestamp = Timestamp::from(datetime!(2021-12-05 19:42 -05:00));
        static ref SORT_SPEC_PARSING_TEST_CASES: Vec<(&'static str, SortSpec<ProjectField>)> = vec![
            ("id", SortSpec::new(ProjectField::Id, Order::Asc)),
            (
                "id,name",
                SortSpec::new(ProjectField::Id, Order::Asc)
                    .and_then(ProjectField::Name, Order::Asc)
            ),
            (
                "id:d, name:a",
                SortSpec::new(ProjectField::Id, Order::Desc)
                    .and_then(ProjectField::Name, Order::Asc),
            )
        ];
        static ref SORT_SPEC_DISPLAY_TEST_CASES: Vec<(SortSpec<ProjectField>, &'static str)> = vec![
            (SortSpec::new(ProjectField::Id, Order::Asc), "id"),
            (
                SortSpec::new(ProjectField::Name, Order::Desc)
                    .and_then(ProjectField::Id, Order::Asc),
                "name:desc,id"
            ),
        ];
        static ref TIMESTAMP_FILTER_PARSING_TEST_CASES: Vec<(&'static str, TimestampFilter)> = vec![
            ("today", TimestampFilter::Today),
            ("tomorrow", TimestampFilter::Tomorrow),
            ("tmrw", TimestampFilter::Tomorrow),
        ];
        static ref DURATION_FILTER_PARSING_TEST_CASES: Vec<(&'static str, DurationFilter)> =
            vec![(
                "< 1h",
                DurationFilter::LessThan(Duration::from(time::Duration::HOUR))
            ),];
    }

    #[test]
    fn sort_spec_parsing() {
        for (s, expected) in SORT_SPEC_PARSING_TEST_CASES.iter() {
            let actual = SortSpec::<ProjectField>::from_str(s).unwrap();
            assert_eq!(actual, expected.clone());
        }
    }

    #[test]
    fn sort_spec_display() {
        for (s, expected) in SORT_SPEC_DISPLAY_TEST_CASES.iter() {
            let actual = s.to_string();
            assert_eq!(&actual, expected);
        }
    }

    #[test]
    fn timestamp_filter_parsing() {
        for (s, expected) in TIMESTAMP_FILTER_PARSING_TEST_CASES.iter() {
            let actual = TimestampFilter::parse(s, *TEST_NOW).unwrap();
            assert_eq!(actual, *expected);
        }
    }

    #[test]
    fn duration_filter_parsing() {
        for (s, expected) in DURATION_FILTER_PARSING_TEST_CASES.iter() {
            let actual = DurationFilter::from_str(s).unwrap();
            assert_eq!(actual, *expected);
        }
    }
}
