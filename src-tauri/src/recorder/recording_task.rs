use std::{fmt::Display, path::PathBuf, time::Duration};

use anyhow::{bail, Result};
use libobs_recorder::settings::{RateControl, RecorderSettings, Resolution, StdResolution, Window};
use libobs_recorder::Recorder;
use shaco::ingame::IngameClient;
use tauri::async_runtime::{self, JoinHandle};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};
use tokio::time::{interval, sleep};
use tokio_util::sync::CancellationToken;

use riot_datatypes::MatchId;

use crate::app::{action, AppEvent, EventManager, RecordingManager, SystemTrayManager};
use crate::cancellable;
use crate::recorder::Deferred;
use crate::state::{CurrentlyRecording, SettingsWrapper};

use super::window::{self, WINDOW_CLASS, WINDOW_PROCESS, WINDOW_TITLE};
use super::MetadataFile;

#[derive(Clone)]
pub struct GameCtx {
    pub app_handle: AppHandle,
    pub match_id: MatchId,
    pub cancel_token: CancellationToken,
}

#[derive(Debug)]
pub struct Metadata {
    pub match_id: MatchId,
    pub output_filepath: PathBuf,
    pub ingame_time_rec_start_offset: f64,
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "match_id={}, filepath={}, rec_offset={}",
            self.match_id,
            self.output_filepath.display(),
            self.ingame_time_rec_start_offset
        ))
    }
}

pub struct RecordingTask {
    join_handle: JoinHandle<Result<(Recorder, Metadata)>>,
    ctx: GameCtx,
}

impl RecordingTask {
    pub fn new(ctx: GameCtx) -> Self {
        let join_handle = async_runtime::spawn(Self::record(ctx.clone()));
        Self { join_handle, ctx }
    }

    pub async fn stop(self) -> Result<Metadata> {
        self.ctx.cancel_token.cancel();
        let (mut recorder, metadata) = self.join_handle.await??;

        async_runtime::spawn_blocking(move || {
            let stopped = recorder.stop_recording();
            let shutdown = recorder.shutdown();
            log::info!("stopping recording: stopped={stopped:?}, shutdown={shutdown:?}");

            self.ctx.app_handle.cleanup_recordings();
            self.ctx.app_handle.state::<CurrentlyRecording>().set(None);
            self.ctx.app_handle.set_tray_menu_recording(false);
            if let Err(e) = self
                .ctx
                .app_handle
                .send_event(AppEvent::RecordingsChanged { payload: () })
            {
                log::error!("RecordingTask failed to send event: {e}");
            }

            Ok(metadata)
        })
        .await?
    }

    async fn record(ctx: GameCtx) -> Result<(Recorder, Metadata)> {
        let (mut recorder, output_filepath) = cancellable!(Self::setup_recorder(&ctx), ctx.cancel_token, Result)?;

        // ingame_client timeout is 200ms, so no need to make cancellable with token
        let ingame_client = IngameClient::new();

        log::info!("waiting for game to start");
        let mut timer = interval(Duration::from_millis(500));
        while !ingame_client.active_game().await {
            let cancelled = cancellable!(timer.tick(), ctx.cancel_token, ());
            if cancelled {
                let shutdown = recorder.shutdown();
                bail!("waiting for game cancelled - recorder shutdown: {shutdown:?}");
            }
        }

        ctx.app_handle
            .state::<CurrentlyRecording>()
            .set(Some(output_filepath.clone()));
        ctx.app_handle.set_tray_menu_recording(true);

        // if initial game_data is successful => start recording
        if let Err(e) = recorder.start_recording() {
            ctx.app_handle.state::<CurrentlyRecording>().set(None);
            ctx.app_handle.set_tray_menu_recording(false);

            // if recording start failed stop recording just in case and retry next 'recorder loop
            let stop_recording = recorder.stop_recording();
            let shutdown = recorder.shutdown();
            bail!("failed to start recording: {e} (stopped={stop_recording:?}, shutdown={shutdown:?})");
        }

        // the ingame time when we start recording
        // this is important when the app gets started and starts recording in the middle of a game
        let ingame_time_rec_start_offset = ingame_client
            .game_stats()
            .await
            .map(|stats| stats.game_time)
            .unwrap_or_default();

        let metadata_file = MetadataFile::Deferred(Deferred {
            favorite: false,
            match_id: ctx.match_id.clone(),
            ingame_time_rec_start_offset,
            highlights: vec![],
        });
        if let Err(e) = action::save_recording_metadata(&output_filepath, &metadata_file) {
            log::info!("failed to save MetadataFile: {e}")
        }

        let metadata = Metadata {
            match_id: ctx.match_id,
            output_filepath,
            ingame_time_rec_start_offset,
        };

        Ok((recorder, metadata))
    }

    async fn setup_recorder(ctx: &GameCtx) -> Result<(Recorder, PathBuf)> {
        let settings_state = ctx.app_handle.state::<SettingsWrapper>();

        let window_size = Self::get_window_size().await?;
        let output_resolution = settings_state
            .get_output_resolution()
            .unwrap_or_else(|| StdResolution::closest_std_resolution(&window_size));

        log::info!("Using resolution ({output_resolution:?}) for window ({window_size:?})");

        let mut filename = settings_state.get_filename_format();
        if !filename.ends_with(".mp4") {
            filename.push_str(".mp4");
        }
        let filename_path = settings_state
            .get_recordings_path()
            .join(format!("{}", chrono::Local::now().format(&filename)));

        let mut settings = RecorderSettings::new(
            Window::new(WINDOW_TITLE, Some(WINDOW_CLASS.into()), Some(WINDOW_PROCESS.into())),
            window_size,
            output_resolution,
            &filename_path,
        );
        settings.set_framerate(settings_state.get_framerate());
        settings.set_rate_control(RateControl::CQP(settings_state.get_encoding_quality()));
        settings.set_audio_source(settings_state.get_audio_source());

        let mut recorder = Recorder::new_with_paths(
            ctx.app_handle
                .path()
                .resolve("libobs/extprocess_recorder.exe", BaseDirectory::Executable)
                .ok(),
            None,
            None,
            None,
        )?;

        log::info!("recorder settings: {settings:?}");
        recorder.configure(&settings)?;
        log::info!("recorder configured");

        log::info!("Selected adapter: {:?}", recorder.adapter_info());
        log::info!("Available encoders for adapter: {:?}", recorder.available_encoders());
        log::info!("Selected encoder: {:?}", recorder.selected_encoder());

        Ok((recorder, filename_path))
    }

    async fn get_window_size() -> Result<Resolution> {
        for _ in 0..60 {
            if let Some(window_size) = window::get_lol_window().and_then(window::get_window_size) {
                return Ok(window_size);
            }

            sleep(Duration::from_millis(500)).await;
        }

        bail!("unable to get window size");
    }
}
