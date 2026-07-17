use std::fmt::Display;

pub mod commands;
pub mod store;
pub mod task;

#[derive(Debug)]
pub enum TodoError {
    Io(std::io::Error),
    Parse { line: usize, reason: String },
    NotFound(u32),
    BadArgs(String),
}

impl Display for TodoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "An error occured: {e}"),
            Self::Parse { line, reason } => write!(f, "error {reason} on line {line}!"),
            Self::NotFound(id) => write!(f, "Task {id} not found!"),
            Self::BadArgs(e) => write!(f, "Unknown args: {e}"),
        }
    }
}

impl From<std::io::Error> for TodoError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<std::num::ParseIntError> for TodoError {
    fn from(err: std::num::ParseIntError) -> Self {
        Self::BadArgs(err.to_string())
    }
}

impl std::error::Error for TodoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_display_names_the_id() {
        assert_eq!(TodoError::NotFound(7).to_string(), "Task 7 not found!");
    }

    #[test]
    fn parse_display_includes_line_and_reason() {
        let err = TodoError::Parse {
            line: 3,
            reason: String::from("Invalid ID: abc"),
        };

        let msg = err.to_string();

        assert!(msg.contains("line 3"));
        assert!(msg.contains("Invalid ID: abc"));
    }

    #[test]
    fn io_error_converts_to_io_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "nope");

        let err: TodoError = io_err.into();

        assert!(matches!(err, TodoError::Io(_)));
    }
}
