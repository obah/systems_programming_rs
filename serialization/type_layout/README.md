# TYPE LAYOUT

A look at how Rust actually arranges a struct in memory, built as part of my Rust and systems engineering learning journey. The same two fields (`i8`, `i16`) are declared four ways — `repr(Rust)`, `repr(C)`, `repr(packed)`, `repr(packed(2))` — and the program prints `size_of` and `offset_of!` for each.

> ℹ️ **The program does not compile on purpose.** The last statement takes a reference into a `repr(packed)` struct, which is `error[E0793]`. That's the lesson, not a bug — see [Roadmap](#roadmap).

## What I learnt

- **Alignment drives size** — a type's alignment is the largest alignment among its fields (`i16` → 2 here), and a struct's size is always rounded up to a multiple of it. Every version below is 2-aligned, so no version can be size 1 or 3 unless alignment itself is changed.
- **Two kinds of padding** — _interior_ padding sits between fields to push the next one onto its boundary; _trailing_ padding sits at the end to round the total up. Same byte count, different reason, and which one you get depends on field order.
- **`repr(C)` freezes the order, `repr(Rust)` doesn't** — the default representation is explicitly unspecified: the compiler may reorder fields (in practice it sorts by decreasing alignment). So `offset_of!` on a `repr(Rust)` struct describes _this_ rustc on _this_ target, and nothing more.
- **`repr(packed)` vs `repr(packed(N))`** — bare `packed` is `packed(1)`: alignment 1, no padding at all. `packed(N)` only _caps_ field alignment at `N`; if no field needs more than `N` the attribute changes nothing. And `packed` alone doesn't imply `C`, so reordering stays on the table.
- **Packing costs you references** — `&value.field` on a packed struct is a compile error, because a reference must be aligned even if it's never dereferenced. Creating a misaligned one is UB, so the compiler refuses.
- **`size_of` is in the prelude, `offset_of!` isn't** — hence the single `use std::mem::offset_of;` at the top and the bare `size_of::<T>()` calls.
- **Why this matters for serialization** — a binary codec can't just reinterpret the bytes of a `repr(Rust)` struct. Only `repr(C)` gives a layout you can write down, document, and depend on across compiler versions.

## Usage

A single binary with no arguments — but it currently fails to build:

```sh
cargo run -p type_layout
```

```text
error[E0793]: reference to field of packed struct is unaligned
  --> serialization/type_layout/src/main.rs:54:13
   |
54 |     let r = &packed_struct.tag;
   |             ^^^^^^^^^^^^^^^^^^
   |
   = note: this struct is 1-byte aligned, but the type of this field may require higher alignment
   = note: creating a misaligned reference is undefined behavior (even if that reference is never dereferenced)
```

Comment out that one line and it runs:

```text
Size of CStruct is 4 and its offset of data is 0, offset of tag is 2
Size of PackedStruct is 3 and its offset of data is 0, offset of tag is 1
Size of PackedStruct2 is 4 and its offset of data is 2, offset of tag is 0
Size of RustStruct is 4 and its offset of data is 2, offset of tag is 0
```

## Implementation

One binary crate, no dependencies, four struct definitions and four `println!`s:

```text
src/
└── main.rs   # the four reprs, the measurements, and the explanation as comments
```

All four hold `data: i8` then `tag: i16`. What the compiler does with them:

```text
                offset 0     1     2     3
CStruct                data  pad   tag──tag     size 4   interior padding at 1
PackedStruct           data  tag──tag           size 3   no padding
PackedStruct2          tag──tag    data  pad    size 4   trailing padding at 3
RustStruct             tag──tag    data  pad    size 4   trailing padding at 3
```

| Struct          | Repr                | Size | Why                                                                                                                                                |
| --------------- | ------------------- | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CStruct`       | `repr(C)`           | 4    | Declaration order is guaranteed, so `data` must sit at 0 and a byte of padding goes in at 1 to get `tag` onto its 2-byte boundary. No trailing pad. |
| `RustStruct`    | default             | 4    | Reordered to `tag` first, `data` second, then a trailing byte rounds 3 up to 4. Same size as `repr(C)`, padding somewhere else.                     |
| `PackedStruct`  | `repr(packed)`      | 3    | Alignment forced to 1, so nothing is padded anywhere. The only version that's actually smaller.                                                     |
| `PackedStruct2` | `repr(packed(2))`   | 4    | Identical to `RustStruct`: max field alignment is already 2, so the cap never binds, and without `C` the reorder still happens.                     |

Some notes on what the numbers show:

- **Reordering saved nothing here** — with two fields there was nothing to win, which is why `repr(Rust)` and `repr(C)` come out the same size. The interesting case needs three: `(u8, u64, u8)` is 24 bytes under `repr(C)` (pad, pad) and 16 under `repr(Rust)` (the two `u8`s pack together after the `u64`).
- **`packed(2)` looks like a no-op because it is one** — the difference from `CStruct` comes from the missing `C`, not from the packing. It would take a field with alignment > 2 (an `i64`) for the cap to do anything visible.
- **The explanation lives in the source** — the outputs and the reasoning are committed as a comment block at the bottom of `main.rs`, so the file is self-contained: read it and you get the answer without running it (which is just as well, since it doesn't build).

## Roadmap

- [ ] **The `E0793` line** (`main.rs:54`) — left failing on purpose. The three ways out, in increasing order of how much you're admitting you meant it: copy the field into a local (`let t = packed_struct.tag;` — a copy is fine, it's the _reference_ that's illegal), take a raw pointer with `&raw const` and read it via `read_unaligned`, or drop `packed` for `packed(N)` at an `N` the field can live with.
- [ ] **A field that makes `packed(2)` bite** — add an `i64` (alignment 8) to `PackedStruct2` and watch the cap clamp it to 2, versus the same struct without the attribute.
- [ ] **A three-field struct** — `(u8, u64, u8)` under both `repr(C)` and `repr(Rust)`, to show the reorder actually recovering 8 bytes instead of merely moving a pad byte around.
- [ ] **Enum layout** — `size_of::<Option<u8>>()` is 2 but `size_of::<Option<&u8>>()` is 8, because the null pointer doubles as the `None` discriminant. The niche optimisation is the next layout rule worth measuring.

## Development

```sh
cargo clippy  # lint
cargo fmt     # format
```
