use diff::{commands, commands::Outcome};

fn main() {
    match commands::run_command() {
        Ok(diff) => match diff {
            Outcome::Same => {
                println!("\x1b[34mThe files are same\x1b[0m");
                std::process::exit(0)
            }
            Outcome::Changed => {
                println!("\x1b[31mThe files have changed\x1b[0m");
                std::process::exit(1)
            }
        },
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    }

    // for (i, line) in content.lines().enumerate() {
    //     let n = i + 1;
    //     let color = if n % 3 == 0 {
    //         34
    //     } else if n % 2 == 1 {
    //         32
    //     } else {
    //         31
    //     };
    //     println!("\x1b[{color}m{line}\x1b[0m");
    // }
}
