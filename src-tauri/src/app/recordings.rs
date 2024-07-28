use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context, Result};
use tauri::{async_runtime, AppHandle, Manager};

use crate::recorder::{self, Deferred, NoData};
use crate::state::{CurrentlyRecording, SettingsWrapper};
use crate::{recorder::MetadataFile, util};

pub trait RecordingManager {
    fn get_recordings(&self) -> Vec<PathBuf>;

    fn cleanup_recordings(&self);
    fn cleanup_recordings_by_size(&self);
    fn cleanup_recordings_by_age(&self);

    fn rename_recording(recording: PathBuf, new_name: String) -> Result<bool>;
    fn delete_recording(recording: PathBuf) -> Result<()>;

    fn get_recording_metadata(video_path: &Path, fetch: bool) -> Result<MetadataFile>;
    fn save_recording_metadata(path: &Path, metadata_file: &MetadataFile) -> Result<()>;
}

impl RecordingManager for AppHandle {
    fn get_recordings(&self) -> Vec<PathBuf> {
        // get all mp4 files in ~/Videos/%folder-name%
        let mut recordings = Vec::<PathBuf>::new();
        let Ok(read_dir) = self.state::<SettingsWrapper>().get_recordings_path().read_dir() else {
            return vec![];
        };

        let currently_recording = self.state::<CurrentlyRecording>().get();

        for entry in read_dir.flatten() {
            let path = entry.path();

            if !path.is_file() || Some(&path) == currently_recording.as_ref() {
                continue;
            }

            if let Some(ext) = path.extension() {
                if ext == "mp4" {
                    recordings.push(path);
                }
            }
        }
        recordings
    }

    fn cleanup_recordings(&self) {
        self.cleanup_recordings_by_age();
        self.cleanup_recordings_by_size();
    }

    fn cleanup_recordings_by_size(&self) {
        use std::cmp::Ordering;

        let Some(max_gb) = self.state::<SettingsWrapper>().max_recordings_size() else { return };
        let max_size = max_gb * 1_000_000_000; // convert to bytes

        let mut recordings = self.get_recordings();
        recordings.sort_by(|a, b| util::compare_time(a, b).unwrap_or(Ordering::Equal));

        let mut total_size = 0;

        // add size from video thats currently being recorded to the total (in case there is one)
        // so the total size of all videos stays below the threshhold set in settings
        if let Some(currently_recording_metadata) = self
            .state::<CurrentlyRecording>()
            .get()
            .and_then(|pb| pb.metadata().ok())
        {
            total_size += currently_recording_metadata.len();
        }

        // split recordings into 'favorites' and 'others' by json metadata 'favorite' value
        // in case reading the metadata fails put the recording into favorites so it doesn't get deleted
        let (favorites, others): (Vec<_>, Vec<_>) = recordings.into_iter().partition(|recording| {
            Self::get_recording_metadata(recording, false)
                .map(|metadata_file| metadata_file.is_favorite())
                .unwrap_or(true)
        });

        // get sum of sizes of recordings marked as favorites
        for recording in favorites {
            match recording.metadata() {
                Ok(metadata) => total_size += metadata.len(),
                Err(e) => log::warn!(
                    "failed to get size of recording (favorite) {}: {e}",
                    recording.display(),
                ),
            }
        }

        for recording in others {
            match recording.metadata() {
                Ok(metadata) => total_size += metadata.len(),
                Err(e) => log::warn!("failed to get size of recording {}: {e}", recording.display(),),
            }

            if total_size > max_size {
                if let Err(e) = Self::delete_recording(recording) {
                    log::error!("failed to delete file due to size limit: {e}");
                }
            }
        }
    }

    fn cleanup_recordings_by_age(&self) {
        fn too_old(file: &Path, max_age: Duration, now: SystemTime) -> Result<bool> {
            let creation_time = file.metadata()?.created()?;
            let time_passed = now.duration_since(creation_time)?;
            Ok(time_passed > max_age)
        }

        fn is_favorite(file: &Path) -> Result<bool> {
            AppHandle::get_recording_metadata(file, false).map(|metadata_file| metadata_file.is_favorite())
        }

        let Some(max_days) = self.state::<SettingsWrapper>().max_recording_age() else { return };
        let max_age = Duration::from_secs(max_days * 24 * 60 * 60);
        let now = SystemTime::now();
        for recording in self.get_recordings() {
            // in case checking 'too_old(...)' or 'is_favorite(...)' fails default to not deleting the file
            if too_old(&recording, max_age, now).unwrap_or(false) && !is_favorite(&recording).unwrap_or(true) {
                if let Err(e) = Self::delete_recording(recording) {
                    log::error!("failed to delete file due to age limit: {e}");
                }
            }
        }
    }

    fn rename_recording(recording_path: PathBuf, new_name: String) -> Result<bool> {
        let mut new_recording_path = recording_path.clone();
        new_recording_path.set_file_name(PathBuf::from(new_name).file_name().context("invalid new filename")?);

        let mut metadata_path = recording_path.clone();
        metadata_path.set_extension("json");

        let mut new_metadata_path = new_recording_path.clone();
        new_metadata_path.set_extension("json");

        if new_recording_path.is_file() || new_metadata_path.is_file() {
            return Ok(false);
        }

        fs::rename(&recording_path, &new_recording_path)?;
        fs::rename(&metadata_path, &new_metadata_path)?;

        Ok(true)
    }

    fn delete_recording(recording: PathBuf) -> Result<()> {
        fs::remove_file(&recording)?;

        let mut metadata_file = recording;
        metadata_file.set_extension("json");
        fs::remove_file(metadata_file)?;

        Ok(())
    }

    fn get_recording_metadata(video_path: &Path, fetch: bool) -> Result<MetadataFile> {
        let mut video_path = video_path.to_owned();
        if !video_path.is_file() {
            bail!("no such video");
        }

        video_path.set_extension("json");

        let filedata = if video_path.exists() && fs::metadata(&video_path)?.is_file() {
            let reader = BufReader::new(File::open(&video_path)?);
            serde_json::from_reader::<_, MetadataFile>(reader)?
        } else {
            let metadata_file = MetadataFile::NoData(NoData { favorite: false });
            Self::save_recording_metadata(&video_path, &metadata_file)?;
            metadata_file
        };

        match filedata {
            MetadataFile::Deferred(Deferred {
                match_id,
                ingame_time_rec_start_offset,
                favorite,
            }) => {
                if !fetch {
                    bail!("deferred, no metadata");
                }

                let mut metadata =
                    async_runtime::block_on(recorder::process_data(ingame_time_rec_start_offset, match_id))?;
                metadata.favorite = favorite;
                let metadata_file = MetadataFile::Metadata(metadata);
                if let Err(e) = Self::save_recording_metadata(&video_path, &metadata_file) {
                    log::error!("failed to save re-processed game metadata: {e}");
                }
                Ok(metadata_file)
            }
            metadata_file => Ok(metadata_file),
        }
    }

    fn save_recording_metadata(path: &Path, metadata_file: &MetadataFile) -> Result<()> {
        let mut path = path.to_owned();
        path.set_extension("json");

        let writer = BufWriter::new(File::create(path)?);
        Ok(serde_json::to_writer(writer, &metadata_file)?)
    }
}
