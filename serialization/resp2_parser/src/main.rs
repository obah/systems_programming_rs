use resp2_parser::server;

fn main() {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("127.0.0.1:6379"));

    if let Err(e) = server::run(&addr) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
