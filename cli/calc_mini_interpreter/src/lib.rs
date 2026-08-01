pub mod evaluator;
pub mod lexer;
pub mod parser;

#[derive(Debug)]
pub enum CalcError {
    Unknown(String),
    Parse { value: String, reason: String },
}
