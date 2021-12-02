//! Integration testing for Loiter.

use loiter::{Project, Store, Task};
use tempfile::tempdir;

#[test]
fn store_and_load() {
    let temp_dir = tempdir().unwrap();
    let store = Store::new(temp_dir.path()).unwrap();
    let config = store.config().unwrap();

    let project1 = Project::new("Project 1");
    store.save_project(&project1).unwrap();
    assert_eq!(project1.id(), "project-1");
    let loaded_project1 = store.project("project-1").unwrap();
    assert_eq!(project1, loaded_project1);

    let task1 =
        Task::new("project-1", "Some task").with_state(config.task_state_config().initial());
    let task1 = store.save_task(&task1).unwrap();
    assert_eq!(task1.id().unwrap(), 1);
    let loaded_task1 = store.task("project-1", 1).unwrap();
    assert_eq!(task1, loaded_task1);

    let task2 =
        Task::new("project-1", "Another task").with_state(config.task_state_config().done());
    let task2 = store.save_task(&task2).unwrap();
    assert_eq!(task2.id().unwrap(), 2);
    let loaded_task2 = store.task("project-1", 2).unwrap();
    assert_eq!(task2, loaded_task2);
}

#[test]
fn rename_project() {
    let temp_dir = tempdir().unwrap();
    let store = Store::new(temp_dir.path()).unwrap();
    let config = store.config().unwrap();

    let project1 = Project::new("Project 1");
    store.save_project(&project1).unwrap();
    let task1 =
        Task::new("project-1", "Some task").with_state(config.task_state_config().initial());
    let task1 = store.save_task(&task1).unwrap();

    let project_renamed = project1.with_name("Project renamed");
    store.rename_project("project-1", &project_renamed).unwrap();
    let loaded_project_renamed = store.project("project-renamed").unwrap();
    assert_eq!(project_renamed, loaded_project_renamed);

    // The old task should be part of the renamed project now.
    let loaded_task1 = store.task("project-renamed", 1).unwrap();
    assert_eq!(loaded_task1.description(), task1.description());

    // We should not be able to find the task under the old project any more.
    let r = store.task("project-1", 1);
    assert!(r.is_err());
}
