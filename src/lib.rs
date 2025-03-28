#![feature(integer_atomics)]

use crate::ThreadState::{Busy, Idle};
use std::any::Any;
use std::sync::atomic::{AtomicU128, Ordering};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

mod builder;
mod threads;
mod background;

fn get_unix_time() -> u128 {
    let now = std::time::SystemTime::now();
    now.duration_since(UNIX_EPOCH).unwrap().as_nanos()
}

pub struct SmartPool {
    threads: usize,
    spawn_delay: u64,
    thread_conns: Arc<Mutex<Vec<(threads::ThreadChannel, ThreadState)>>>,
    last_completed: AtomicU128,
}

#[derive(Eq, PartialEq, Hash)]
enum ThreadState {
    Idle,
    Busy,
}

impl Default for SmartPool {
    fn default() -> Self {
        SmartPool {
            threads: num_cpus::get(),
            spawn_delay: 5 * 1000 * 1000,
            thread_conns: Arc::new(Mutex::new(vec![])),
            last_completed: AtomicU128::new(0),
        }
    }
}

impl SmartPool {
    pub fn start(&mut self) {
        for _ in 0..self.threads {
            let thread_channel = threads::create_long_thread();
            self.thread_conns
                .lock()
                .unwrap()
                .push((thread_channel, Idle));
        }
    }

    pub fn execute<F, T>(&self, task: F) -> T
    where
        F: FnOnce() -> T + Send + 'static,
        T: Any + Send,
    {
        loop {
            for (conn, state) in self.thread_conns.lock().unwrap().iter_mut() {
                if *state == Idle {
                    *state = Busy;
                    let erased_ret = || Box::new(task()) as Box<dyn Any + Send>;
                    conn.thread_tx
                        .send(threads::Message::Task(Box::new(erased_ret)))
                        .unwrap();
                    let result = conn.main_rx.recv().unwrap().unwrap();
                    *state = Idle;
                    self.last_completed
                        .store(get_unix_time(), Ordering::Relaxed);
                    return *result.downcast().unwrap();
                }
            }
            let now = get_unix_time();
            if now - self.last_completed.load(Ordering::Relaxed) > self.spawn_delay as u128 {
                let (tx, rx) = threads::create_temp_thread();
                tx.send(task).unwrap();
                return rx.recv().unwrap();
            }
        }
    }
}

impl Drop for SmartPool {
    fn drop(&mut self) {
        for (conn, _) in self.thread_conns.lock().unwrap().iter() {
            conn.thread_tx.send(threads::Message::Shutdown).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_pool() {
        let mut pool = SmartPool::default();
        pool.start();
        let result = pool.execute(|| {42});
        assert_eq!(result, 42);
        let result2 = pool.execute(|| {"Hello, World!"});
        assert_eq!(result2, "Hello, World!");
    }
}
