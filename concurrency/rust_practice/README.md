# CONCURRENCY PRACTICE

Threads, channels, shared state and atomics — the four ways std gives you to have more than one thing happening at once — built as part of my Rust and systems engineering learning journey. `lessons/` follows the book with the code typed out and annotated; `exercises/` is where the same idea gets rebuilt without the book open.

> ⚠️ **Work in progress.** `cargo run` currently hangs. `main` runs `atomics::run()` → `progress_report()`, whose `process_item` is still `todo!()`, so the worker thread panics on the first item and the progress loop spins on `working.. 0/100 done` forever. Why it hangs instead of crashing is [a lesson in itself](#a-panic-in-a-scoped-thread-waits-for-the-scope). The channels exercise is empty. See [Roadmap](#roadmap).

## What I learnt

### `spawn` needs `'static`, `scope` doesn't

`thread::spawn` can't know when the thread ends, so its closure must outlive everything — `'static`. That's why borrowing a local is a compile error and why `move` shows up on nearly every spawned closure, dragging the `Vec` into the thread with it.

`thread::scope` closes that hole by guaranteeing all threads are joined before it returns, which lets them borrow:

```rust
let numbers = vec![1, 2, 3];

thread::scope(|s| {
    s.spawn(|| println!("length: {}", numbers.len()));  // borrows, no move
    s.spawn(|| for n in &numbers { println!("{n}") });  // twice, shared
});
```

Two threads holding `&numbers` at once is fine — they're shared references. `move` would have been a problem here, not a solution: the first closure would take the `Vec` and the second wouldn't compile.

### A panic in a scoped thread waits for the scope

This is the one the code taught me by hanging. A panicking thread doesn't take the process with it — the panic is caught and stored, and `join()` hands it back as `Err`. `thread::scope` does the same thing at the end: it joins everything, and re-raises if any thread panicked.

The catch is *when*. `progress_report` spawns a worker, then loops in the scope body waiting for the worker's counter to reach 100:

```rust
thread::scope(|s| {
    s.spawn(|| { for i in 0..100 { process_item(i); num_done.store(i + 1, Relaxed); ... } });

    loop {
        if num_done.load(Relaxed) == 100 { break; }   // never true — the worker died at i = 0
        println!("working.. {n}/100 done");
        thread::park_timeout(Duration::from_secs(1));
    }
});  // ← the re-raise lives here, and control never gets here
```

The worker panics immediately (`process_item` is `todo!()`), so `num_done` stays at 0, so the loop never breaks, so the scope never ends, so the stored panic is never re-raised. A dead worker and a live waiter produce a hang, not a crash. The panic message does print to stderr, which is the only clue you get.

Worth noting the termination check is `== 100` rather than `>= 100`. Safe here — one writer, storing every value in order — but it's the kind of condition that turns a single missed update into exactly this hang.

### `send` takes ownership, and the receiver ends when the last sender drops

`mpsc` is multi-producer, single-consumer: clone the sender per producer, and the receiver stays put.

`send(val)` moves `val` in, which is the type system enforcing the whole point of message passing — once it's sent you can't read it, mutate it, or drop it, so there's nothing to race over.

The part that isn't obvious is the shutdown rule. `for received in rx` ends when **every** sender has dropped, not when one has. That's why `tx` gets moved into the second thread rather than cloned again and left sitting in `main`: a stray live sender in scope means the loop never ends, and the program hangs with no error to read. Here the receive loop doubles as the join — `main` can't get past it until both producers have finished and dropped.

`recv()` blocks the caller; `try_recv()` returns immediately with an `Err` if nothing's queued, for when the receiving thread has its own work to get on with.

### `Arc<Mutex<T>>` is the `Rc<RefCell<T>>` of threads

Ownership says one owner, so sharing a counter across ten threads needs both a shared owner and a way to mutate through a shared reference.

- **`Mutex` gives the mutation.** `lock()` returns a guard that derefs to `&mut T` through a `&self` — interior mutability, with the borrow checked at runtime by the lock rather than at compile time.
- **The guard unlocks by dropping.** There is no `unlock()`. The lock is held until the guard goes out of scope, so an inner `{ }` block is how you release early. Holding one across an `.await` or a long computation is how you build a bottleneck.
- **`Arc`, not `Rc`.** `Rc` refcounts non-atomically and deliberately doesn't implement `Send`, so `thread::spawn` refuses it: *`Rc<Mutex<i32>>` cannot be sent between threads safely*. `Arc` is the same API with atomic refcounting — you pay for it, which is why `Rc` still exists as the single-threaded default.
- **`lock()` returns `Result`** because a thread that panics while holding the lock *poisons* it, and every later `lock()` reports that the data may be half-updated. `.unwrap()` is the usual answer in a lesson, and a real decision anywhere else.

### Atomics: the ordering is the interesting part

When the shared state is one integer or one flag, an atomic replaces the whole `Arc<Mutex<_>>` — no allocation, no guard, no poisoning.

- **`Relaxed` guarantees the operation, not the neighbourhood.** The load or store itself won't tear, but nothing is promised about how it orders against *other* variables. Both lessons get away with it for the same reason: there's exactly one shared variable, and the reader is fine with a slightly stale value. A progress bar that briefly shows 7 instead of 8 is still correct. The moment a second variable has to be visible *before* the flag — publishing data, then flipping "ready" — `Relaxed` stops being enough and it's `Release`/`Acquire`.
- **Atomics are const-constructible**, so `static STOP: AtomicBool = AtomicBool::new(false)` just works — no `lazy_static`, no `OnceLock`. A `static` is also `'static`, which is exactly what `thread::spawn` demands, so a static atomic is the no-ceremony way to signal a detached thread.
- **`park`/`unpark` is a latency knob, not correctness.** The waiter still re-checks the condition in a loop, because `park` can return spuriously and an `unpark` that arrives before the `park` is just remembered. `park_timeout` bounds the damage: a lost wake-up costs one second of staleness rather than a permanent sleep.

### `collect()` before `join()`, or the threads run one at a time

From the `sum_vec` exercise — split a slice into 8 chunks, sum each on its own thread:

```rust
let handles: Vec<_> = vals.chunks(chunk).map(|c| s.spawn(move || c.iter().sum::<u64>())).collect();
handles.into_iter().map(|h| h.join().unwrap()).sum()
```

The `collect()` is load-bearing. Iterators are lazy, so chaining `.map(spawn).map(join).sum()` into one expression would spawn a thread, immediately block joining it, *then* spawn the next — eight threads run strictly in sequence, and the whole thing is slower than the plain loop. Collecting forces every spawn to happen before the first join.

`thread::scope` again pulls its weight: the chunks are `&[u64]` borrowed from the caller's slice, so nothing is cloned to satisfy `'static`. And `div_ceil(8).max(1)` — `chunks(0)` panics, so the empty input needs the `max`.

## Usage

The crate is a set of lessons, not a program. `main` runs one of them, picked by editing the file:

```rust
fn main() {
    // threads::run();
    // channels::run();
    // shared_state::run();
    atomics::run();
}
```

```sh
cargo run -p rust_practice
```

As shipped that runs `atomics`, which hangs — see the callout at the top. `threads`, `channels` and `shared_state` all run to completion; `channels` takes about 5 seconds, since each producer sleeps a second between sends.

## Implementation

```text
src/
├── main.rs            # picks one lesson to run
├── lib.rs             # pub mod exercises; pub mod lessons;
├── lessons/
│   ├── threads.rs     # spawn vs scope, move closures, thread ids
│   ├── channels.rs    # mpsc, cloned senders, rx as an iterator
│   ├── shared_state.rs # Mutex, Rc vs Arc, the ten-thread counter
│   └── atomics.rs     # AtomicBool stop flag, AtomicUsize progress + park/unpark
└── exercises/
    ├── threads.rs     # sum_vec — parallel sum over 8 chunks, plus its test
    └── channels.rs    # empty
```

Some details:

- **The superseded versions are kept, commented out.** `shared_state.rs` still holds the `Rc` attempt above the `Arc` one that replaced it, and `threads.rs` the `move`-closure version above the scoped one. The diff between what failed and what compiled is the lesson, so deleting it would delete the point.
- **Lessons depend on exercises, not the other way round.** `lessons::threads::run` finishes by calling `exercises::threads::sum_vec`, which is why the exercise sits in the library rather than next to its lesson.
- **One test in the crate**, on `sum_vec`: the parallel sum is checked against `iter().sum()` for `n` in `[0, 1, 7, 8, 9, 100_000]` — empty, fewer items than threads, one short of a clean split, exact, one over, and large. The lessons themselves print rather than assert.
- **`stop_flag()` is never called** — it's read, not run, because it blocks on stdin. That plus the commented-out code is where the `dead_code` and `unused_imports` warnings come from.

## Roadmap

- [ ] **Fill in `process_item` and `some_work`** — both are `todo!()`, which is what makes `cargo run` hang. Even a `thread::sleep` would make the progress reporter demonstrate the thing it's there to demonstrate.
- [ ] **Write the channels exercise** — `exercises/channels.rs` is an empty file. A worker pool (N threads pulling jobs off one channel, results back over another) would exercise the drop-the-sender rule properly, since that's where forgetting it actually bites.
- [ ] **An ordering lesson beyond `Relaxed`** — everything here is `Relaxed` and gets away with it. A `Release` store publishing data behind an `Acquire`-loaded ready flag is the case where it stops working, and it's the whole reason orderings exist.
- [ ] **Pick the lesson with an argument** — `cargo run -p rust_practice -- atomics` instead of commenting out three lines in `main`. A `match std::env::args().nth(1)` is enough, and it would clear the unused-import warnings too.
- [ ] **Assertions in the lessons** — `shared_state` prints `final result is 10` and nobody checks it. `assert_eq!(*counter.lock().unwrap(), 10)` would turn the lost-update bug it exists to prevent into a test failure.

## Development

```sh
cargo test -p rust_practice   # one test: sum_vec against a serial sum
cargo clippy                  # lint — the crate currently has 7 warnings, all from kept-around commented code
cargo fmt                     # format
```
