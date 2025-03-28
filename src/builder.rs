use std::sync::{Arc, Mutex};
use crate::SmartPool;

pub struct SmartPoolBuilder {
    pub(crate) threads: usize,
    spawn_delay: u64,
}

impl SmartPoolBuilder {
    pub fn new() -> Self {
        SmartPoolBuilder {
            threads: SmartPool::default().threads,
            spawn_delay: SmartPool::default().spawn_delay
        }
    }

    pub fn threads(mut self, threads: usize) -> Self {
        self.threads = threads;
        self
    }

    pub fn spawn_delay(mut self, spawn_delay: u64) -> Self {
        self.spawn_delay = spawn_delay;
        self
    }

    pub fn build(self) -> SmartPool {
        SmartPool {
            threads: self.threads,
            spawn_delay: self.spawn_delay,

            thread_conns: Arc::new(Mutex::new(vec![])),
            last_completed: Default::default()
        }
    }
}