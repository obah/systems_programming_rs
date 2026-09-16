use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
};

pub fn run() {
    // let m = Mutex::new(5);

    // {
    //     let mut num = m.lock().unwrap();
    //     println!("inside the scope, m is {num}");
    //     *num = 6;
    // }
    // ? the lock is released/dropped automatically as the scope ended

    // println!("m is {m:?}");

    //? this uses Rc<T> to share the mutex between different threads
    //? Rc<T> doesnt implement Send as it isnt safe to share
    // let counter = Rc::new(Mutex::new(0));
    // let mut handles = vec![];

    // for _ in 0..10 {
    //     let counter = Rc::clone(&counter);

    //     let handle = thread::spawn(move || {
    //         let mut num = counter.lock().unwrap();
    //         *num += 1;
    //     });

    //     handles.push(handle);
    // }

    // for handle in handles {
    //     handle.join().unwrap();
    // }

    // println!("final result is {}", counter.lock().unwrap());

    //? Arc<T> is like Rc<T> but safe for concurrency
    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let counter = Arc::clone(&counter);

        let handle = thread::spawn(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("final result is {}", counter.lock().unwrap());
}
