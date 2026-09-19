# RESP2 PARSER

A parser, encoder and TCP server for RESP2 — the wire protocol Redis speaks — built as part of my Rust and systems engineering learning journey. Real `redis-cli` talks to it. The framing problem is the same one [tcp_codec](../tcp_codec/README.md) solves with a 4-byte length prefix, but RESP is text-shaped and self-describing, so the parser has to work out how long a frame is *while* reading it.

> ℹ️ **This is a server.** It parses commands coming from a client and sends replies back. Every reply is currently a stubbed `+OK` — see [Roadmap](#roadmap).

## The protocol in one screen

Every frame starts with one type byte and ends with `\r\n`:

| Byte | Type          | Example              | Frame                        |
| ---- | ------------- | -------------------- | ---------------------------- |
| `+`  | Simple string | `+OK\r\n`            | `SimpleString("OK")`         |
| `-`  | Error         | `-ERR unknown\r\n`   | `Error("ERR unknown")`       |
| `:`  | Integer       | `:42\r\n`            | `Integer(42)`                |
| `$`  | Bulk string   | `$5\r\nhello\r\n`    | `BulkString(b"hello")`       |
| `*`  | Array         | `*1\r\n:1\r\n`       | `Array([Integer(1)])`        |

`$-1\r\n` and `*-1\r\n` are the two null spellings. Arrays nest, which is what makes the parser recursive.

## What I learnt

### Two kinds of length, and only one of them is safe

The five types split into two families, and the difference decides everything downstream:

- **Line-terminated** (`+`, `-`, `:`) — the frame ends at the first `\r\n`. You find the end by scanning. The payload therefore *cannot contain* `\r` or `\n`, or it would forge a frame boundary.
- **Length-prefixed** (`$`, `*`) — a count comes first, so the reader knows exactly how far to jump. Nothing needs escaping and the payload is fully binary-safe.

This is why `SET key "a\r\nb"` works at all. The bulk string says `$4`, the parser jumps 4 bytes, and the CRLF sitting *inside* the payload is never scanned for. A test pins exactly that (`a_command_round_trips_back_into_bulk_string_frames`), because a parser that searched for the terminator instead of trusting the length would pass every other test and fail this one.

The encoder has the mirror obligation. Bulk strings need no escaping; simple strings and errors do, so `write_frame` carries a `debug_assert!(!s.contains(['\r', '\n']))` on those two arms. It's an assert rather than a `Result` because that content comes from my own code, not off the wire.

### `Ok(None)` means incomplete, never "done" and never "empty"

`parse(&[u8]) -> Result<Option<(Frame, usize)>, Error>` is three states wearing two:

| Return                  | Means                                                              |
| ----------------------- | ------------------------------------------------------------------ |
| `Ok(Some((frame, n)))`  | A whole frame, and it took exactly `n` bytes.                       |
| `Ok(None)`              | **Incomplete** — these bytes are a *prefix* of a frame. Read more.  |
| `Err(_)`                | Not RESP at all. The stream is unrecoverable.                       |

Every byte short of a complete frame must be `None`, including the ones deep inside a nested array. The test that actually enforces this loops over *every* truncation of a two-element array:

```rust
let full = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
for n in 0..full.len() {
    assert_eq!(parse(&full[..n]).unwrap(), None, "{n}");
}
```

That loop is the one that catches the interesting bug: an inner frame returning `None` has to propagate all the way out of `read_array` and discard the items already collected, rather than handing back a half-built array.

### The consumed count is the whole API

`parse` returns how many bytes it used and never touches the caller's buffer. That one decision is what keeps it a pure function over a `&[u8]` — trivially testable with byte-string literals, no state to reset between cases — and it pushes buffer management up into `Decoder`, where it belongs.

`Decoder` then owns the contract in the other direction:

```rust
let Some((frame, consumed)) = parse(&self.buf[self.cursor..])? else {
    return Ok(None);   // cursor NOT advanced
};
self.cursor += consumed;
```

The `let-else` returns *before* the `+=`, so an incomplete frame is re-read from the same offset after the next `feed`. Advancing on the `None` path would silently corrupt every frame after it — the same trap `decode` has in [tcp_codec](../tcp_codec/README.md#the-decoders-three-state-return), reached from the opposite direction.

### Why the borrow checker allows `self.cursor += n`

`parse(&self.buf[..])` borrows `buf` immutably, yet the next line takes `&mut self`. That compiles only because `Frame` owns its data — `String` and `Vec<u8>`, not `&str` and `&[u8]` — so the returned value carries no lifetime tied to the buffer and the borrow ends at the `let`.

A zero-copy `Frame<'a>` holding slices would be faster and would not compile here: the frame would still be borrowing `buf` at the moment the cursor tries to move. Getting that version to work means `Bytes`-style refcounted slices, which is exactly what tcp_codec uses. Owning the bytes is the cost of the simple design.

### The buffer has to be compacted, or it never shrinks

`feed` appends and the cursor only moves forward, so a long-lived connection would hold every byte it has ever received. Past 4 KiB of consumed bytes, `next_frame` drains:

```rust
if self.cursor >= COMPACT_AT {
    self.buf.drain(..self.cursor);
    self.cursor = 0;
}
```

Two details. It compacts only after a *successful* frame, because `drain` memmoves the unconsumed tail and doing that on every partial read would copy the same pending bytes over and over. And the threshold counts *consumed* bytes, not buffer length — a single 3 KiB bulk string in flight isn't waste, it's a frame still being assembled.

### `?` runs `From::from` on the way out

The first version of this parser used `type Error = String` and a `.map_err(|_| "...")` closure at every fallible call. Writing two impls deleted all of them:

```rust
impl From<Utf8Error> for Error { fn from(_: Utf8Error) -> Self { Self::InvalidUtf8 } }
impl From<ParseIntError> for Error { fn from(_: ParseIntError) -> Self { Self::InvalidInteger } }
```

`?` doesn't just early-return — it converts first. So `std::str::from_utf8(bytes)?` and `s.parse::<usize>()?` now produce an `Error` with no closure in sight. `Debug` (derived) is for me, `Display` is for whoever reads the message, and the one-line `impl std::error::Error for Error {}` is what makes it a real error type.

### The length field is attacker-controlled

`$999999999999\r\n` is a client asking the server to trust a number. Two guards, for two different failure modes:

- **`MAX_BULK_LEN` (512 MiB, Redis's own `proto-max-bulk-len`)** is checked before the length is used for anything.
- **`read_array` never calls `Vec::with_capacity(count)`.** `*999999999\r\n` is four bytes of input that would otherwise ask for a gigabyte of allocation. Growing the `Vec` as frames actually arrive costs a few reallocations and cannot be weaponised.

The second one is the subtler of the two: the obvious "optimisation" is the vulnerability.

### What redis-cli actually sends

Captured by running this server on `:6380` and pointing a real `redis-cli 8.10.2` at it:

```text
<- *3\r\n$3\r\nSET\r\n$5\r\nmykey\r\n$11\r\nhello world\r\n     SET mykey "hello world"
<- *2\r\n$5\r\nHELLO\r\n$1\r\n3\r\n                            redis-cli -3
<- *2\r\n$7\r\nCOMMAND\r\n$4\r\nDOCS\r\n                       interactive mode, before you type
```

Four things I only knew for sure after watching the bytes:

- **A command is always an array of bulk strings.** Never a simple string, never an integer. That's why `encode_command(&[&[u8]]) -> Vec<u8>` needs no `Frame` — the shape is fixed.
- **Quoting is not escaping.** `"hello world"` arrives as `$11` with the space inside the payload; the shell's quotes did their job before the bytes existed.
- **RESP3 is negotiated inside RESP2.** `HELLO 3` is an ordinary array frame, so a RESP2 parser reads the handshake perfectly well. It's the *reply* that would have to change.
- **Interactive mode opens with `COMMAND DOCS`**, before a single keystroke. Answering `+OK` instead of the command table just means no tab-completion hints.

One thing the capture also proves is missing: `printf 'PING\r\n' | nc` gets `-ERR protocol error: unknown frame type byte 0x50`. Real Redis accepts that legacy *inline command* form. `redis-cli` never sends it, so this only bites when poking the server with telnet or nc.

### A protocol error has to close the connection

There's no resync point in a byte stream. Once `parse` returns `Err`, the cursor is sitting at a byte that isn't a frame boundary and no amount of reading fixes it. So `handle` writes the error frame and returns, which drops the `TcpStream` and hangs up — the same thing Redis does.

That behaviour is testable precisely *because* the socket closes: `a_protocol_error_is_answered_then_the_socket_closes` calls `read_to_end`, which can only return once the server is gone.

## Usage

The binary is a server. It takes one optional argument, the address to bind (default `127.0.0.1:6379`, which will collide with a real `redis-server`):

```sh
cargo run -p resp2_parser -- 127.0.0.1:6380
```

Then, in another terminal:

```sh
redis-cli -p 6380 PING
redis-cli -p 6380 SET mykey "hello world"
redis-cli -p 6380 GET mykey
```

Every command answers `OK` for now. The server side prints the capture:

```text
listening on 127.0.0.1:6380
-- open  Some(127.0.0.1:54603)
<- *3\r\n$3\r\nSET\r\n$5\r\nmykey\r\n$11\r\nhello world\r\n
   parsed Array([BulkString([83, 69, 84]), BulkString([109, 121, 107, 101, 121]), BulkString([104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100])])
-> +OK\r\n
-- close Some(127.0.0.1:54603)
```

`<-` is raw bytes off the wire, `->` is what went back. Both are printed with `[u8]::escape_ascii()`, so `\r\n` shows as escapes and arbitrary bytes stay readable without a hexdump.

### Watching the decoder do its job

A command split across two TCP writes proves the frame is reassembled rather than parsed per-read:

```text
<- *3\r\n$3\r\nSET\r\n$3\r\nfo        ← first read ends mid-bulk-string
<- o\r\n$3\r\nbar\r\n
   parsed Array([BulkString([83, 69, 84]), BulkString([102, 111, 111]), BulkString([98, 97, 114])])
```

The first read returned `Ok(None)`, the cursor stayed put, and the frame only appeared after the second.

## Implementation

A library crate with a thin server binary on top:

```text
src/
├── main.rs      # entry point: reads the bind address, runs the server, sets the exit code
├── lib.rs       # crate root: declares modules and the shared Error type
├── frame.rs     # domain: the Frame enum, one variant per RESP2 type
├── parser.rs    # bytes -> (Frame, consumed) — pure, no buffer, no I/O
├── decoder.rs   # buffer management: feed bytes, pull whole frames, compact
├── encoder.rs   # Frame -> bytes (replies), and args -> bytes (commands)
└── server.rs    # TCP listener: read, decode, log, reply
```

The dependency arrow points one way — `server` → `decoder` → `parser` → `frame` — and every layer is testable without the one above it. `parser` is `pub(crate)`: `parse` is an implementation detail of `Decoder`, not API.

Parsing `*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n`:

```text
*2\r\n              read_array: count = 2, consumed = 4
  $5\r\nhello\r\n   recurse -> BulkString("hello"), consumed += 11
  $5\r\nworld\r\n   recurse -> BulkString("world"), consumed += 11
                    Array([...]), 26 bytes
```

Some design details:

- **`Frame` owns its bytes** — see [why the borrow checker allows it](#why-the-borrow-checker-allows-selfcursor--n). `BulkString(Vec<u8>)` not `String`, because a Redis value is arbitrary bytes; the type byte and the length line are the only parts guaranteed to be UTF-8.
- **Nulls are folded** — the parser maps both `$-1` and `*-1` to `Frame::Null`, and the encoder always emits `$-1`. So `parse(encode(f)) == f` holds for every frame (the round-trip test asserts it), while `encode(parse(b)) == b` does not.
- **Two encoders, one writer** — `encode(&Frame)` for replies and `encode_command(&[&[u8]])` for commands both sit on `write_line` and `write_bulk`, so the framing rules exist in exactly one place.
- **The error enum is the crate root's** — `Error` lives in `lib.rs` next to its `From`, `Display` and `std::error::Error` impls, mirroring `TodoError` in [todo](../../cli/todo/README.md). I/O errors are deliberately *not* a variant: `server::run` returns `std::io::Result`, so a dead socket and a malformed frame stay separate problems.
- **`escape_ascii()` is the whole logging layer** — stdlib since 1.60, no hexdump helper, no `from_utf8_lossy` that mangles binary keys.
- **11 tests, colocated** — `parser.rs` holds the frame and truncation cases, `encoder.rs` the two round trips, `decoder.rs` the chunk-splitting and compaction ones, `server.rs` two that open a real socket on `127.0.0.1:0` (port 0 lets the OS pick, so tests never collide).

## Roadmap

Known gaps, roughly in priority order:

- [ ] **Actually execute commands.** Every reply is `+OK`, so `GET` lies. A `HashMap<Vec<u8>, Vec<u8>>` behind `PING`/`SET`/`GET`/`DEL` makes this a server `redis-cli` can be used against — and it's where `Frame::Null` (missing key) and `Frame::Integer` (delete count) finally get emitted for real.
- [ ] **One thread per connection.** `server::run` handles one client at a time; a second `redis-cli` waits for the first to hang up. `thread::spawn(move || handle(stream))` fixes it, and the moment shared state arrives it needs an `Arc<Mutex<_>>` around the map.
- [ ] **Cap the nesting depth.** `read_array` recurses once per level, so `*1\r\n` repeated a few hundred thousand times is a stack overflow — a crash, not an error. A depth counter threaded through `parse` turns it into a rejected frame.
- [ ] **Remember partial parse state.** An incomplete frame is re-parsed from its start on every `feed`, which is O(n²) on a byte-at-a-time stream. Only worth it once profiling says so.
- [ ] **Inline commands** — accept `PING\r\n` from telnet/nc, as real Redis does: when the first byte isn't a known type byte, split the line on spaces into bulk strings instead of erroring.
- [ ] **`encode_into(&Frame, &mut Vec<u8>)`** — append into a caller's buffer so a batch of replies is one `write_all` instead of an allocation per frame.
- [ ] **RESP3** — answer `HELLO 3` properly and add the new types (maps `%`, sets `~`, doubles `,`, booleans `#`, nulls `_`). The parser already reads the handshake; nothing yet replies to it.
- [ ] **A `type` alias for `read_bulk_string`'s return** — `Result<Option<(Option<&[u8]>, usize)>, Error>` trips clippy's `type_complexity`. Only worth doing if that signature spreads.

## Development

```sh
cargo test -p resp2_parser   # 11 tests: parser, encoder round trips, decoder, two over a real socket
cargo clippy                 # lint
cargo fmt                    # format
```

Unit tests live in `#[cfg(test)]` modules inside each source file. The two in `server.rs` bind an ephemeral port and drive `handle` over a real `TcpStream`, so the read loop, the reply path and the hang-up-on-error path are all exercised without a mock.
