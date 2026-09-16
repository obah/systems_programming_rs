use std::{println, sync::mpsc, thread, time::Duration, vec};

pub fn run() {
    let (tx, rx) = mpsc::channel();

    let tx1 = tx.clone();

    thread::spawn(move || {
        let vals = vec![
            String::from("hello"),
            String::from("from"),
            String::from("the"),
            String::from("other"),
            String::from("side"),
        ];

        for val in vals {
            tx1.send(val).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    });

    thread::spawn(move || {
        // let val = String::from("hello there");
        // tx.send(val).unwrap();

        //multiple values
        let vals = vec![
            String::from("i"),
            String::from("just"),
            String::from("called"),
            String::from("to"),
            String::from("say..."),
        ];

        for val in vals {
            tx.send(val).unwrap();
            thread::sleep(Duration::from_secs(1));
        }

        ////! this shouldnt work because val has moved, if not it could be dropped or editted here
        //? send takes ownership of its values
        // println!("val that was send is {val}");
    });

    //? recv() is blocking, but if we have other things to do in the main thread, then use try_recv()
    // let received = rx.recv().unwrap();
    // println!("Got: {received}");

    for received in rx {
        println!("Got: {received}");
    }
}
