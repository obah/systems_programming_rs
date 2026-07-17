use todo::commands;

fn main() {
    if let Err(e) = commands::run_command() {
        eprintln!("{e}");
        std::process::exit(1);
    };
}
