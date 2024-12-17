use serde::{Deserialize, Serialize};

#[allow(clippy::enum_variant_names)]
#[cfg_attr(test, derive(specta::Type, tauri_specta::Event))]
#[derive(Debug, Clone, strum_macros::IntoStaticStr, Serialize, Deserialize)]
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
        use tauri::Emitter;
        use AppEvent::*;

        match &event {
            RecordingsChanged { payload } => self.emit((&event).into(), payload)?,
            MetadataChanged { payload } => self.emit((&event).into(), payload)?,
            MarkerflagsChanged { payload } => self.emit((&event).into(), payload)?,
        };

        Ok(())
    }
}