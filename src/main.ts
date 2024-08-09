import 'video.js/dist/video-js.min.css';
import videojs from 'video.js';
import type Player from 'video.js/dist/types/player';
import { type MarkerOptions, MarkersPlugin, type Settings } from '@fffffffxxxxxxx/videojs-markers';

import { convertFileSrc } from '@tauri-apps/api/core';
import { join, sep } from '@tauri-apps/api/path';

import { commands, type GameEvent, type MarkerFlags } from './bindings';
import ListenerManager from './listeners';
import UI from './ui';
import { splitRight, UnreachableError } from './util';

// sets the time a marker jumps to before the actual event happens
// jumps to (eventTime - EVENT_DELAY) when a marker is clicked
const EVENT_DELAY = 2;

const ui = new UI(videojs);

type RecordingEvents = {
    participantId: number,
    recordingOffset: number
    events: Array<GameEvent>
}

let currentEvents: RecordingEvents | null = null;

const VIDEO_JS_OPTIONS = {
    aspectRatio: '16:9',
    playbackRates: [0.5, 1, 1.5, 2],
    autoplay: false,
    controls: true,
    preload: 'auto',
    enableSourceset: true,
    notSupportedMessage: ' '
}

const player = videojs('video_player', VIDEO_JS_OPTIONS) as Player & { markers: (settings?: Settings) => MarkersPlugin };

void main();
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
            minWidth: '6px',
            maxWidth: '16px',
            borderRadius: '0%'
        }
    });

    player.on('sourceset', ({ src }: { src: string }) => {
        // if src is a blank string that means no recording is selected
        if (src === '') {
            player.markers().removeAll();
            ui.setVideoDescription('', 'No recording selected!');
            ui.setActiveVideoId(null);

            // make sure the bigplaybutton and controlbar are hidden
            ui.showBigPlayButton(false);
            player.controls(false);
        } else {
            // split src ('https://asset.localhost/{path_to_file}') at the last '/' to get the video path from the src
            // to get the videoId split path/to/file.mp4 at the last directory separator which can be '/' or '\' (=> sep)
            // since videoId has to be a valid filename and filenames can't contain '/' this works always
            const videoPath = decodeURIComponent(splitRight(src, '/'));
            const videoId = splitRight(videoPath, sep());
            ui.setActiveVideoId(videoId);
            setMetadata(videoId);

            // re-show the bigplaybutton and controlbar when a new video src is set
            ui.showBigPlayButton(true);
            player.controls(true);
        }
    });

    // add events to html elements
    ui.setRecordingsFolderBtnOnClickHandler(commands.openRecordingsFolder);
    ui.setCheckboxOnClickHandler(() => {
        changeMarkers()
        commands.setMarkerFlags(ui.getMarkerFlags())
    });
    ui.setShowTimestampsOnClickHandler(showTimestamps);

    // listen if the videojs player fills the whole window
    // and keep the tauri fullscreen setting in sync
    addEventListener('fullscreenchange', _e => ui.setFullscreen(!!document.fullscreenElement));

    // handle keybord shortcuts
    addEventListener('keydown', handleKeyboardEvents);

    const listenerManager = new ListenerManager();
    listenerManager.listen_app('RecordingsChanged', updateSidebar);
    listenerManager.listen_app('MarkerflagsChanged', () => commands.getMarkerFlags().then(flags => ui.setMarkerFlags(flags)));
    listenerManager.listen_app('MetadataChanged', ({ payload }) => {
        const activeVideoId = ui.getActiveVideoId();
        if (activeVideoId !== null && payload.includes(activeVideoId)) {
            // update metadata for currently selected recording
            setMetadata(activeVideoId);
        }
    });

    // load data
    commands.getMarkerFlags().then(ui.setMarkerFlags);

    const videoIds = await updateSidebar();
    const firstVideo = videoIds[0];
    if (firstVideo) {
        void setVideo(firstVideo.videoId);
        player.one('canplay', ui.showWindow);
    } else {
        void setVideo(null);
        player.one("ready", ui.showWindow);
    }
}

// --- SIDEBAR, VIDEO PLAYER, DESCRIPTION  ---

// use this function to update the sidebar
async function updateSidebar() {
    const activeVideoId = ui.getActiveVideoId();

    const [recordings, recordingsSize] = await Promise.all([commands.getRecordingsList(), commands.getRecordingsSize()])
    ui.updateSideBar(recordingsSize, recordings, setVideo, commands.toggleFavorite, showRenameModal, showDeleteModal);

    if (!ui.setActiveVideoId(activeVideoId)) {
        void setVideo(null);
    }

    return recordings;
}

// use this function to set the video (null => no video)
async function setVideo(videoId: string | null) {
    if (videoId === ui.getActiveVideoId()) {
        return;
    }

    if (videoId === null) {
        player.src('');
    } else {
        const recordingsPath = await commands.getRecordingsPath();
        player.src({ type: 'video/mp4', src: convertFileSrc(await join(recordingsPath, videoId)) });
    }
}

async function setMetadata(videoId: string) {
    const data = await commands.getMetadata(videoId);
    if (data && 'Metadata' in data) {
        ui.showMarkerFlags(true);
        ui.setVideoDescriptionMetadata(data.Metadata);
        currentEvents = {
            participantId: data.Metadata.participantId,
            recordingOffset: data.Metadata.ingameTimeRecStartOffset,
            events: data.Metadata.events
        }
    } else {
        ui.showMarkerFlags(false);
        ui.setVideoDescription('', 'No Data');
        currentEvents = null;
    }

    changeMarkers();
}

function changeMarkers() {
    player.markers().removeAll();
    if (currentEvents === null) {
        return;
    }

    const checkbox = ui.getMarkerFlags();
    const { participantId, recordingOffset } = currentEvents;

    const markers = new Array<MarkerOptions>();
    for (const event of currentEvents.events) {
        markers.push(createMarker(event.timestamp, recordingOffset, eventName(event, participantId, checkbox)!));
    }

    player.markers().add(markers);
}

type EventType = 'Kill' | 'Death' | 'Assist' | 'Turret' | 'Inhibitor' | 'Voidgrub' | 'Herald' | 'Baron'
    | 'Infernal-Dragon' | 'Ocean-Dragon' | 'Mountain-Dragon' | 'Cloud-Dragon' | 'Hextech-Dragon' | 'Chemtech-Dragon' | 'Elder-Dragon';

function eventName(gameEvent: GameEvent, participantId: number, checkbox: MarkerFlags | null): EventType | null {
    if ('ChampionKill' in gameEvent) {
        if ((checkbox?.kill ?? true) && gameEvent.ChampionKill.killer_id === participantId) {
            return 'Kill';
        } else if ((checkbox?.assist ?? true) && gameEvent.ChampionKill.assisting_participant_ids.includes(participantId)) {
            return 'Assist';
        } else if ((checkbox?.death ?? true) && gameEvent.ChampionKill.victim_id === participantId) {
            return 'Death';
        }
    } else if ('BuildingKill' in gameEvent) {
        if ((checkbox?.turret ?? true) && 'TOWER_BUILDING' in gameEvent.BuildingKill.building_type) {
            return 'Turret';
        } else if ((checkbox?.inhibitor ?? true) && 'INHIBITOR_BUILDING' in gameEvent.BuildingKill.building_type) {
            return 'Inhibitor';
        }
    } else if ('EliteMonsterKill' in gameEvent) {
        const monsterType = gameEvent.EliteMonsterKill.monster_type;

        if ((checkbox?.herald ?? true) && monsterType.monsterType === 'HORDE') {
            return 'Voidgrub';
        } else if ((checkbox?.herald ?? true) && monsterType.monsterType === 'RIFTHERALD') {
            return 'Herald';
        } else if ((checkbox?.baron ?? true) && monsterType.monsterType === 'BARON_NASHOR') {
            return 'Baron';
        } else if ((checkbox?.dragon ?? true) && monsterType.monsterType === 'DRAGON') {
            switch (monsterType.monsterSubType) {
                case "FIRE_DRAGON":
                    return 'Infernal-Dragon';
                case "EARTH_DRAGON":
                    return 'Mountain-Dragon';
                case "WATER_DRAGON":
                    return 'Ocean-Dragon';
                case "AIR_DRAGON":
                    return 'Cloud-Dragon';
                case "HEXTECH_DRAGON":
                    return 'Hextech-Dragon';
                case "CHEMTECH_DRAGON":
                    return 'Chemtech-Dragon';
                case "ELDER_DRAGON":
                    return 'Elder-Dragon';
                default:
                    throw new UnreachableError(monsterType.monsterSubType);
            }
        }
    }

    return null;
}

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
    ui.showRenameModal(videoId, (await commands.getRecordingsList()).map(r => r.videoId), renameVideo);
}

async function renameVideo(videoId: string, newVideoId: string) {
    const activeVideoId = ui.getActiveVideoId();

    const ok = await commands.renameVideo(videoId, newVideoId);
    if (ok) {
        if (videoId === activeVideoId) {
            const time = player.currentTime()!;
            void updateSidebar();
            setVideo(newVideoId).then(() => player.currentTime(time));
        }
    } else {
        ui.showErrorModal('Error renaming video!');
    }
}

function showDeleteModal(videoId: string) {
    commands.confirmDelete().then(confirmDelete => {
        if (confirmDelete) {
            ui.showDeleteModal(videoId, deleteVideo);
        } else {
            deleteVideo(videoId);
        }
    });
}

async function deleteVideo(videoId: string) {
    if (videoId === ui.getActiveVideoId()) {
        player.src(null);
    }

    const ok = await commands.deleteVideo(videoId);
    if (!ok) {
        ui.showErrorModal('Error deleting video!');
    }
}

function showTimestamps() {
    if (currentEvents === null) return;

    const timelineEvents = new Array<{ timestamp: number, text: string }>();
    for (const event of currentEvents.events) {
        const name = eventName(event, currentEvents.participantId, null);
        if (name === null) {
            continue;
        }

        let secs = event.timestamp / 1000;

        let minutes = Math.floor(secs / 60);
        secs -= minutes * 60;

        const hours = Math.floor(minutes / 60);
        minutes -= hours * 60;

        const timestamp = event.timestamp;
        const text = `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${Math.floor(secs).toString().padStart(2, '0')} ${name}`;
        timelineEvents.push({ timestamp, text });
    }

    ui.showTimelineModal(timelineEvents, secs => player.currentTime(secs / 1000 - EVENT_DELAY));
}

function formatTimestamp(gameEvent: GameEvent, participantId: number): string | null {
    const name = eventName(gameEvent, participantId, null);
    if (name === null) {
        return null;
    }

    let secs = gameEvent.timestamp / 1000;

    let minutes = Math.floor(secs / 60);
    secs -= minutes * 60;

    const hours = Math.floor(minutes / 60);
    minutes -= hours * 60;

    return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${Math.floor(secs).toString().padStart(2, '0')} ${name}`;
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
