use std::{fmt::Display, str::FromStr};

const STRUCT_FIELDS: usize = 4;

#[derive(Debug, PartialEq)]
pub enum Status {
    Pending,
    Done,
}

#[derive(Debug, PartialEq)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub status: Status,
    pub deleted: bool,
}

impl Task {
    pub fn new(id: u32, title: String) -> Self {
        Self {
            id,
            title: title.trim().to_string(),
            status: Status::Pending,
            deleted: false,
        }
    }

    pub fn rename(&mut self, new_title: String) {
        self.title = new_title;
    }

    pub fn delete(&mut self) {
        self.deleted = true;
    }

    pub fn complete(&mut self) {
        self.status = Status::Done;
    }

    pub fn pend(&mut self) {
        self.status = Status::Pending;
    }
}

impl Status {
    pub fn checkbox_char(&self) -> char {
        match self {
            Self::Done => 'X',
            Self::Pending => ' ',
        }
    }
}

//encode
impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}|{}|{}|{}",
            self.id, self.status, self.deleted, self.title
        )
    }
}

//decode
impl FromStr for Task {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let items: Vec<&str> = s.splitn(STRUCT_FIELDS, '|').collect();

        if items.len() < STRUCT_FIELDS {
            return Err(format!(
                "Malformed task string. Expected {} fields, found {}",
                STRUCT_FIELDS,
                items.len()
            ));
        }

        let id = items[0]
            .parse::<u32>()
            .map_err(|_| format!("Invalid ID: {}", items[0]))?;

        let status: Status = items[1]
            .parse()
            .map_err(|e| format!("Invalid status: {}", e))?;

        let deleted = items[2]
            .parse::<bool>()
            .map_err(|_| format!("Invalid deleted value: {}", items[2]))?;

        let title = items[3].to_string();

        Ok(Self {
            id,
            title,
            status,
            deleted,
        })
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Done => write!(f, "done"),
        }
    }
}

impl FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cleaned_s = s.trim().to_lowercase();

        match cleaned_s.as_str() {
            "pending" => Ok(Self::Pending),
            "done" => Ok(Self::Done),
            other => Err(format!("'{}' is not a valid status", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_trims_title_and_sets_defaults() {
        let task = Task::new(1, String::from("  buy milk \n"));

        assert_eq!(task.id, 1);
        assert_eq!(task.title, "buy milk");
        assert_eq!(task.status, Status::Pending);
        assert!(!task.deleted);
    }

    #[test]
    fn encode_decode_round_trip() {
        let mut task = Task::new(3, String::from("buy milk"));
        task.complete();

        let decoded: Task = task.to_string().parse().unwrap();

        assert_eq!(decoded, task);
    }

    #[test]
    fn round_trip_preserves_pipe_in_title() {
        let task = Task::new(1, String::from("buy milk | eggs | bread"));

        let decoded: Task = task.to_string().parse().unwrap();

        assert_eq!(decoded.title, "buy milk | eggs | bread");
    }

    #[test]
    fn decode_valid_line() {
        let task: Task = "7|done|true|walk the dog".parse().unwrap();

        assert_eq!(task.id, 7);
        assert_eq!(task.status, Status::Done);
        assert!(task.deleted);
        assert_eq!(task.title, "walk the dog");
    }

    #[test]
    fn decode_rejects_too_few_fields() {
        assert!("5|done".parse::<Task>().is_err());
    }

    #[test]
    fn decode_rejects_empty_line() {
        assert!("".parse::<Task>().is_err());
    }

    #[test]
    fn decode_rejects_invalid_id() {
        assert!("abc|pending|false|title".parse::<Task>().is_err());
    }

    #[test]
    fn decode_rejects_invalid_status() {
        assert!("1|donee|false|title".parse::<Task>().is_err());
    }

    #[test]
    fn decode_rejects_invalid_deleted_flag() {
        let err = "1|pending|maybe|title".parse::<Task>().unwrap_err();

        assert!(err.contains("maybe"));
    }

    #[test]
    fn status_parse_trims_and_ignores_case() {
        assert_eq!("  DONE ".parse::<Status>().unwrap(), Status::Done);
        assert_eq!("Pending".parse::<Status>().unwrap(), Status::Pending);
    }

    #[test]
    fn status_parse_rejects_unknown_value() {
        assert!("finished".parse::<Status>().is_err());
    }

    #[test]
    fn complete_and_pend_toggle_status() {
        let mut task = Task::new(1, String::from("a"));

        task.complete();
        assert_eq!(task.status, Status::Done);

        task.pend();
        assert_eq!(task.status, Status::Pending);
    }

    #[test]
    fn delete_marks_task_deleted() {
        let mut task = Task::new(1, String::from("a"));

        task.delete();

        assert!(task.deleted);
    }

    #[test]
    fn rename_replaces_title() {
        let mut task = Task::new(1, String::from("old"));

        task.rename(String::from("new"));

        assert_eq!(task.title, "new");
    }
}
