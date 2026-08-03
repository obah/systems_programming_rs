# BYTES

A round-trip exercise in binary serialization, built as part of my Rust and systems engineering learning journey. It writes a `(u32, i64, f64)` tuple into a byte buffer and reads it back, in both little- and big-endian, asserting the exact bytes along the way.

> ℹ️ **The program ends in a panic on purpose.** The final assertion compares a tuple containing `f64::NAN` and fails, because `NaN != NaN` by IEEE-754. That's part of the lesson, not a bug — see [Roadmap](#roadmap).

## What I learnt

- **Binary data handling** — how a typed value becomes a sequence of bytes and back, and why the byte count is fixed by the type (`u32` → 4, `i64` → 8, `f64` → 8, so 20 bytes total for the tuple).
- **`Cursor`** — wrapping a `Vec<u8>` or `&[u8]` to get `Read`/`Write` (and `Seek`) over an in-memory buffer, with a position that advances as you go, so no manual offset bookkeeping.
- **Endianness** — the same value producing different byte orders, and how two's complement and IEEE-754 bit patterns look once laid out in memory.
- **`byteorder`** — writing endianness as a _type parameter_ (`E: ByteOrder`) so one function covers both orders, instead of a copy per byte order.

## Usage

It's a single binary with no arguments — run it and watch the assertions.

```sh
cargo run -p bytes
```

Output:

```text
special nums are (4294967295,-9223372036854775808,NaN) and in little endian form: [255, 255, 255, 255, z0, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 248, 127]

thread 'main' panicked at serialization/bytes/src/main.rs:57:5:
assertion `left == right` failed
  left: (4294967295, -9223372036854775808, NaN)
 right: (4294967295, -9223372036854775808, NaN)
```

Note the two sides print identically and still compare unequal. Every earlier assertion — the exact little-endian bytes, both round trips, the cursor position, and little-endian ≠ big-endian — passed before this one.

## Implementation

One binary crate, two functions and a `main` full of assertions:

```text
src/
└── main.rs   # write_endian / read_endian, both generic over E: ByteOrder
```

The little-endian layout of `(12, -12, 12.12)`, asserted byte for byte:

```text
0x0C 0x00 0x00 0x00                                 u32 12
0xF4 0xFF 0xFF 0xFF 0xFF 0xFF 0xFF 0xFF             i64 -12, two's complement
0x3D 0x0A 0xD7 0xA3 0x70 0x3D 0x28 0x40             f64 12.12, IEEE-754 0x40283D70A3D70A3D reversed
```

Some design details:

- **Endianness as a type parameter** — `write_endian<E: ByteOrder>` and `read_endian<E: ByteOrder>` are each written once and instantiated at `LittleEndian` and `BigEndian`. The byte order is chosen at the call site, resolved at compile time, and the bodies never mention it.
- **`Cursor` on both sides** — writing uses `Cursor<Vec<u8>>` (the vec grows), reading uses `Cursor<&[u8]>` (a borrowed view). `into_inner()` unwraps the finished buffer; `position()` reports how far the reads got, which is why `read_endian` returns it alongside the values — it should equal 20, and asserting that proves nothing was silently skipped or over-read.
- **Assertions instead of tests** — the round trips run in `main` rather than a `#[cfg(test)]` module, because failing loudly _when run_ is the point of the exercise. `assert_ne!(le, be)` is the one that actually proves endianness matters: identical values, different bytes.
- **Comparing values, not bytes** — the round trip asserts on the decoded tuple, not on the buffer, so it catches a reader that mis-parses bytes it read correctly. The exact-byte assertion on `expected_le` covers the other direction.
- **Old stdlib version kept below** — the commented-out block at the bottom of `main.rs` is the earlier attempt: a `ValType`/`Kind` enum pair with `to_le_bytes`/`from_le_bytes`, plus a `byteorder` variant of each. Left in deliberately as a before/after: it needed one match arm per type per direction, which is what the generic `E: ByteOrder` version collapses.

## Roadmap

- [ ] **The `NaN` assertion** (`main.rs:57`) — left failing on purpose, with the fix noted in the comment above it. `f64::NAN == f64::NAN` is `false` under IEEE-754, so `assert_eq!` on a tuple containing it can never pass, however correct the serialization is. The bytes _did_ survive intact — `[0, 0, 0, 0, 0, 0, 248, 127]` is `0x7FF8000000000000`, a quiet NaN. Fix by comparing `to_bits()` (bit-exact, treats NaN as equal to itself) or by checking `is_nan()` on both sides and comparing the other two fields normally.

## Development

```sh
cargo clippy  # lint
cargo fmt     # format
```
