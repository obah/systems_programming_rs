# TCP CODEC

A length-prefixed framing codec over TCP, built as part of my Rust and systems engineering learning journey. Each payload is preceded by a 4-byte big-endian length, which is what turns Tokio's byte stream back into discrete messages — and `Framed` then turns the socket into a `Stream` of frames and a `Sink` for them.

> ℹ️ **The crate holds two things.** `lib.rs` + `main.rs` + `bin/client.rs` are the codec and its echo server. `bin/server.rs`, `bin/tokio_tut_client.rs` and `examples/hello-redis.rs` are working notes from the Tokio tutorial (mini-redis, shared state, channels) — kept because they're where the runtime concepts below were actually learnt.

## What I learnt

### What the Tokio runtime actually is

`#[tokio::main]` is not magic and not a runtime. It's a macro that rewrites `async fn main` into an ordinary sync `main` that builds a runtime and calls `block_on` on the body. With `features = ["full"]` that's the multi-threaded work-stealing scheduler.

The pieces that made it click:

- **Futures are lazy.** Calling an `async fn` runs none of it — it builds a state machine. Nothing happens until something polls it, which is why an un-`await`ed future warns instead of working.
- **`.await` yields the thread, it doesn't block it.** When a read isn't ready the task hands the worker thread back to the scheduler, which runs somebody else's task on it. That's the whole trick — one thread serves many connections because they spend most of their time waiting.
- **`tokio::spawn` creates a task, not a thread.** Tasks are cheap (an allocation, not an 8 MiB stack), so one per connection is the normal shape. The future must be `Send + 'static`, which is what forces `async move` and forces `db.clone()` *before* the spawn rather than inside it.
- **The accept loop must stay empty.** `listener.accept().await` in the loop, everything else spawned:

  ```rust
  loop {
      let (socket, addr) = listener.accept().await?;
      tokio::spawn(async move { /* the whole connection lives here */ });
  }
  ```

  Do the read/echo inline and the server still compiles, still works, and serves exactly one client at a time. The bug is invisible until the second connection.

- **Blocking inside a task blocks the worker.** A `std::sync::Mutex` is still the right choice in `bin/server.rs` — but only because the guard is dropped at the end of the match arm and never crosses an `.await`. Hold a std guard across an await point and the future stops being `Send`; hold a lock across an await at all and you've serialized every task behind it.
- **`mpsc::channel(32)` is bounded, and the receiver ends itself.** `rx.recv()` returns `None` once *every* sender has been dropped — that's what terminates the `while let` in `bin/tokio_tut_client.rs`, not any explicit close.

### TCP gives you a byte stream, not messages

This is the one the whole project exists for. TCP guarantees that bytes arrive, in order, exactly once. It guarantees **nothing** about boundaries. One `send` is not one `read`: the kernel, the MTU and Nagle's algorithm are all free to split one message across two reads or coalesce three messages into one.

So framing is the application's job. Three options: a delimiter (newline — cheap until the payload contains one), a fixed size (only for fixed records), or a **length prefix**, which is what this uses. Write the length, then the bytes; the reader knows exactly how much to wait for and payloads stay fully binary-safe.

The other half of that: **the length prefix is attacker-controlled input.** A 4-byte packet claiming `0xFFFFFFFF` asks the server to reserve 4 GiB. Checking `length > MAX` before touching the buffer isn't a nicety, it's the reason the check exists.

### The decoder's three-state return

`decode(&mut BytesMut) -> Result<Option<Bytes>>` looks like two states and is actually three:

| Return            | Means                                                                  |
| ----------------- | ---------------------------------------------------------------------- |
| `Ok(Some(frame))` | Took a whole frame; the buffer has been advanced past it.               |
| `Ok(None)`        | **Incomplete** — not enough bytes yet. Leave the buffer alone, call me again. |
| `Err(_)`          | The stream is unrecoverable. `Framed` stops.                            |

`Ok(None)` never means "finished". `Framed` owns the buffer and keeps it between calls, so consuming bytes on a `None` path silently corrupts every frame after it. There are two `None` paths here — fewer than 4 bytes (no header yet) and header read but body short — and neither one touches the buffer.

`decode_eof` came free. The default implementation calls `decode`, and if that returns `None` with bytes still sitting in the buffer, it errors:

```text
⚠️ Frame decoding error: bytes remaining on stream
```

Which is exactly right: a header promising 10 bytes followed by 3 bytes and a close is a truncated frame, not a clean disconnect. A clean disconnect leaves the buffer empty and `next()` simply returns `None`.

### `BytesMut` splitting is not copying

`bytes` is the reason the decode path allocates nothing per frame:

- `src.advance(4)` drops the header off the front by moving a pointer.
- `src.split_to(length)` detaches the frame as its own `BytesMut` — same allocation, refcounted, no memcpy.
- `.freeze()` converts that to an immutable `Bytes`, which is why it's cheap to clone and hand to another task.
- `reserve` is a *hint*: telling the buffer up front how much more is coming lets it grow once instead of on every partial read.

The test leans on the inverse, `unsplit`, to glue a buffer back together and feed the decoder its missing byte.

### `Framed`, and where the methods come from

`Framed::new(socket, codec)` takes anything `AsyncRead + AsyncWrite` and a codec, and produces one object that is both a `Stream<Item = Result<Bytes, io::Error>>` and a `Sink<Bytes>`. From there the server is just:

```rust
while let Some(frame_res) = framed.next().await { ... framed.send(bytes).await ... }
```

`.next()` and `.send()` are not inherent methods — they come from `futures::{StreamExt, SinkExt}`. Without those two imports the methods simply don't exist, which produces a confusing "no method named `next`" on a type that obviously is a stream.

One asymmetry worth noticing in the traits: `Encoder<Item>` takes its item as a **generic parameter**, while `Decoder` declares `type Item` as an **associated type**. So a single codec can encode several different types into the same wire format, but it decodes into exactly one.

## Usage

Four binaries live in this package, so `cargo run -p tcp_codec` on its own won't do:

```text
error: `cargo run` could not determine which binary to run. Use the `--bin` option...
available binaries: client, server, tcp_codec, tokio_tut_client
```

### The echo server and its client

Terminal 1:

```sh
cargo run -p tcp_codec --bin tcp_codec
```

Terminal 2:

```sh
cargo run -p tcp_codec --bin client
```

Client output:

```text
Connected to server!
Sending: b"Hello, Length-Prefixed World!"
Received Echo: b"Hello, Length-Prefixed World!"
```

Server output:

```text
Server listening on 127.0.0.1:8080
New connection from: 127.0.0.1:58230
From 127.0.0.1:58230, received: b"Hello, Length-Prefixed World!"
Connection closed
```

### The Tokio tutorial pieces

A mini-redis server with shared state, and a client that sets and gets one key:

```sh
cargo run -p tcp_codec --bin server         # listens on 127.0.0.1:6379
cargo run -p tcp_codec --example hello-redis
```

```text
got value from the server: result=Some(b"world")
```

And two `mpsc` senders feeding one receiver:

```sh
cargo run -p tcp_codec --bin tokio_tut_client
```

```text
GOT: sending from first handle
GOT: sending from second handle
```

## Implementation

One library plus four binaries and an example:

```text
src/
├── lib.rs                    # LengthPrefixedCodec: Encoder + Decoder, and the round-trip test
├── main.rs                   # echo server on :8080 — Framed + the codec, one task per connection
└── bin/
    ├── client.rs             # connects to :8080, sends one frame, prints the echo
    ├── server.rs             # tutorial: mini-redis server on :6379, Arc<Mutex<HashMap>> state
    └── tokio_tut_client.rs   # tutorial: bounded mpsc, two cloned senders
examples/
└── hello-redis.rs            # tutorial: mini-redis client, set then get
```

The wire format:

```text
 ┌──────────────────────────┬──────────────────────────────┐
 │  length: u32 big-endian  │  payload: `length` bytes     │
 │         4 bytes          │                              │
 └──────────────────────────┴──────────────────────────────┘
```

`Bytes::from("Hello, Length-Prefixed World!")` — 29 bytes of payload, 33 on the wire:

```text
00 00 00 1D                          length = 29
48 65 6C 6C 6F 2C 20 ... 21          "Hello, Length-Prefixed World!"
```

Some design details:

- **Big-endian, because the wire is** — `to_be_bytes` / `from_be_bytes`. Correctness only needs both ends to agree; big-endian is the network convention and has the side benefit that a hex dump reads left to right.
- **`MAX` is checked on both sides, for different reasons** — 4 MiB. On `encode` it catches your own bug before it reaches the socket. On `decode` it's a real defence, because that number came off the wire; the check sits *before* the `reserve` it protects.
- **Nothing is consumed on the `None` path** — both early returns leave `src` exactly as they found it. See [the decoder's three-state return](#the-decoders-three-state-return).
- **Frame extraction is copy-free** — `src.advance(4)` then `src.split_to(length).freeze()`, both pointer work over the buffer `Framed` already owns.
- **The codec is stateless** — `LengthPrefixedCodec {}` carries no fields; `&mut self` exists in the trait for codecs that need to remember where they were. A stateful `Head`/`Body` machine would avoid re-reading the 4-byte header on every partial call, but re-parsing four bytes is cheaper than the branch that would avoid it.
- **One test covering the two behaviours that matter** — `roundtrip_and_partial` encodes a frame, hands the decoder the buffer one byte short and asserts `None`, then `unsplit`s the missing byte back and asserts the exact payload. Two details are deliberate: the payload is `b"\xff\xfe not utf8"`, so a codec that quietly assumes text fails; and the final `assert!(partial.is_empty())` is what proves the decoder consumed the header and the frame and nothing beyond.

## Roadmap

- [ ] **Read `tokio_util::codec::LengthDelimitedCodec`** — it ships exactly this, with configurable prefix width, endianness, max frame size and header offsets. The hand-written version is the lesson; the built-in one is the answer in real code, and reading the two side by side shows which knobs a production framing layer actually needs.
- [ ] **Tests for the rejection paths** — oversize `length` on decode, oversize item on encode, and the truncated-frame-at-EOF case. That last one is currently only verified by hand (the `bytes remaining on stream` output above), which means nothing catches a regression in it.
- [ ] **`LengthPrefixedCodec {}` → `LengthPrefixedCodec;`** — a unit struct, so every construction site loses the empty braces.
- [ ] **Generic encoder** — `impl<T: AsRef<[u8]>> Encoder<T>` would let callers send `&str` or `Vec<u8>` without wrapping in `Bytes` first. Purely additive, since `Encoder`'s item is a type parameter rather than an associated type.
- [ ] **Timeouts and graceful shutdown** — a client that connects and sends nothing holds a task open forever. `tokio::time::timeout` around `next()` handles the idle case; `tokio::signal::ctrl_c` in a `select!` handles the shutdown one.
- [ ] **The dead `Command` enum** in `bin/tokio_tut_client.rs` warns on every build. Either finish the tutorial step that routes commands over the channel, or delete it.

## Development

```sh
cargo test -p tcp_codec   # roundtrip_and_partial
cargo clippy              # lint
cargo fmt                 # format
```
