import '../video-js/video.min.js';
import '../videojs-markers/videojs-markers.js';

import tauri from './tauri.js';
import ui from './ui.js';
import { sleep } from './util.js';

// sets the time a marker jumps to before the actual event happens
// jumps to (eventTime - EVENT_DELAY) when a marker is clicked
const EVENT_DELAY = 3;

let currentEvents = [];

// create video player
const player = videojs('video_player', {
    'aspectRatio': '16:9',
    'playbackRates': [0.5, 1, 1.5, 2],
    'autoplay': false,
    'controls': true,
    'preload': 'auto',
    'enableSourceset': true
});

main();

async function main() {
    // disable right click menu
    addEventListener('contextmenu', event => event.preventDefault());

    // configure and start marker plugin
    player.markers({
        'markerTip': {
            'display': true,
            'innerHtml': marker => marker.text ?? '',
        },
        'markerStyle': {
            'minWidth': '4px',
            'borderRadius': '30%'
        }
    });

    // listen for events from player.reset() and player.src() to update the UI accordingly

    player.on('playerreset', () => {
        player.markers().removeAll();
        ui.setVideoDescription('', 'No recording selected!');
        ui.setActiveVideoId(null);

        // make sure the bigplaybutton and controlbar are hidden when resetting the video src
        player.bigPlayButton.hide();
        player.controlBar.hide();
    });

    player.on('sourceset', ({ src }) => {
        // ignore all sources that are falsy (e.g. null, undefined, empty string)
        // because player.reset() for example triggers a 'sourceset' event with { src: "" }
        if (!src) return;

        // split src ('http://127.0.0.1:49152/{videoId}') at the last '/' to get the video from the src
        // since {videoId} has to be a valid filename and filenames can't contain '/' this works always
        const videoId = src.substring(src.lastIndexOf('/') + 1, src.length);
        ui.setActiveVideoId(videoId);
        setMetadata(videoId);

        // re-show the bigplaybutton and controlbar when a new video src is set
        player.bigPlayButton.show();
        player.controlBar.show();
    });

    // add events to html elements
    ui.setRecordingsFolderBtnOnClickHandler(tauri.openRecordingsFolder);
    ui.setCheckboxOnClickHandler(changeMarkers);

    // listen if the videojs player fills the whole window
    // and keep the tauri fullscreen setting in sync
    addEventListener('fullscreenchange', e => ui.setFullscreen(!!document.fullscreenElement));

    // handle keybord shortcuts
    addEventListener('keydown', handleKeyboardEvents);

    // listen for new recordings
    __TAURI__.event.listen('reload_recordings', updateSidebar);
    __TAURI__.event.listen('new_recording_metadata', () => {
        const activeVideoId = ui.getActiveVideoId();
        if (activeVideoId) setMetadata(activeVideoId);
    });

    // load data
    ui.setCheckboxes(await tauri.getMarkerSettings());
    const videoIds = await updateSidebar();
    if (videoIds.length > 0) {
        setVideo(videoIds[0]);
        player.one('canplay', tauri.showWindow);
    } else {
        player.reset();
        player.ready(tauri.showWindow);
    }
}

// --- SIDEBAR, VIDEO PLAYER, DESCRIPTION  ---

async function updateSidebar() {
    const activeVideoId = ui.getActiveVideoId();

    const [videoIds, recordingsSize] = await Promise.all([tauri.getRecordingsNames(), tauri.getRecordingsSize()])
    ui.updateSideBar(recordingsSize, videoIds, setVideo, showRenameModal, showDeleteModal);

    if (!ui.setActiveVideoId(activeVideoId)) {
        player.reset();
    }

    return videoIds;
}
async function setVideo(videoId) {
    if (videoId === ui.getActiveVideoId()) {
        return;
    }

    player.src({ type: 'video/mp4', src: await tauri.getVideoPath(videoId) });
}

async function setMetadata(videoId) {
    const md = await tauri.getMetadata(videoId);
    if (md) {
        ui.setVideoDescriptionStats(md);
    } else {
        ui.setVideoDescription('', 'No Data');
    }

    currentEvents = md?.events ?? [];
    changeMarkers();
}

function changeMarkers() {
    const arr = [];
    const checkbox = ui.getCheckboxes();
    for (const e of currentEvents) {
        let visible = false;
        switch (e['name']) {
            case 'Kill':
                visible = checkbox.kill;
                break;
            case 'Death':
                visible = checkbox.death;
                break;
            case 'Assist':
                visible = checkbox.assist;
                break;
            case 'Turret':
                visible = checkbox.turret;
                break;
            case 'Inhibitor':
                visible = checkbox.inhibitor;
                break;
            case 'Infernal-Dragon':
            case 'Ocean-Dragon':
            case 'Mountain-Dragon':
            case 'Cloud-Dragon':
            case 'Hextech-Dragon':
            case 'Chemtech-Dragon':
            case 'Elder-Dragon':
                visible = checkbox.dragon;
                break;
            case 'Voidgrub':
            case 'Herald':
                visible = checkbox.herald;
                break;
            case 'Baron':
                visible = checkbox.baron;
                break;
            default:
                break;
        }
        if (visible) {
            arr.push({
                'time': e['time'] - EVENT_DELAY,
                'text': e['name'],
                'class': e['name']?.toLowerCase(),
                'duration': 5
            });
        }
    }
    player.markers().removeAll();
    player.markers().add(arr);
}

// --- MODAL ---

async function showRenameModal(videoId) {
    ui.showRenameModal(videoId, await tauri.getRecordingsNames(), renameVideo);
}

async function renameVideo(videoId, newVideoName) {
    if (videoId === ui.getActiveVideoId()) {
        // make sure the video is not in use before renaming it
        player.reset();
        await sleep(250);
    }

    const ok = await tauri.renameVideo(videoId, newVideoName + '.mp4');
    if (!ok) {
        ui.showErrorModal('Error renaming video!');
    }
}

function showDeleteModal(videoId) {
    ui.showDeleteModal(videoId, deleteVideo);
}

async function deleteVideo(videoId) {
    if (videoId === document.querySelector('#sidebar-content li.active')?.id) {
        // make sure the video is not in use before deleting it
        player.reset();
        await sleep(250);
    }

    const ok = await tauri.deleteVideo(videoId);
    if (!ok) {
        ui.showErrorModal('Error deleting video!');
    }
}

// --- KEYBOARD SHORTCUTS ---

function handleKeyboardEvents(event) {
    // if (ui.getActiveVideoId() === null || ui.modal.style.display === 'block') return;
    if (ui.getActiveVideoId() === null) return;

    switch (event.key) {
        case ' ':
            player.paused() ? player.play() : player.pause();
            break;
        case 'ArrowRight':
            event.shiftKey ? player.markers().next() : player.currentTime(player.currentTime() + 5);
            break;
        case 'ArrowLeft':
            event.shiftKey ? player.markers().prev() : player.currentTime(player.currentTime() - 5);
            break;
        case 'ArrowUp':
            player.volume(player.volume() + 0.1)
            break;
        case 'ArrowDown':
            player.volume(player.volume() - 0.1)
            break;
        case 'f':
        case 'F':
            // this only makes the videojs player fill the whole window
            // the listener for the 'fullscreenchange' event handles keeping the tauri window fullscreen status in sync
            player.isFullscreen() ? player.exitFullscreen() : player.requestFullscreen();
            break;
        case 'm':
        case 'M':
            player.muted(!player.muted());
            break;
        case '<':
            if (player.playbackRate() > 0.25)
                player.playbackRate(player.playbackRate() - 0.25);
            break;
        case '>':
            if (player.playbackRate() < 3)
                player.playbackRate(player.playbackRate() + 0.25);
            break;
        default:
            // return early to not call preventDefault()
            return;
    }
    event.preventDefault();
}
