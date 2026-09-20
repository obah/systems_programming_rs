# Systems Programming in Rust

A collection of my systems programming learning projects, built in Rust as one Cargo workspace. Each project lives in its own folder with its own README.

```sh
git clone https://github.com/obah/systems_programming_rs.git
cd systems_programming_rs
cargo run -p <package> -- <args>
```

## Projects

### CLI tools

| Project                                            | Status | Description                                                                                                      |
| -------------------------------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------- |
| [todo](cli/todo/)                                   | ✅ Done | A task manager for the terminal — add, list, rename, complete and delete tasks, persisted to a plain text file.   |
| [calc_mini_interpreter](cli/calc_mini_interpreter/) | 🚧 WIP  | An arithmetic REPL — a lexer → parser → evaluator pipeline in miniature. Variables exist in the evaluator but aren't reachable from input yet. |
| [diff](cli/diff/)                                   | 🚧 WIP  | Compares two files line by line. Reports *whether* they differ; per-line output and an LCS algorithm are still to come. |

### Serialization & binary formats

| Project                                                  | Status | Description                                                                                                      |
| --------------------------------------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------- |
| [bit_manipulation](serialization/bit_manipulation/)        | ✅ Done | Packing a record header (4-bit tag, 1-bit flag, 11-bit length) into a single `u16` and reading it back out.       |
| [type_layout](serialization/type_layout/)                  | ✅ Done | How Rust lays a struct out in memory — the same two fields under `repr(Rust)`, `repr(C)`, `repr(packed)` and `repr(packed(2))`. Fails to build on purpose (`E0793`). |
| [bytes](serialization/bytes/)                              | ✅ Done | Round-tripping a `(u32, i64, f64)` through a byte buffer in both endiannesses with `Cursor` and `byteorder`. Ends in a deliberate `NaN != NaN` panic. |
| [serde_serialization](serialization/serde_serialization/)  | ✅ Done | Serde's data model — a derived `Point`, and a hand-written `Deserialize` with its own `Visitor` for a `Message` enum. |
| [tcp_codec](serialization/tcp_codec/)                      | ✅ Done | Length-prefixed framing over TCP with Tokio — a 4-byte big-endian prefix turned into a `Stream`/`Sink` via `Framed`, plus Tokio-tutorial working notes. |
| [resp2_parser](serialization/resp2_parser/)                | 🚧 WIP  | A parser, encoder and TCP server for RESP2, the protocol Redis speaks — real `redis-cli` talks to it. Replies are still stubbed `+OK`. |

### Concurrency

| Project                                        | Status | Description                                                                                     |
| ----------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------- |
| [rust_practice](concurrency/rust_practice/)     | 🚧 WIP  | Lessons on threads, channels, shared state and atomics, with exercises alongside. `cargo run` hangs on the atomics lesson — a `todo!()` worker, which is its own lesson. |

## Development

```sh
cargo test -p <package>   # test one project
cargo test                # test everything
cargo clippy
cargo fmt
```
