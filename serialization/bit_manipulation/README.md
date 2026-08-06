# BIT MANIPULATION

Packing a record header — a 4-bit type tag, a 1-bit compressed flag, and an 11-bit length — into a single `u16` and reading it back out, built as part of my Rust and systems engineering learning journey. The fields add up to exactly 16 bits, which turns out to shape the whole design.

## What I learnt

### The four bitwise operators

| Op    | Rust | What it's for                                                            |
| ----- | ---- | ------------------------------------------------------------------------ |
| AND   | `&`  | **Masking** — keep the bits you want, zero the rest. `x & 0xF` reads the low nibble. |
| OR    | `\|` | **Merging** — drop a field into a hole. Only safe when the hole is already zero. |
| XOR   | `^`  | **Toggling** — `x ^ (1 << 4)` flips the compressed flag without reading it first. |
| NOT   | `!`  | **Inverting** — `!0u8` is `255`, all bits set.                            |

Rust spells bitwise NOT `!`, not `~` as in C. There is no `~` operator at all. `!` is overloaded through the `Not` trait: on `bool` it's logical negation, on integers it's a bit flip.

`^` doesn't appear in this code — with `|`-merging into known-zero space there's nothing to toggle — but it's the operator for flipping a flag in an already-packed value.

### Shifting and masking are two halves of one move

The rule that made it click: **to read bits at the LSB end, mask; to read bits further up, shift down first, then mask.**

```rust
packed & 0xF                     // tag: already at the bottom, just mask
(packed >> 4) & 0b1              // compressed: shift it down to bit 0, then mask
(packed >> 5) & 0b111_1111_1111  // length: same move, wider mask
```

Writing is the mirror image — shift *up* into position, then `|` them together:

```rust
tag as u16 | (compressed as u16) << 4 | length << 5
```

Two things that were not obvious:

- **`bool as u16` is `0` or `1`.** No branch needed to pack a flag; the cast already produces exactly the one bit.
- **`as` on a narrowing cast truncates, it doesn't error.** `packed as u8` silently discards the top 8 bits. In `unpack` that's used deliberately — it does half the masking work for free — but it's the same silent truncation that made the newtypes worth having.

### The `1 << n` family

| Expression         | Value               | Use                                             |
| ------------------ | ------------------- | ----------------------------------------------- |
| `1 << n`           | 2ⁿ, one bit set     | Build a single-bit flag: `1 << 4` is the compressed bit. |
| `(1 << n) - 1`     | n low bits set      | Build an n-bit mask. `(1 << 4) - 1` is `0xF`.   |
| `x & ((1 << n) - 1)` | x's low n bits    | Truncate to n bits — cheap `x % 2ⁿ`.            |
| `x & (x - 1)`      | x with its lowest set bit cleared | `x != 0 && x & (x - 1) == 0` tests power-of-two. |

`(1 << n) - 1` is the one this project leans on: subtracting 1 from a lone set bit borrows all the way down and fills everything below it. Both newtypes derive their bound that way rather than hardcoding `0xF` and `0x7FF`.

There's a trap in it. `1 << n` overflows when `n` equals the type's width, so `(1 << 8) - 1` on a `u8` doesn't give `255` — it doesn't compile at all, because const evaluation catches it:

```text
error[E0080]: attempt to shift left by `8_u32`, which would overflow
```

Good failure mode as long as it's a `const`. In a runtime expression the same shift panics in debug and wraps in release.

### Precedence, where Rust and C disagree

Rust orders these `<<` `>>` → `&` → `^` → `|` → `==`, so the shifts and masks in this code need no parentheses to mean what they look like:

```rust
packed >> 4 & 0b1                    // (packed >> 4) & 0b1
tag | (compressed as u16) << 4       // tag | ((compressed as u16) << 4)
```

The one to remember is `&` binding **tighter** than `==` — the opposite of C, where `x & mask == 0` silently parses as `x & (mask == 0)`. In Rust `x & (x - 1) == 0` compiles and means what you'd hope. Parenthesising anyway costs nothing and survives the reader who learnt the C order.

### Making the invariant unrepresentable

The first version validated at runtime — `pack` returned `Result` and checked `type_tag > TAG_MAX` on every call. Replacing the raw `u8`/`u16` with `Tag` and `Length` newtypes moved that check to construction, so `pack` now takes values that are already known to fit and returns a plain `u16` with no error path.

The part that actually enforces it is the **module boundary**, not the newtype. Rust field privacy is scoped to the defining *module*, and a single-file crate is one module — so a bare `struct Tag(u8)` sitting next to `main` can still be built as `Tag(255)`, or mutated afterwards with `t.0 = 200`, with no warning. Wrapping each type in its own `mod` makes `new()` the only way in:

```text
error[E0423]: cannot initialize a tuple struct which contains private fields
error[E0616]: field `0` of struct `Tag` is private
```

Compile errors, not runtime ones — the difference between a bad state being *rejected* and being *unrepresentable*. Worth noting that rustc's own help text on E0423 suggests `pub struct Tag(pub u8)`, which would undo the whole thing.

This is also why `pack` dropped its `& 0xF` masks: with the invariant in the type there's nothing to truncate. That only holds because of the module walls — without them, an out-of-range tag would silently spill into the neighbouring fields, and the masks were the only thing containing the damage.

### Round-tripping an exhaustive space

4 + 1 + 11 = 16 bits with nothing left over, so *every* `u16` is a valid header and the entire state space is 65,536 values. Small enough to walk completely, which makes a random fuzzer both more code and strictly weaker.

It also means one direction suffices. `unpack` is total over `u16`, so proving `pack(unpack(x)) == x` for all `x` forces the two to be exact inverses — the other direction comes free by counting.

What a round trip *can't* catch is `pack` and `unpack` both using the wrong shift, since the errors cancel out. That needs one golden value pinning the real layout.

## Usage

A single binary with no arguments — it packs one header, unpacks it, and prints both.

```sh
cargo run -p bit_manipulation
```

Output:

```text
tester packed is 10111110111, from 111 + 1 / true + 101111
packed tester unpacked is tag: 111, compressed: true, length: 101111
```

## Implementation

One binary crate, two newtype modules and the header:

```text
src/
└── main.rs   # mod tag, mod length, RecordHeader::pack / ::unpack, tests
```

The layout, LSB on the right:

```text
 bit  15                      5    4    3        0
     ┌─────────────────────────┬────┬─────────────┐
     │         length          │ C  │     tag     │
     │        11 bits          │ 1  │    4 bits   │
     └─────────────────────────┴────┴─────────────┘
```

`tag = 0b0111`, `compressed = true`, `length = 0b000_0010_1111`:

```text
tag                       0b0000_0000_0000_0111
compressed  << 4          0b0000_0000_0001_0000
length      << 5          0b0000_0101_1110_0000   OR
                          ─────────────────────
packed                    0b0000_0101_1111_0111   = 1527
```

Some design details:

- **Tag and Length are separate modules** — each with a private field, a checked `new() -> Option<Self>`, and a `get()`. The `mod` is doing the load-bearing work; see [Making the invariant unrepresentable](#making-the-invariant-unrepresentable).
- **`pack` returns `u16`, not `Result`** — there is no failure left to report once both fields carry their width in the type. Validation happens once at the boundary in `main`, where `Tag::new(...).ok_or(...)?` turns a `None` into the program's error.
- **The `expect`s in `unpack` are infallible by construction** — `packed >> 5 & 0b111_1111_1111` cannot exceed 11 bits, so `None` is unreachable. The message documents *why* rather than guarding against anything.
- **`unpack` masks after casting** — `packed as u8 & 0xF` leans on `as` truncating the high byte, then masks the low nibble out of what's left.
- **Exhaustive tests, not random ones** — `round_trip_every_header` walks all 65,536 `u16` values; `layout_is_length_compressed_tag` asserts one known encoding so a mirrored shift bug in `pack`/`unpack` can't hide behind a passing round trip. Both were checked by mutating a shift and confirming they fail.

## Roadmap

- [ ] **`From` instead of inherent methods** — `impl From<RecordHeader> for u16` and `impl From<u16> for RecordHeader`, wrapping the same bodies as `pack`/`unpack`. It's the idiom Rust readers expect for conversions, it composes with generic code taking `impl Into<u16>`, and it gets the reciprocal `Into` for free. `From` rather than `TryFrom` is the right choice here specifically *because* the fields fill all 16 bits — every `u16` is a valid header, so the conversion is total in both directions. Note `From<RecordHeader>` takes `self` by value, so `RecordHeader` should derive `Copy` (all three fields already are) to keep call sites from moving.
- [ ] **Derive `PartialEq`** on `Tag`, `Length`, and `RecordHeader` — right now the tests can only compare through the packed `u16`, because there's no way to assert two headers are equal. With it, the round trip could read `assert_eq!(RecordHeader::unpack(bits), expected)` and failures would name the field that drifted instead of printing two bit patterns to diff by eye. `Debug` is already derived on the newtypes but not on `RecordHeader`, which needs it too for the assertion message.

## Development

```sh
cargo test    # round-trip over all 65_536 headers, plus the layout assertion
cargo clippy  # lint
cargo fmt     # format
```
