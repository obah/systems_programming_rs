# SERDE SERIALIZATION

An exercise in serde's data model, built as part of my Rust and systems engineering learning journey. A `Point` struct gets both traits from the derive macro; a `Message` enum gets a hand-written `Deserialize` with its own `Visitor`, so the line between _the format_ and _the type_ stops being magic.

> ℹ️ **The brief said hand-implement `Serialize`; this hand-implements `Deserialize` instead.** Serializing is the easy direction — you already hold a valid value and just walk it. Deserializing is where the visitor actually earns its keep, because the input is untrusted and every shape has to be accounted for. `serialize_enum` here is still a `format!` string, not a `Serialize` impl — see [Roadmap](#roadmap).

## What I learnt

- **Two types, not one** — `Deserialize::deserialize` is almost never where the logic goes. Its whole job is to pick a hint and hand over a visitor. The visitor is a separate, usually zero-sized struct with `type Value = YourType`, an `expecting()` that writes the error message, and one `visit_*` method per data-model kind you accept. Anything not implemented defaults to a type error. Cramming logic into `deserialize` itself is the classic wrong turn.
- **The hint is a request, not a command** — `deserialize_map` says what you _expect_. JSON is self-describing, so it reads the actual bytes and may call a different `visit_*` anyway; bincode and postcard have no type info on the wire and _must_ obey. Same trait, materially different contract, which is why `deserialize_any` works for JSON and hard-fails for bincode.
- **The `'de` lifetime is the input's lifetime** — `visit_str(&str)` is a transient borrow you must copy out of; `visit_borrowed_str(&'de str)` lives as long as the input buffer, which is what makes zero-copy possible. `visit_borrowed_str` forwards to `visit_str` by default, so zero-copy is opt-in. Most serde confusion traces back to this one distinction.
- **Compound types are pull-based iterators** — `visit_map` hands you a `MapAccess` and you drive it: `next_key()` until `None`, each key followed by its `next_value()`. Skip the value and the next `next_key()` reads that dangling value _as_ a key.
- **Tagging decides how much you can stream** — with the tag internal (`"type"` beside the payload fields), the variant isn't known until the map is drained, so every field has to be buffered. Externally tagged enums know the variant first and can construct as they read.

## Usage

A single binary with no arguments — run it and read the four lines.

```sh
cargo run -p serde_serialization
```

Output:

```text
serialized = {"x":1,"y":2}
deserialized = Point { x: 1, y: 2 }
message = {"type": "Request", "id": "1", "method": "ping", "length": 4}
message = Request { id: "1", method: "ping", length: 4 }
```

The first two lines are the derive doing everything. The last two are the hand-written path: `format!` out, visitor back in.

## Implementation

One binary crate, everything in one file:

```text
src/
└── main.rs   # Point (derived), Message + MessageVisitor (hand-written), tests
```

The wire format is an **internally tagged** enum — the discriminant sits beside the payload rather than wrapping it:

```text
{"type": "Request",  "id": "1", "method": "ping", "length": 4}
{"type": "Response", "id": "1", "result": 1.5}
```

Some design details:

- **`deserialize` picks the hint, the visitor does the work** — `deserialize_map(MessageVisitor)` is the entire impl. `MessageVisitor` is zero-sized: it carries no state, it's just somewhere to hang the callbacks. Only `visit_map` is implemented, so anything that isn't a map fails with the `expecting()` string.
- **Fields are buffered, the variant is decided last** — JSON maps are unordered, so `"type"` can arrive after the fields it describes. `visit_map` collects all five possible fields into `Option`s and only matches on the tag once `next_key()` returns `None`. That's exactly what `#[serde(tag = "type")]` generates, and it's why internally tagged enums cost more than externally tagged ones.
- **Validation lives at the boundary** — which fields are required depends on the variant, so the checks happen after the loop, not per field: `missing_field` for a `Request` with no `method`, `unknown_variant` for a `"type"` that isn't one of the two. The wire can't build a `Message` the enum's own shape forbids.
- **Unknown keys are consumed, not ignored** — `map.next_value::<de::IgnoredAny>()?` parses and discards without allocating. Dropping the value instead of consuming it desynchronizes the key/value pairing for everything after it.
- **`next_value()` infers its own type** — the `Option` it lands in decides the parse, so `"length"` comes back as a real `u16` rather than a string that needs a second pass.
- **The serializer had to be fixed to be JSON at all** — it originally emitted `"type": "Request", ...` with no surrounding braces and quoted the numbers (`"length": "4"`), so nothing could parse it back. Braces added, quotes dropped. A round trip is the only honest test of a serializer.

## Roadmap

- [ ] **Hand-implement `Serialize` for `Message`** — the actual brief. Replace the `format!` with `serialize_struct_variant` / `serialize_map` so both directions go through the data model instead of one going through string formatting. `format!` also can't escape: a `method` containing a `"` produces invalid JSON today.
- [ ] **A real config struct** — `Point` is a stand-in. Swap in something with `#[serde(default)]`, `#[serde(rename)]`, and `Option` fields to see what the derive handles that a hand-written impl would have to spell out.
- [ ] **`visit_seq`** — bincode and postcard send structs and enum payloads as positional sequences, so this impl only works for self-describing formats right now. Derived impls always write both arms.
- [ ] **Zero-copy** — `next_key::<String>()` allocates per key. `&'de str` fields plus `visit_borrowed_str` avoid it, at the cost of tying `Message` to the lifetime of its input buffer.
- [ ] **`deserialize_enum` should return `Result`** — it currently `expect`s, which turns a bad byte on the wire into a panic. Fine for a lesson, wrong anywhere else.
- [ ] **`cargo expand`** — derive `#[serde(tag = "type")] Deserialize` on `Message` and read the generated code next to the hand-written version. The hidden `Field` enum and the dual seq/map visitor are the parts worth seeing.

## Development

```sh
cargo test    # run all unit tests
cargo clippy  # lint
cargo fmt     # format
```

Three tests in a `#[cfg(test)]` module at the bottom of `main.rs`: a round trip through both variants, one proving field order and unknown keys don't matter, and one asserting the four ways bad input is rejected (missing fields, unknown variant, no tag, not a map).
