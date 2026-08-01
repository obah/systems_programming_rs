use std::io::{self, Write};

use calc_mini_interpreter::CalcError;
use calc_mini_interpreter::evaluator::{Env, eval};
use calc_mini_interpreter::lexer::tokenize;
use calc_mini_interpreter::parser::parse;

fn run(src: &str, env: &mut Env) -> Result<f64, CalcError> {
    let tokens = tokenize(src.to_string())?;
    let ast = parse(&tokens)?;
    eval(&ast, env)
}

fn main() {
    let mut env = Env::new();
    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("> ");

        if io::stdout().flush().is_err() {
            break;
        }

        line.clear();
        match stdin.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("read error: {e}");
                break;
            }
        }

        let src = line.trim();
        if src.is_empty() {
            continue;
        }
        if src == "exit" || src == "quit" {
            break;
        }

        match run(src, &mut env) {
            Ok(value) => println!("{value}"),
            Err(e) => eprintln!("error: {e:?}"),
        }
    }
}
