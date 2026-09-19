#[derive(Debug, PartialEq)]
pub enum Frame {
    SimpleString(String),
    Error(String),
    Integer(i64),
    Null,
    BulkString(Vec<u8>),
    Array(Vec<Frame>),
}
