import 'video.js/dist/video-js.min.css';
import videojs from 'video.js';
import type Player from 'video.js/dist/types/player';
import { MarkersPlugin, type Settings, type MarkerOptions } from '@fffffffxxxxxxx/videojs-markers';

import { appWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import * as tauri from './bindings';

import UI from './ui';
import { sleep, splitRight, toVideoName } from './util';
import { convertFileSrc } from '@tauri-apps/api/tauri';
import { sep, join } from '@tauri-apps/api/path';

// sets the time a marker jumps to before the actual event happens
// jumps to (eventTime - EVENT_DELAY) when a marker is clicked
const EVENT_DELAY = 3;

const ui = new UI(videojs, appWindow);

let currentEvents = new Array<tauri.GameEvent>();

const player = videojs('video_player', {
    aspectRatio: '16:9',
    playbackRates: [0.5, 1, 1.5, 2],
    autoplay: false,
    controls: true,
    preload: 'auto',
    enableSourceset: true
}) as Player & { markers: (settings?: Settings) => MarkersPlugin };

main();
async function main() {
    // disable right click menu
    addEventListener('contextmenu', event => event.preventDefault());

    // configure and start marker plugin
    player.markers({
        markerTip: {
            display: true,
            innerHtml: marker => marker.text ?? '',
        },
        markerStyle: {
            minWidth: '4px',
            maxWidth: '15px',
            borderRadius: '30%'
        }
    });

    // listen for events from player.reset() and player.src() to update the UI accordingly
    player.on('playerreset', () => {
        player.markers().removeAll();
        ui.setVideoDescription('', 'No recording selected!');
        ui.setActiveVideoId(null);

        // make sure the bigplaybutton and controlbar are hidden when resetting the video src
        ui.showBigPlayButton(false);
        player.controls(false);
    });

    player.on('sourceset', ({ src }: { src: string }) => {
        // ignore all sources that are falsy (e.g. null, undefined, empty string)
        // because player.reset() for example triggers a 'sourceset' event with { src: "" }
        if (!src) return;

        // split src ('https://asset.localhost/{path_to_file}') at the last '/' to get the video path from the src
        // to get the videoId split path/to/file.mp4 at the last directory separator which can be '/' or '\' (=> sep)
        // since videoId has to be a valid filename and filenames can't contain '/' this works always
        const videoPath = decodeURIComponent(splitRight(src, '/'));
        const videoId = splitRight(videoPath, sep);
        ui.setActiveVideoId(videoId);
        setMetadata(videoId);

        // re-show the bigplaybutton and controlbar when a new video src is set
        ui.showBigPlayButton(true);
        player.controls(true);
    });

    // add events to html elements
    ui.setRecordingsFolderBtnOnClickHandler(tauri.openRecordingsFolder);
    ui.setCheckboxOnClickHandler(changeMarkers);

    // listen if the videojs player fills the whole window
    // and keep the tauri fullscreen setting in sync
    addEventListener('fullscreenchange', _e => ui.setFullscreen(!!document.fullscreenElement));

    // handle keybord shortcuts
    addEventListener('keydown', handleKeyboardEvents);

    listen<void>('recordings_changed', updateSidebar);
    listen<Array<string>>('metadata_changed', ({ payload }) => {
        const activeVideoId = ui.getActiveVideoId();
        if (activeVideoId !== null && payload.includes(toVideoName(activeVideoId))) {
            // update metadata for currently selected recording
            setMetadata(activeVideoId);
        }
    });

    // load data
    ui.setCheckboxes(await tauri.getMarkerFlags());
    const videoIds = await updateSidebar();
    const firstVideo = videoIds[0];
    if (firstVideo) {
        setVideo(firstVideo);
        player.one('canplay', tauri.showAppWindow);
    } else {
        player.reset();
        player.ready(tauri.showAppWindow);
    }
}

// --- SIDEBAR, VIDEO PLAYER, DESCRIPTION  ---

async function updateSidebar() {
    const activeVideoId = ui.getActiveVideoId();

    const [videoIds, recordingsSize] = await Promise.all([tauri.getRecordingsList(), tauri.getRecordingsSize()])
    ui.updateSideBar(recordingsSize, videoIds, setVideo, showRenameModal, showDeleteModal);

    if (!ui.setActiveVideoId(activeVideoId)) {
        player.reset();
    }

    return videoIds;
}
async function setVideo(videoId: string) {
    if (videoId === ui.getActiveVideoId()) {
        return;
    }

    const recordingsPath = await tauri.getRecordingsPath();
    player.src({ type: 'video/mp4', src: convertFileSrc(await join(recordingsPath, videoId)) });
}

async function setMetadata(videoId: string) {
    const data = await tauri.getMetadata(videoId);
    if (data) {
        ui.setVideoDescriptionStats(data);
    } else {
        ui.setVideoDescription('', 'No Data');
    }

    currentEvents = data?.events ?? [];
    changeMarkers();
}

function changeMarkers() {
    const arr = new Array<MarkerOptions>();
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
            case 'InfernalDragon':
            case 'OceanDragon':
            case 'MountainDragon':
            case 'CloudDragon':
            case 'HextechDragon':
            case 'ChemtechDragon':
            case 'ElderDragon':
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
                time: e['time'] - EVENT_DELAY,
                text: e['name'],
                class: e['name']?.toLowerCase(),
                duration: 5
            });
        }
    }
    player.markers().removeAll();
    player.markers().add(arr);
    tauri.setMarkerFlags({
        kill: checkbox.kill,
        death: checkbox.death,
        assist: checkbox.assist,
        turret: checkbox.turret,
        inhibitor: checkbox.inhibitor,
        dragon: checkbox.dragon,
        herald: checkbox.herald,
        baron: checkbox.baron
    });
}

// --- MODAL ---

async function showRenameModal(videoId: string) {
    ui.showRenameModal(videoId, await tauri.getRecordingsList(), renameVideo);
}

async function renameVideo(videoId: string, newVideoName: string) {
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

function showDeleteModal(videoId: string) {
    ui.showDeleteModal(videoId, deleteVideo);
}

async function deleteVideo(videoId: string) {
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

function handleKeyboardEvents(event: KeyboardEvent) {
    if (ui.modalIsOpen()) {
        switch (event.key) {
            case 'Escape':
                ui.hideModal();
                break;
            default:
                // return early to not call preventDefault()
                return;
        }
        event.preventDefault();
    } else {
        if (ui.getActiveVideoId() === null) return;

        switch (event.key) {
            case ' ':
            case 'Enter':
                player.paused() ? player.play() : player.pause();
                break;
            case 'ArrowRight':
                event.shiftKey ? player.markers().next() : player.currentTime(player.currentTime()! + 5);
                break;
            case 'ArrowLeft':
                event.shiftKey ? player.markers().prev() : player.currentTime(player.currentTime()! - 5);
                break;
            case 'ArrowUp':
                player.volume(player.volume()! + 0.1)
                break;
            case 'ArrowDown':
                player.volume(player.volume()! - 0.1)
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
                if (player.playbackRate()! > 0.25)
                    player.playbackRate(player.playbackRate()! - 0.25);
                break;
            case '>':
                if (player.playbackRate()! < 3)
                    player.playbackRate(player.playbackRate()! + 0.25);
                break;
            default:
                // return early to not call preventDefault()
                return;
        }
        event.preventDefault();
    }
}
