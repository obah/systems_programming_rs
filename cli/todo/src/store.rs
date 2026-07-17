use crate::TodoError;
use crate::task::Task;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
};

pub fn open_or_create_todo() -> Result<File, TodoError> {
    Ok(OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("my_todo.txt")?)
}

pub fn get_tasks(file: &mut File) -> Result<Vec<Task>, TodoError> {
    file.seek(std::io::SeekFrom::Start(0))?;

    let mut contents = String::new();

    file.read_to_string(&mut contents)?;

    let mut tasks = Vec::new();

    for (i, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let task = line.parse::<Task>().map_err(|reason| TodoError::Parse {
            line: i + 1,
            reason,
        })?;
        tasks.push(task);
    }

    Ok(tasks)
}

pub fn save_all_tasks(file: &mut File, tasks: &[Task]) -> Result<(), TodoError> {
    file.set_len(0)?;

    file.seek(std::io::SeekFrom::Start(0))?;

    // file.write(tasks)?;

    for task in tasks {
        writeln!(file, "{task}")?;
    }

    Ok(())
}

// how to implement the TaskList(Vec<Task>) type ??

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file_with(contents: &str) -> File {
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        file
    }

    #[test]
    fn save_then_read_round_trip() {
        let mut file = tempfile::tempfile().unwrap();
        let mut tasks = vec![
            Task::new(1, String::from("buy milk")),
            Task::new(2, String::from("walk the dog")),
        ];
        tasks[1].complete();

        save_all_tasks(&mut file, &tasks).unwrap();
        let read_back = get_tasks(&mut file).unwrap();

        assert_eq!(read_back, tasks);
    }

    #[test]
    fn empty_file_gives_empty_vec() {
        let mut file = tempfile::tempfile().unwrap();

        let tasks = get_tasks(&mut file).unwrap();

        assert!(tasks.is_empty());
    }

    #[test]
    fn blank_lines_are_skipped() {
        let mut file = temp_file_with("1|pending|false|buy milk\n\n  \n2|done|false|walk the dog\n");

        let tasks = get_tasks(&mut file).unwrap();

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, 1);
        assert_eq!(tasks[1].id, 2);
    }

    #[test]
    fn malformed_line_reports_line_number() {
        let mut file = temp_file_with("1|pending|false|buy milk\n\nnot a task\n");

        let err = get_tasks(&mut file).unwrap_err();

        match err {
            TodoError::Parse { line, .. } => assert_eq!(line, 3),
            other => panic!("expected Parse error, got: {other}"),
        }
    }

    #[test]
    fn save_overwrites_previous_content() {
        let mut file = tempfile::tempfile().unwrap();
        let three_tasks = vec![
            Task::new(1, String::from("a")),
            Task::new(2, String::from("b")),
            Task::new(3, String::from("c")),
        ];
        let one_task = vec![Task::new(9, String::from("only me"))];

        save_all_tasks(&mut file, &three_tasks).unwrap();
        save_all_tasks(&mut file, &one_task).unwrap();
        let read_back = get_tasks(&mut file).unwrap();

        assert_eq!(read_back, one_task);
    }
}
