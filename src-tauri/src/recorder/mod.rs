mod game_data;
mod recorder;
mod session_event;
mod util;
#[cfg(target_os = "windows")]
mod window;

pub use game_data::GameMetadata;
pub use recorder::start_recorder;
