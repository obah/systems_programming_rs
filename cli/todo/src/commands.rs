use crate::TodoError;
use crate::{store, task::Task};
use std::fs::File;
use std::{env, io};

const ADD_COMMAND: &str = "add";
const RENAME_COMMAND: &str = "rename";
const COMPLETE_COMMAND: &str = "complete";
const LIST_COMMAND: &str = "list";
const DELETE_COMMAND: &str = "delete";
const PEND_COMMAND: &str = "pend";

#[derive(Debug, PartialEq)]
enum Command {
    Add(Option<String>),
    Rename { id: u32, title: String },
    Complete(u32),
    Pend(u32),
    Delete(u32),
    List,
}

enum UpdateOp {
    Rename(String),
    Complete,
    Delete,
    Pend,
}

pub fn get_user_input(prompt: &str) -> Result<String, TodoError> {
    println!("{}", prompt);
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn parse_command(args: &[String]) -> Result<Command, TodoError> {
    let cmd = args
        .get(1)
        .ok_or_else(|| TodoError::BadArgs("usage: todo <add|list|...>".into()))?;

    match cmd.as_str() {
        ADD_COMMAND => Ok(Command::Add(args.get(2).cloned())),
        DELETE_COMMAND => Ok(Command::Delete(parse_id(args.get(2))?)),
        RENAME_COMMAND => Ok(Command::Rename {
            id: parse_id(args.get(2))?,
            title: args
                .get(3)
                .cloned()
                .ok_or_else(|| TodoError::BadArgs("rename needs a new title".into()))?,
        }),
        COMPLETE_COMMAND => Ok(Command::Complete(parse_id(args.get(2))?)),
        PEND_COMMAND => Ok(Command::Pend(parse_id(args.get(2))?)),
        LIST_COMMAND => Ok(Command::List),
        _ => Err(TodoError::BadArgs(String::from("Unkown command!"))),
    }
}

fn parse_id(id: Option<&String>) -> Result<u32, TodoError> {
    match id {
        Some(id) => id
            .trim()
            .parse::<u32>()
            .map_err(|_| TodoError::BadArgs(format!("invalid task id: {}", id.trim()))),
        None => Err(TodoError::BadArgs("ID not provided!".to_string())),
    }
}

pub fn run_command() -> Result<(), TodoError> {
    let args: Vec<String> = env::args().collect();

    let command = parse_command(&args)?;

    match command {
        Command::Add(title) => run_add(title),
        Command::Rename { id, title } => run_update(id, UpdateOp::Rename(title), "renamed"),
        Command::Complete(id) => run_update(id, UpdateOp::Complete, "completed"),
        Command::List => run_list(),
        Command::Delete(id) => run_update(id, UpdateOp::Delete, "deleted"),
        Command::Pend(id) => run_update(id, UpdateOp::Pend, "now pending"),
    }
}

fn run_add(title: Option<String>) -> Result<(), TodoError> {
    let task_title = if let Some(t) = title {
        t
    } else {
        get_user_input("Enter title for new task: ")?
    };

    let (mut file, mut tasks) = get_tasks()?;

    let id: u32 = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;

    let new_task = Task::new(id, task_title);
    tasks.push(new_task);

    store::save_all_tasks(&mut file, &tasks)?;

    println!("New task added!");

    Ok(())
}

fn run_update(id: u32, op: UpdateOp, success_msg: &str) -> Result<(), TodoError> {
    let (mut file, mut tasks) = get_tasks()?;

    update_task(&mut tasks, id, op)?;

    store::save_all_tasks(&mut file, &tasks)?;

    println!("Task {success_msg}!");

    Ok(())
}

fn update_task(tasks: &mut [Task], id: u32, operation: UpdateOp) -> Result<(), TodoError> {
    let task = tasks
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or(TodoError::NotFound(id))?;

    match operation {
        UpdateOp::Complete => task.complete(),
        UpdateOp::Delete => task.delete(),
        UpdateOp::Rename(new_title) => task.rename(new_title),
        UpdateOp::Pend => task.pend(),
    }

    Ok(())
}

fn run_list() -> Result<(), TodoError> {
    let (_, tasks) = get_tasks()?;

    let tasks = tasks.into_iter().filter(|t| !t.deleted);

    for task in tasks {
        println!(
            "{}. [{}] {}",
            task.id,
            task.status.checkbox_char(),
            task.title
        );
    }

    Ok(())
}

fn get_tasks() -> Result<(File, Vec<Task>), TodoError> {
    let mut file = store::open_or_create_todo()?;
    let tasks = store::get_tasks(&mut file)?;

    Ok((file, tasks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Status;

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("todo")
            .chain(list.iter().copied())
            .map(String::from)
            .collect()
    }

    fn sample_tasks() -> Vec<Task> {
        vec![
            Task::new(1, String::from("buy milk")),
            Task::new(2, String::from("walk the dog")),
        ]
    }

    #[test]
    fn no_args_is_bad_args() {
        let err = parse_command(&args(&[])).unwrap_err();

        assert!(matches!(err, TodoError::BadArgs(_)));
    }

    #[test]
    fn unknown_command_is_bad_args() {
        let err = parse_command(&args(&["frobnicate"])).unwrap_err();

        assert!(matches!(err, TodoError::BadArgs(_)));
    }

    #[test]
    fn add_with_title() {
        let command = parse_command(&args(&["add", "buy milk"])).unwrap();

        assert_eq!(command, Command::Add(Some(String::from("buy milk"))));
    }

    #[test]
    fn bare_add_prompts_later() {
        let command = parse_command(&args(&["add"])).unwrap();

        assert_eq!(command, Command::Add(None));
    }

    #[test]
    fn delete_with_id() {
        let command = parse_command(&args(&["delete", "5"])).unwrap();

        assert_eq!(command, Command::Delete(5));
    }

    #[test]
    fn delete_without_id_is_bad_args() {
        let err = parse_command(&args(&["delete"])).unwrap_err();

        assert!(matches!(err, TodoError::BadArgs(_)));
    }

    #[test]
    fn non_numeric_id_is_bad_args() {
        let err = parse_command(&args(&["complete", "abc"])).unwrap_err();

        assert!(matches!(err, TodoError::BadArgs(_)));
    }

    #[test]
    fn rename_parses_id_and_title() {
        let command = parse_command(&args(&["rename", "3", "new title"])).unwrap();

        assert_eq!(
            command,
            Command::Rename {
                id: 3,
                title: String::from("new title")
            }
        );
    }

    #[test]
    fn rename_without_title_is_bad_args() {
        let err = parse_command(&args(&["rename", "3"])).unwrap_err();

        assert!(matches!(err, TodoError::BadArgs(_)));
    }

    #[test]
    fn parse_id_trims_padding() {
        let id = parse_id(Some(&String::from(" 7 "))).unwrap();

        assert_eq!(id, 7);
    }

    #[test]
    fn update_complete_only_touches_matching_task() {
        let mut tasks = sample_tasks();

        update_task(&mut tasks, 2, UpdateOp::Complete).unwrap();

        assert_eq!(tasks[1].status, Status::Done);
        assert_eq!(tasks[0].status, Status::Pending);
    }

    #[test]
    fn update_rename_replaces_title() {
        let mut tasks = sample_tasks();

        update_task(
            &mut tasks,
            1,
            UpdateOp::Rename(String::from("buy oat milk")),
        )
        .unwrap();

        assert_eq!(tasks[0].title, "buy oat milk");
    }

    #[test]
    fn update_delete_marks_deleted() {
        let mut tasks = sample_tasks();

        update_task(&mut tasks, 1, UpdateOp::Delete).unwrap();

        assert!(tasks[0].deleted);
        assert!(!tasks[1].deleted);
    }

    #[test]
    fn update_pend_reverts_completed_task() {
        let mut tasks = sample_tasks();
        tasks[0].complete();

        update_task(&mut tasks, 1, UpdateOp::Pend).unwrap();

        assert_eq!(tasks[0].status, Status::Pending);
    }

    #[test]
    fn update_missing_id_is_not_found() {
        let mut tasks = sample_tasks();

        let err = update_task(&mut tasks, 99, UpdateOp::Complete).unwrap_err();

        assert!(matches!(err, TodoError::NotFound(99)));
    }
}
