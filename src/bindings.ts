// This file was generated by [tauri-specta](https://github.com/oscartbeaumont/tauri-specta). Do not edit this file manually.

import type { Recording, Deferred, Stats, GameMetadata, Queue, MatchId, BuildingType, Event, TowerType, Player, Team, NoData, MarkerFlags, GameEvent, DragonType, MetadataFile, Position, LaneType, MonsterType, } from "@fffffffxxxxxxx/league_record_types";

declare global {
    interface Window {
        __TAURI_INVOKE__<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
    }
}

// Function avoids 'window not defined' in SSR
const invoke = () => window.__TAURI_INVOKE__;

export function getMarkerFlags() {
    return invoke()<MarkerFlags>("get_marker_flags")
}

export function setMarkerFlags(markerFlags: MarkerFlags) {
    return invoke()<null>("set_marker_flags", { markerFlags })
}

export function getRecordingsPath() {
    return invoke()<string>("get_recordings_path")
}

export function getRecordingsSize() {
    return invoke()<number>("get_recordings_size")
}

export function getRecordingsList() {
    return invoke()<Recording[]>("get_recordings_list")
}

export function openRecordingsFolder() {
    return invoke()<null>("open_recordings_folder")
}

export function deleteVideo(videoId: string) {
    return invoke()<boolean>("delete_video", { videoId })
}

export function renameVideo(videoId: string, newVideoId: string) {
    return invoke()<boolean>("rename_video", { videoId,newVideoId })
}

export function getMetadata(videoId: string) {
    return invoke()<MetadataFile | null>("get_metadata", { videoId })
}

export function toggleFavorite(videoId: string) {
    return invoke()<boolean | null>("toggle_favorite", { videoId })
}
