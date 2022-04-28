use std::{fs::create_dir_all, path::PathBuf};

use tauri::api::path::video_dir;

pub struct AssetPort(u16);
impl AssetPort {
    pub fn new() -> Self {
        let port = port_check::free_local_port_in_range(1024, 65535).unwrap();
        AssetPort(port)
    }
    pub fn get(&self) -> u16 {
        self.0
    }
}

pub struct RecordingsFolder(PathBuf);
impl RecordingsFolder {
    pub fn new() -> Self {
        let mut rec_dir = video_dir().unwrap();
        rec_dir.push(PathBuf::from("league_recordings"));
        if !rec_dir.exists() {
            let _ = create_dir_all(rec_dir.as_path());
        }
        return Self(rec_dir);
    }
    pub fn get(&self) -> PathBuf {
        self.0.clone()
    }
    pub fn get_as_string(&self) -> Result<String, ()> {
        if let Ok(str) = self.0.clone().into_os_string().into_string() {
            return Ok(str);
        }
        Err(())
    }
}
