use crate::SmartPool;
use crate::builder::SmartPoolBuilder;
use crossbeam_channel::Sender;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

pub struct BackgroundSmartPool {
    pool: Arc<SmartPool>,
    tx: Arc<Mutex<Option<Sender<Box<dyn FnOnce() + Send>>>>>,
    started: AtomicBool
}

impl SmartPoolBuilder {
    pub fn to_background(self) -> BackgroundSmartPool {
        let mut new_pool = self;
        if new_pool.threads > 1 {
            new_pool.threads -= 1;
        }
        BackgroundSmartPool {
            pool: Arc::new(new_pool.build()),
            tx: Default::default(),
            started: AtomicBool::new(false)
        }
    }
}

fn background_handler(
    rx: crossbeam_channel::Receiver<Box<dyn FnOnce() + Send>>,
    pool: Arc<SmartPool>,
) {
    loop {
        let task = rx.recv().unwrap();
        pool.execute(task);
    }
}

impl BackgroundSmartPool {
    pub fn start(&self) {
        let pool = self.pool.clone();
        let (tx, rx) = crossbeam_channel::unbounded();
        *self.tx.lock().unwrap() = Some(tx);
        std::thread::spawn(move || background_handler(rx, pool));
        self.started.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn execute<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if !self.started.load(std::sync::atomic::Ordering::Relaxed) {
            panic!("BackgroundSmartPool not started");
        }
        unsafe {
            self.tx
                .lock()
                .unwrap()
                .clone()
                // There isn't any way for this to be None since you can't get a fully constructed
                // BackgroundSmartPool without a tx, and we check if it's started before executing
                .unwrap_unchecked()
                .send(Box::new(task))
                .unwrap();
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_background_smart_pool() {
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let pool = SmartPoolBuilder::new().threads(2).to_background();
        pool.start();
        for _ in 0..10 {
            let counter = counter.clone();
            pool.execute(move || {
                counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            });
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 10);
    }

    #[test]
    fn test_background_smart_pool_speed() {
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let pool = SmartPoolBuilder::new().to_background();
        pool.start();
        let start = std::time::Instant::now();
        for _ in 0..100 {
            let counter = counter.clone();
            pool.execute(move || {
                counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                std::thread::sleep(std::time::Duration::from_millis(10));
            });
        }
        let multi_threaded_time = start.elapsed();
        let start = std::time::Instant::now();
        for _ in 0..100 {
            let counter = counter.clone();
            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let single_threaded_time = start.elapsed();
        assert!(multi_threaded_time < single_threaded_time);
    }
}
