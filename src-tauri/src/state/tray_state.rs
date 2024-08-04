use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Default)]
pub struct TrayState {
    update_available: AtomicBool,
    recording: AtomicBool,
}

impl TrayState {
    pub fn set_update_available(&self, update_available: bool) {
        self.update_available.store(update_available, Ordering::Release);
    }

    pub fn update_available(&self) -> bool {
        self.update_available.load(Ordering::Acquire)
    }

    pub fn set_recording(&self, recording: bool) {
        self.recording.store(recording, Ordering::Release);
    }

    pub fn recording(&self) -> bool {
        self.recording.load(Ordering::Acquire)
    }
}
