use std::sync::Mutex;

#[derive(Debug)]
pub struct FileWatcher(Mutex<notify::RecommendedWatcher>);

impl FileWatcher {
    pub fn new(watcher: notify::RecommendedWatcher) -> Self {
        FileWatcher(Mutex::new(watcher))
    }

    pub fn set(&self, watcher: notify::RecommendedWatcher) {
        // dropping the previous filewatcher stops it
        drop(std::mem::replace(&mut *self.0.lock().unwrap(), watcher));
    }
}
