function showWindow() {
    __TAURI__.invoke('show_app_window');
}

async function getVideoPath(videoId) {
    const port = await __TAURI__.invoke('get_asset_port');
    return `http://127.0.0.1:${port}/${videoId}`;
}

async function renameVideo(videoId, newVideoId) {
    return await __TAURI__.invoke('rename_video', { 'videoId': videoId, 'newVideoId': newVideoId });
}

async function deleteVideo(videoId) {
    return await __TAURI__.invoke('delete_video', { 'videoId': videoId });
}

async function openRecordingsFolder() {
    await __TAURI__.invoke('open_recordings_folder');
}

async function getRecordingsNames() {
    return await __TAURI__.invoke('get_recordings_list');
}

async function getRecordingsSize() {
    return (await __TAURI__.invoke('get_recordings_size')).toString().substring(0, 4);
}

async function getMetadata(videoId) {
    return await __TAURI__.invoke('get_metadata', { 'video': videoId })
}

async function getMarkerSettings() {
    return await __TAURI__.invoke('get_current_marker_flags');
}

async function setMarkerSettings(markers) {
    await __TAURI__.invoke('set_current_marker_flags', { 'markerFlags': markers });
}

export default {
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
