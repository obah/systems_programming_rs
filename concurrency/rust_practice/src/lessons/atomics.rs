use std::{
    io, println,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
    thread,
    time::Duration,
    todo,
};

pub fn run() {
    progress_report();
}

fn stop_flag() {
    static STOP: AtomicBool = AtomicBool::new(false);

    let background_thread = thread::spawn(|| {
        while !STOP.load(Relaxed) {
            some_work();
        }
    });

    for line in io::stdin().lines() {
        match line.unwrap().as_str() {
            "help" => println!("commands: help, stop"),
            "stop" => break,
            cmd => println!("unknown command: {cmd:?}"),
        }
    }

    STOP.store(true, Relaxed);

    background_thread.join().unwrap();
}

fn progress_report() {
    let num_done = AtomicUsize::new(0);

    let main_thread = thread::current();

    thread::scope(|s| {
        s.spawn(|| {
            for i in 0..100 {
                process_item(i);
                num_done.store(i + 1, Relaxed);
                main_thread.unpark();
            }
        });

        loop {
            let n = num_done.load(Relaxed);
            if n == 100 {
                break;
            }
            println!("working.. {n}/100 done");
            thread::park_timeout(Duration::from_secs(1));
        }
    });

    println!("done");
}

fn some_work() {
    todo!()
}

fn process_item(val: usize) {
    todo!()
}
