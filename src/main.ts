import 'video.js/dist/video-js.min.css';
import videojs from 'video.js';
import type Player from 'video.js/dist/types/player';
import { MarkersPlugin, type Settings, type MarkerOptions } from '@fffffffxxxxxxx/videojs-markers';
import type { AppEvent, GameEvent } from '@fffffffxxxxxxx/league_record_types';

import { listen, type EventCallback, type UnlistenFn } from '@tauri-apps/api/event';
import * as tauri from './bindings';

import UI from './ui';
import { splitRight, toVideoName } from './util';
import { convertFileSrc } from '@tauri-apps/api/tauri';
import { sep, join } from '@tauri-apps/api/path';

// sets the time a marker jumps to before the actual event happens
// jumps to (eventTime - EVENT_DELAY) when a marker is clicked
const EVENT_DELAY = 3;

const ui = new UI(videojs);

type RecordingEvents = {
    participantId: number,
    recordingOffset: number
    events: Array<GameEvent>
}

let currentEvents: RecordingEvents | null = null;

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
    ui.setCheckboxOnClickHandler(() => {
        changeMarkers()
        tauri.setMarkerFlags(ui.getMarkerFlags())
    });

    // listen if the videojs player fills the whole window
    // and keep the tauri fullscreen setting in sync
    addEventListener('fullscreenchange', _e => ui.setFullscreen(!!document.fullscreenElement));

    // handle keybord shortcuts
    addEventListener('keydown', handleKeyboardEvents);

    // wrapper function to typecheck subscribed events against TS bindings (type AppEvent)
    function listen_event<T>(event: AppEvent, callback: EventCallback<T>): Promise<UnlistenFn> {
        return listen<T>(event, callback);
    }

    listen_event<void>('RecordingsChanged', updateSidebar);
    listen_event<void>('MarkerflagsChanged', () => tauri.getMarkerFlags().then(flags => ui.setCheckboxes(flags)));
    listen_event<Array<string>>('MetadataChanged', ({ payload }) => {
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
        setVideo(firstVideo.videoId);
        player.one('canplay', ui.showWindow);
    } else {
        setVideo(null);
        player.ready(ui.showWindow);
    }
}

// --- SIDEBAR, VIDEO PLAYER, DESCRIPTION  ---

// use this function to update the sidebar
async function updateSidebar() {
    const activeVideoId = ui.getActiveVideoId();

    const [recordings, recordingsSize] = await Promise.all([tauri.getRecordingsList(), tauri.getRecordingsSize()])
    ui.updateSideBar(recordingsSize, recordings, setVideo, toggleFavorite, showRenameModal, showDeleteModal);

    if (!ui.setActiveVideoId(activeVideoId)) {
        setVideo(null);
    }

    return recordings;
}

// use this function to set the video (null => no video)
async function setVideo(videoId: string | null) {
    if (videoId === null) {
        player.reset();
        return;
    }

    if (videoId === ui.getActiveVideoId()) {
        return;
    }

    const recordingsPath = await tauri.getRecordingsPath();
    player.src({ type: 'video/mp4', src: convertFileSrc(await join(recordingsPath, videoId)) });
}

async function setMetadata(videoId: string) {
    const data = await tauri.getMetadata(videoId);
    if (data && 'Metadata' in data) {
        ui.setVideoDescriptionMetadata(data.Metadata);
        currentEvents = {
            participantId: data.Metadata.participantId,
            recordingOffset: data.Metadata.ingameTimeRecStartOffset,
            events: data.Metadata.events
        }
    } else {
        ui.setVideoDescription('', 'No Data');
        currentEvents = null;
    }

    changeMarkers();
}

async function toggleFavorite(video_id: string): Promise<boolean | null> {
    return await tauri.toggleFavorite(video_id)
}

function changeMarkers() {
    player.markers().removeAll();
    if (currentEvents === null) {
        return;
    }

    const checkbox = ui.getMarkerFlags();
    const { participantId, recordingOffset } = currentEvents;

    const markers = new Array<MarkerOptions>();
    for (const gameEvent of currentEvents.events) {
        const { timestamp, event } = gameEvent;

        if ('ChampionKill' in event) {
            if (checkbox.kill && event.ChampionKill.killer_id === participantId) {
                markers.push(createMarker(timestamp, recordingOffset, 'Kill'));
            } else if (checkbox.assist && event.ChampionKill.assisting_participant_ids.includes(participantId)) {
                markers.push(createMarker(timestamp, recordingOffset, 'Assist'));
            } else if (checkbox.death && event.ChampionKill.victim_id === participantId) {
                markers.push(createMarker(timestamp, recordingOffset, 'Death'));
            }
        } else if ('BuildingKill' in event) {
            if (checkbox.turret && 'TOWER_BUILDING' in event.BuildingKill.building_type) {
                markers.push(createMarker(timestamp, recordingOffset, 'Turret'));
            } else if (checkbox.inhibitor && 'INHIBITOR_BUILDING' in event.BuildingKill.building_type) {
                markers.push(createMarker(timestamp, recordingOffset, 'Inhibitor'));
            }
        } else if ('EliteMonsterKill' in event) {
            const monsterType = event.EliteMonsterKill.monster_type;

            if (checkbox.herald && monsterType.monsterType === 'HORDE') {
                markers.push(createMarker(timestamp, recordingOffset, 'Voidgrub'));
            } else if (checkbox.herald && monsterType.monsterType === 'RIFTHERALD') {
                markers.push(createMarker(timestamp, recordingOffset, 'Herald'));
            } else if (checkbox.baron && monsterType.monsterType === 'BARON_NASHOR') {
                markers.push(createMarker(timestamp, recordingOffset, 'Baron'));
            } else if (checkbox.dragon && monsterType.monsterType === 'DRAGON') {
                switch (monsterType.monsterSubType) {
                    case "FIRE_DRAGON":
                        markers.push(createMarker(timestamp, recordingOffset, 'Infernal-Dragon'));
                        break;
                    case "EARTH_DRAGON":
                        markers.push(createMarker(timestamp, recordingOffset, 'Mountain-Dragon'));
                        break;
                    case "WATER_DRAGON":
                        markers.push(createMarker(timestamp, recordingOffset, 'Ocean-Dragon'));
                        break;
                    case "AIR_DRAGON":
                        markers.push(createMarker(timestamp, recordingOffset, 'Cloud-Dragon'));
                        break;
                    case "HEXTECH_DRAGON":
                        markers.push(createMarker(timestamp, recordingOffset, 'Hextech-Dragon'));
                        break;
                    case "CHEMTECH_DRAGON":
                        markers.push(createMarker(timestamp, recordingOffset, 'Chemtech-Dragon'));
                        break;
                    case "ELDER_DRAGON":
                        markers.push(createMarker(timestamp, recordingOffset, 'Elder-Dragon'));
                        break;
                }
            }
        }
    }

    player.markers().add(markers);
}

type EventType = 'Kill' | 'Death' | 'Assist' | 'Turret' | 'Inhibitor' | 'Voidgrub' | 'Herald' | 'Baron'
    | 'Infernal-Dragon' | 'Ocean-Dragon' | 'Mountain-Dragon' | 'Cloud-Dragon' | 'Hextech-Dragon' | 'Chemtech-Dragon' | 'Elder-Dragon';

function createMarker(timestamp: number, recordingOffset: number, eventType: EventType): MarkerOptions {
    return {
        time: (timestamp / 1000 - recordingOffset) - EVENT_DELAY,
        text: eventType,
        class: eventType.toLowerCase(),
        duration: 2 * EVENT_DELAY
    };
}

// --- MODAL ---

async function showRenameModal(videoId: string) {
    ui.showRenameModal(videoId, (await tauri.getRecordingsList()).map(r => r.videoId), renameVideo);
}

async function renameVideo(videoId: string, newVideoId: string) {
    const activeVideoId = ui.getActiveVideoId();
    let time = null;
    if (videoId === activeVideoId) {
        time = player.currentTime()!;
    }

    const ok = await tauri.renameVideo(videoId, newVideoId);
    if (ok) {
        if (time !== null) {
            await updateSidebar();
            await setVideo(newVideoId);
            player.currentTime(time);
        }
    } else {
        ui.showErrorModal('Error renaming video!');
    }
}

function showDeleteModal(videoId: string) {
    ui.showDeleteModal(videoId, deleteVideo);
}

async function deleteVideo(videoId: string) {
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
