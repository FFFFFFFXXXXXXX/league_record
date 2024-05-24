use std::sync::Mutex;

pub struct WindowState {
    size: Mutex<(f64, f64)>,
    position: Mutex<Option<(f64, f64)>>,
}

impl WindowState {
    pub fn get_size(&self) -> (f64, f64) {
        *self.size.lock().unwrap()
    }

    pub fn set_size(&self, size: (f64, f64)) {
        match self.size.lock() {
            Ok(mut s) => *s = size,
            Err(e) => log::error!("set_size - failed to lock WindowState: {e}"),
        };
        log::info!("saved window size: {}x{}", size.0, size.1);
    }

    pub fn get_position(&self) -> Option<(f64, f64)> {
        *self.position.lock().unwrap()
    }

    pub fn set_position(&self, position: (f64, f64)) {
        match self.position.lock() {
            Ok(mut s) => *s = Some(position),
            Err(e) => log::error!("set_position - failed to lock WindowState: {e}"),
        };
        log::info!("saved window position: {}x {}y", position.0, position.1);
    }
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            size: Mutex::from((1200.0, 650.0)),
            position: Mutex::from(None),
        }
    }
}
