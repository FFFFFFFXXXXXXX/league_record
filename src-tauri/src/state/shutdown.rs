use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Default)]
pub struct Shutdown(AtomicBool);

impl Shutdown {
    pub fn set(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn get(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
