use std::thread;

/// Spawn 8 threads that each sum a slice of a big Vec<u64>.
pub fn sum_vec(vals: &[u64]) -> u64 {
    let chunk = vals.len().div_ceil(8).max(1); // chunks(0) panics

    thread::scope(|s| {
        let handles: Vec<_> = vals
            .chunks(chunk)
            .map(|c| {
                s.spawn(move || {
                    let partial = c.iter().sum::<u64>();
                    println!(
                        "{:?} summed {} values -> {partial}",
                        thread::current().id(),
                        c.len()
                    );
                    partial
                })
            })
            .collect();

        handles.into_iter().map(|h| h.join().unwrap()).sum()
    })
}

#[test]
fn sums_match_serial() {
    for n in [0usize, 1, 7, 8, 9, 100_000] {
        let v: Vec<u64> = (0..n as u64).collect();
        assert_eq!(sum_vec(&v), v.iter().sum::<u64>(), "n={n}");
    }
}
