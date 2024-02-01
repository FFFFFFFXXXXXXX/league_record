import { invoke } from "@tauri-apps/api";
import { EventCallback, UnlistenFn, listen } from "@tauri-apps/api/event";
import { WindowManager } from "@tauri-apps/api/window";

import { Checkboxes } from "./ui.js";

function newWindowManager(): any {
    return new WindowManager('main');
}

async function eventListener<T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn> {
    return await listen<T>(event, handler);
}

async function showWindow(): Promise<void> {
    await invoke('show_app_window');
}

async function getVideoPath(videoId: string): Promise<string> {
    const port = await invoke('get_asset_port');
    return `http://127.0.0.1:${port}/${videoId}`;
}

async function renameVideo(videoId: string, newVideoId: string): Promise<boolean> {
    return await invoke('rename_video', { 'videoId': videoId, 'newVideoId': newVideoId });
}

async function deleteVideo(videoId: string): Promise<boolean> {
    return await invoke('delete_video', { 'videoId': videoId });
}

async function openRecordingsFolder(): Promise<void> {
    await invoke('open_recordings_folder');
}

async function getRecordingsNames(): Promise<Array<string>> {
    return await invoke('get_recordings_list');
}

async function getRecordingsSize(): Promise<number> {
    return (await invoke('get_recordings_size'));
}

async function getMetadata(videoId: string): Promise<any> {
    return await invoke('get_metadata', { 'video': videoId })
}

async function getMarkerSettings(): Promise<Checkboxes> {
    return await invoke('get_current_marker_flags');
}

async function setMarkerSettings(markers: Checkboxes): Promise<void> {
    await invoke('set_current_marker_flags', { 'markerFlags': markers });
}

export default {
    newWindowManager: newWindowManager,
    eventListener: eventListener,
    showWindow: showWindow,
    getVideoPath: getVideoPath,
    renameVideo: renameVideo,
    deleteVideo: deleteVideo,
    openRecordingsFolder: openRecordingsFolder,
    getRecordingsNames: getRecordingsNames,
    getRecordingsSize: getRecordingsSize,
    getMetadata: getMetadata,
    getMarkerSettings: getMarkerSettings,
    setMarkerSettings: setMarkerSettings
};
