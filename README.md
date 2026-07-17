# Systems Programming in Rust

A collection of my systems programming learning projects, built in Rust as one Cargo workspace. Each project lives in its own folder with its own README.

```sh
git clone https://github.com/obah/systems_programming_rs.git
cd systems_programming_rs
cargo run -p <package> -- <args>
```

## Projects

### CLI tools

| Project           | Status  | Description                                                                                                     |
| ----------------- | ------- | --------------------------------------------------------------------------------------------------------------- |
| [todo](cli/todo/) | ✅ Done | A task manager for the terminal — add, list, rename, complete and delete tasks, persisted to a plain text file. |

## Development

```sh
cargo test -p <package>   # test one project
cargo test                # test everything
cargo clippy
cargo fmt
```
