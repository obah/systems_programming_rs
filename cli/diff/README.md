# DIFF CLI

A simple tool that checks two files for differences, built as part of my Rust and systems engineering learning journey.

> ⚠️ **Work in progress.** Right now it only reports *whether* two files differ. Per-line diff output and a proper longest-common-subsequence algorithm are still to come — see [Roadmap](#roadmap).

## Features

- Compares two files line by line and tells you whether they are the same or have changed.
- Colored output and a meaningful exit code (`0` same, `1` changed, `2` error) so it composes in scripts.

## Usage

Pass the two files as `[file1] [file2]`. Paths are resolved relative to the crate's `src/` directory, so the sample files under `src/files/` are addressed as `/files/a.txt`, `/files/b.txt`, `/files/c.txt`.

### Running from the repo (no install)

Use `cargo run -p diff --` from the workspace root and pass the files after the `--` separator:

```sh
# identical files -> "The files are same" (exit 0)
cargo run -p diff -- /files/a.txt /files/c.txt

# one line differs -> "The files have changed" (exit 1)
cargo run -p diff -- /files/a.txt /files/b.txt
```

### Installed

Install the binary into `~/.cargo/bin` and use it directly:

```sh
cargo install --path .

diff /files/a.txt /files/b.txt
```

Example output:

```text
The files are same        # a.txt vs c.txt (printed in blue)
The files have changed    # a.txt vs b.txt (printed in red)
```

The included samples exist to exercise both paths: `a.txt` and `c.txt` are identical, and `b.txt` differs from them by a single line.

## Implementation

A small library crate with a thin binary on top:

```text
src/
├── main.rs      # entry point: runs the command, prints the outcome, sets the exit code
├── lib.rs       # crate root: declares modules
├── commands.rs  # reads the args, loads both files, compares them line by line
└── files/       # sample inputs (a == c, b differs by one line)
```

Some design details:

- **Positional line comparison** — both files are split into lines and compared index by index. Each position becomes a `Line` variant (`Same`, `Changed`, `Added`, `Removed`); the run collapses to an `Outcome` of `Same` or `Changed`.
- **Outcome as exit code** — `main` maps the `Outcome` to a colored message and a process exit code (`0`/`1`), and prints errors to stderr with code `2`.
- **Errors as strings** — for now failures are surfaced as `Result<_, String>`; a dedicated error type is a candidate for later.

## Roadmap

Known gaps, roughly in priority order:

- [ ] **Render the diff per line** — the `diffs` vec is already computed but thrown away; use it to print each line colored by its `Line` variant (added / removed / changed / same).
- [ ] **Real diff algorithm** — replace the positional line-by-line comparison with a longest-common-subsequence (dynamic programming) approach so inserted/removed lines don't misalign everything after them.
- [ ] **Tests** — unit tests for `compare_files` and end-to-end tests for the binary against fixture files.
- [ ] **Cleanup** — remove the commented-out scratch code, drop the `src`-relative path quirk in favor of plain paths, and introduce a proper error type.

## Development

```sh
cargo test    # run tests (once added)
cargo clippy  # lint
cargo fmt     # format
```
