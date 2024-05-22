use std::{path::PathBuf, sync::Mutex};

#[derive(Debug, Default)]
pub struct CurrentlyRecording(Mutex<Option<PathBuf>>);

impl CurrentlyRecording {
    pub fn set(&self, path: Option<PathBuf>) {
        *self.0.lock().unwrap() = path;
    }

    pub fn get(&self) -> Option<PathBuf> {
        self.0.lock().unwrap().clone()
    }
}
