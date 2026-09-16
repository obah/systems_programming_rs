use std::{println, sync::mpsc, thread};

pub fn run() {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let val = String::from("hello there");
        tx.send(val).unwrap();
    });

    //? recv() is blocking, but if we have other things to do in the main thread, then use try_recv()
    let received = rx.recv().unwrap();
    println!("Got: {received}");
}
