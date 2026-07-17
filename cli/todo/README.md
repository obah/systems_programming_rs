# TODO CLI

A simple task manager for the terminal, built as part of my Rust and systems engineering learning journey.

## Features

- Add tasks with a title (as an argument, or via an interactive prompt)
- List all active tasks with their id and completion status
- Rename a task
- Mark a task as complete, or move it back to pending
- Delete a task (soft delete — hidden from the list but kept in the data file)
- Tasks persist between runs in a plain text file

## Usage

Every command takes the form `<command> [arguments]`.

| Command | Arguments | Example | Description |
| --- | --- | --- | --- |
| `add` | `[title]` | `add "buy milk"` | Add a new task. If the title is omitted, you'll be prompted for one. |
| `list` | — | `list` | Show all active tasks. |
| `rename` | `<id> <title>` | `rename 1 "buy oat milk"` | Change a task's title. |
| `complete` | `<id>` | `complete 1` | Mark a task as done. |
| `pend` | `<id>` | `pend 1` | Move a completed task back to pending. |
| `delete` | `<id>` | `delete 1` | Remove a task from the list. |

### Running from the repo (no install)

Use `cargo run -p todo --` from the workspace root and pass the command after the `--` separator:

```sh
cargo run -p todo -- add "buy milk"
cargo run -p todo -- list
cargo run -p todo -- complete 1
```

### Installed

Install the binary into `~/.cargo/bin` and use it directly:

```sh
cargo install --path .

todo_cli add "buy milk"
todo_cli list
todo_cli complete 1
```

Example `list` output:

```text
1. [X] buy milk
2. [ ] walk the dog
```

> **Note:** tasks are stored in a `my_todo.txt` file created relative to wherever you run the command, so each working directory gets its own independent todo list.

## Implementation

The project is a small library crate with a thin binary on top:

```text
src/
├── main.rs      # entry point: runs the command, prints errors, sets the exit code
├── lib.rs       # crate root: declares modules and the shared TodoError type
├── commands.rs  # CLI layer: parses args into a Command enum and executes it
├── task.rs      # domain: Task struct, Status enum, encoding/decoding
└── store.rs     # persistence: opening, reading and writing the data file
tests/
└── cli.rs       # end-to-end tests that run the compiled binary
```

Some design details:

- **Parsing before executing** — arguments are first parsed into a typed `Command` enum (`Add`, `Rename { id, title }`, `Complete(id)`, ...), then matched and executed. Invalid input fails at the parsing stage with a clear error instead of surfacing mid-operation.
- **Storage format** — tasks are stored one per line as pipe-delimited text: `id|status|deleted|title` (e.g. `1|pending|false|buy milk`). The title is the last field and the line is split at most 4 times, so titles may safely contain `|`. Encoding and decoding are implemented as the standard `Display` and `FromStr` traits on `Task`.
- **Whole-file rewrites** — every mutation loads all tasks, applies the change in memory, and rewrites the file. Simple and safe at this scale.
- **Error handling** — a single `TodoError` enum (`Io`, `Parse`, `NotFound`, `BadArgs`) implements `std::error::Error` and is propagated everywhere with `?`. Parse errors report the offending line number. On failure the binary prints the error to stderr and exits with a non-zero code.
- **Soft deletes** — deleting a task sets a `deleted` flag rather than removing the line, and `list` filters flagged tasks out.

## Development

```sh
cargo test    # run all unit + integration tests
cargo clippy  # lint
cargo fmt     # format
```

Unit tests live in `#[cfg(test)]` modules inside each source file. Integration tests in `tests/cli.rs` run the actual binary against a temporary working directory per test.
