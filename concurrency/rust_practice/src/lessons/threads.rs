use std::{thread, time::Duration, vec};

use crate::exercises::threads::sum_vec;

pub fn run() {
    // let handle = thread::spawn(|| {
    //     for i in 1..10 {
    //         println!("hi number {i} from the spawned thread!");
    //         thread::sleep(Duration::from_millis(1));
    //     }
    // });

    // for i in 1..5 {
    //     println!("hi number {i} from the main thread!");
    //     thread::sleep(Duration::from_millis(1));
    // }

    // handle.join().unwrap();

    // let v = vec![1, 2, 3];

    // let handle = thread::spawn(move || {
    //     println!("Here's a vector: {v:?}");
    // });

    // handle.join().unwrap();

    // thread::spawn(f);
    // thread::spawn(f);

    println!("hello from the main thread");

    let numbers = vec![1, 2, 3];

    thread::scope(|s| {
        s.spawn(|| {
            println!("length: {}", numbers.len());
        });
        s.spawn(|| {
            for n in &numbers {
                println!("{n}");
            }
        });
    });

    let vals = [0u64, 1, 7, 8, 9, 100_000];
    sum_vec(&vals);
}

fn f() {
    println!("hello from another thread");

    let id = thread::current().id();
    println!("this is my thread id: {id:?}");
}
