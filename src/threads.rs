use std::any::Any;
use crossbeam_channel::Sender;
use rand::Rng;

pub(crate) enum Message {
    Task(Box<dyn FnOnce() -> Box<dyn Any + Send> + Send>),
    Shutdown,
}


pub(crate) struct ThreadChannel {
    hashable_bit: u64,
    pub(crate) thread_tx: crossbeam_channel::Sender<Message>,
    pub(crate) main_rx: crossbeam_channel::Receiver<Option<Box<dyn Any + Send>>>,
}

pub(crate) fn create_long_thread() -> ThreadChannel {
    let (thread_tx, thread_rx) = crossbeam_channel::unbounded();
    let (main_tx, main_rx) = crossbeam_channel::unbounded();
    std::thread::spawn(move || {
        loop {
            let task: Message = thread_rx.recv().unwrap();
            match task {
                Message::Shutdown => break,
                Message::Task(task) => {
                    let result = task();
                    main_tx.send(Some(result)).unwrap();
                }
            }
        }
    });
    ThreadChannel {
        hashable_bit: rand::rng().random(),
        thread_tx,
        main_rx,
    }
}

pub(crate) fn create_temp_thread<T: Any + Send, F: FnOnce() -> T + Send + 'static>() -> (Sender<F>, crossbeam_channel::Receiver<T>) {
    let (thread_tx, thread_rx) = crossbeam_channel::unbounded();
    let (main_tx, main_rx) = crossbeam_channel::unbounded();
    std::thread::spawn(move || {
        let task: F = thread_rx.recv().unwrap();
        let result = task();
        main_tx.send(result).unwrap();
    });
    (thread_tx, main_rx)
}