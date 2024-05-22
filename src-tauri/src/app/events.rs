use serde::{Deserialize, Serialize};

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, strum_macros::IntoStaticStr, specta::Type, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AppEvent {
    RecordingsChanged { payload: () },
    MetadataChanged { payload: Vec<String> },
    MarkerflagsChanged { payload: () },
}

pub trait EventManager {
    fn send_event(&self, event: AppEvent) -> anyhow::Result<()>;
}

impl EventManager for tauri::AppHandle {
    fn send_event(&self, event: AppEvent) -> anyhow::Result<()> {
        use tauri::Manager;
        use AppEvent::*;

        match &event {
            RecordingsChanged { payload } => self.emit_all((&event).into(), payload)?,
            MetadataChanged { payload } => self.emit_all((&event).into(), payload)?,
            MarkerflagsChanged { payload } => self.emit_all((&event).into(), payload)?,
        };

        Ok(())
    }
}
