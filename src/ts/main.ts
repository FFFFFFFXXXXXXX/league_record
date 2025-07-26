import videojs from "video.js";
import type Player from "video.js/dist/types/player";
import { type MarkerOptions, MarkersPlugin, type Settings } from "@fffffffxxxxxxx/videojs-markers";

import { convertFileSrc } from "@tauri-apps/api/core";
import { join, sep } from "@tauri-apps/api/path";

import { commands, type GameEvent, type MarkerFlags } from "./bindings";
import ListenerManager from "./listeners";
import UI from "./ui";
import { splitRight, UnreachableError } from "./util";

// sets the time a marker jumps to before the actual event happens
// jumps to (eventTime - EVENT_DELAY) when a marker is clicked
const EVENT_DELAY = 2;

const ui = new UI(videojs);

type RecordingEvents = {
    participantId: number;
    recordingOffset: number;
    events: Array<GameEvent>;
};

type HighlightEvents = {
    recordingOffset: number;
    events: Array<number>;
};

let currentEvents: RecordingEvents | null = null;
let highlightEvents: HighlightEvents | null = null;

const VIDEO_JS_OPTIONS = {
    aspectRatio: "16:9",
    playbackRates: [0.5, 1, 1.5, 2],
    autoplay: false,
    controls: true,
    preload: "auto",
    enableSourceset: true,
    notSupportedMessage: " ",
};

const player = videojs("video_player", VIDEO_JS_OPTIONS) as Player & {
    markers: (settings?: Settings) => MarkersPlugin;
};

console.log(MarkersPlugin);

void main();
async function main() {
    // disable right click menu
    addEventListener("contextmenu", (event) => event.preventDefault());

    // configure and start marker plugin
    player.markers({
        markerTip: {
            display: true,
            innerHtml: (marker) => marker.text ?? "",
        },
        markerStyle: {
            minWidth: "6px",
            maxWidth: "16px",
            borderRadius: "0%",
        },
    });

    player.on("sourceset", ({ src }: { src: string }) => {
        // if src is a blank string that means no recording is selected
        if (src === "") {
            player.markers().removeAll();
            ui.setVideoDescription("", "No recording selected!");
            ui.setActiveVideoId(null);

            // make sure the bigplaybutton and controlbar are hidden
            ui.showBigPlayButton(false);
            player.controls(false);
        } else {
            // split src ('https://asset.localhost/{path_to_file}') at the last '/' to get the video path from the src
            // to get the videoId split path/to/file.mp4 at the last directory separator which can be '/' or '\' (=> sep)
            // since videoId has to be a valid filename and filenames can't contain '/' this works always
            const videoPath = decodeURIComponent(splitRight(src, "/"));
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
        changeMarkers();
        commands.setMarkerFlags(ui.getMarkerFlags());
    });
    ui.setShowTimestampsOnClickHandler(showTimestamps);

    // listen if the videojs player fills the whole window
    // and keep the tauri fullscreen setting in sync
    addEventListener("fullscreenchange", (_e) => ui.setFullscreen(!!document.fullscreenElement));

    // handle keybord shortcuts
    addEventListener("keydown", handleKeyboardEvents);

    const listenerManager = new ListenerManager();
    listenerManager.listen_app("RecordingsChanged", updateSidebar);
    listenerManager.listen_app("MarkerflagsChanged", () =>
        commands.getMarkerFlags().then((flags) => ui.setMarkerFlags(flags)),
    );
    listenerManager.listen_app("MetadataChanged", ({ payload }) => {
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
        player.one("canplay", ui.showWindow);
    } else {
        void setVideo(null);
        player.one("ready", ui.showWindow);
    }
}

// --- SIDEBAR, VIDEO PLAYER, DESCRIPTION  ---

// use this function to update the sidebar
async function updateSidebar() {
    const activeVideoId = ui.getActiveVideoId();

    const [recordings, recordingsSize] = await Promise.all([
        commands.getRecordingsList(),
        commands.getRecordingsSize(),
    ]);
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
        player.src("");
    } else {
        const recordingsPath = await commands.getRecordingsPath();
        player.src({ type: "video/mp4", src: convertFileSrc(await join(recordingsPath, videoId)) });
    }
}

async function setMetadata(videoId: string) {
    const data = await commands.getMetadata(videoId);
    if (data && "Metadata" in data) {
        ui.showMarkerFlags(true);
        ui.setVideoDescriptionMetadata(data.Metadata);
        currentEvents = {
            participantId: data.Metadata.participantId,
            recordingOffset: data.Metadata.ingameTimeRecStartOffset,
            events: data.Metadata.events,
        };
        highlightEvents = {
            recordingOffset: data.Metadata.ingameTimeRecStartOffset,
            events: data.Metadata.highlights,
        };
    } else if (data && "Deferred" in data) {
        ui.showMarkerFlags(false);
        ui.setVideoDescription("", "No Data");
        currentEvents = null;
        highlightEvents = {
            recordingOffset: data.Deferred.ingameTimeRecStartOffset,
            events: data.Deferred.highlights,
        };
    } else {
        ui.showMarkerFlags(false);
        ui.setVideoDescription("", "No Data");
        currentEvents = null;
        highlightEvents = null;
    }

    changeMarkers();
}

function changeMarkers() {
    const markers = new Array<MarkerOptions>();

    if (highlightEvents !== null) {
        for (const event of highlightEvents.events) {
            markers.push(createMarker(event, highlightEvents.recordingOffset, "Highlight"));
        }
    }

    if (currentEvents !== null) {
        const checkbox = ui.getMarkerFlags();
        const { participantId, recordingOffset } = currentEvents;

        for (const event of currentEvents.events) {
            const name = eventName(event, participantId, checkbox);
            if (name === null) {
                continue;
            }
            markers.push(createMarker(event.timestamp, recordingOffset, name));
        }
    }

    player.markers().removeAll();
    player.markers().add(markers);
}

type EventType =
    | "Kill"
    | "Death"
    | "Assist"
    | "Turret"
    | "Inhibitor"
    | "Voidgrub"
    | "Herald"
    | "Atakhan"
    | "Baron"
    | "Infernal-Dragon"
    | "Ocean-Dragon"
    | "Mountain-Dragon"
    | "Cloud-Dragon"
    | "Hextech-Dragon"
    | "Chemtech-Dragon"
    | "Elder-Dragon"
    | "Highlight";

function eventName(gameEvent: GameEvent, participantId: number, checkbox: MarkerFlags | null): EventType | null {
    if ("ChampionKill" in gameEvent) {
        if ((checkbox?.kill ?? true) && gameEvent.ChampionKill.killer_id === participantId) {
            return "Kill";
        }
        if ((checkbox?.assist ?? true) && gameEvent.ChampionKill.assisting_participant_ids.includes(participantId)) {
            return "Assist";
        }
        if ((checkbox?.death ?? true) && gameEvent.ChampionKill.victim_id === participantId) {
            return "Death";
        }
    } else if ("BuildingKill" in gameEvent) {
        if ((checkbox?.structure ?? true) && "TOWER_BUILDING" in gameEvent.BuildingKill.building_type) {
            return "Turret";
        }
        if ((checkbox?.structure ?? true) && "INHIBITOR_BUILDING" in gameEvent.BuildingKill.building_type) {
            return "Inhibitor";
        }
    } else if ("EliteMonsterKill" in gameEvent) {
        const monsterType = gameEvent.EliteMonsterKill.monster_type;
        if ((checkbox?.herald ?? true) && monsterType.monsterType === "HORDE") {
            return "Voidgrub";
        }
        if ((checkbox?.herald ?? true) && monsterType.monsterType === "RIFTHERALD") {
            return "Herald";
        }
        if ((checkbox?.atakhan ?? true) && monsterType.monsterType === "ATAKHAN") {
            return "Atakhan";
        }
        if ((checkbox?.baron ?? true) && monsterType.monsterType === "BARON_NASHOR") {
            return "Baron";
        }
        if ((checkbox?.dragon ?? true) && monsterType.monsterType === "DRAGON") {
            switch (monsterType.monsterSubType) {
                case "FIRE_DRAGON":
                    return "Infernal-Dragon";
                case "EARTH_DRAGON":
                    return "Mountain-Dragon";
                case "WATER_DRAGON":
                    return "Ocean-Dragon";
                case "AIR_DRAGON":
                    return "Cloud-Dragon";
                case "HEXTECH_DRAGON":
                    return "Hextech-Dragon";
                case "CHEMTECH_DRAGON":
                    return "Chemtech-Dragon";
                case "ELDER_DRAGON":
                    return "Elder-Dragon";
                default:
                    throw new UnreachableError(monsterType.monsterSubType);
            }
        }
    }

    return null;
}

function createMarker(timestamp: number, recordingOffset: number, eventType: EventType): MarkerOptions {
    return {
        time: timestamp / 1000 - recordingOffset - EVENT_DELAY,
        text: eventType,
        class: eventType.toLowerCase(),
        duration: 2 * EVENT_DELAY,
    };
}

// --- MODAL ---

async function showRenameModal(videoId: string) {
    ui.showRenameModal(
        videoId,
        (await commands.getRecordingsList()).map((r) => r.videoId),
        renameVideo,
    );
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
        ui.showErrorModal("Error renaming video!");
    }
}

function showDeleteModal(videoId: string) {
    commands.confirmDelete().then((confirmDelete) => {
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
        ui.showErrorModal("Error deleting video!");
    }
}

function showTimestamps() {
    const timelineEvents = new Array<{ timestamp: number; text: string }>();

    if (highlightEvents !== null) {
        for (const event of highlightEvents.events) {
            timelineEvents.push({ timestamp: event, text: `${formatTimestamp(event)} Highlight` });
        }
    }

    if (currentEvents !== null) {
        for (const event of currentEvents.events) {
            const name = eventName(event, currentEvents.participantId, null);
            if (name !== null) {
                const text = `${formatTimestamp(event.timestamp)} ${name}`;
                const timestamp = event.timestamp;
                timelineEvents.push({ timestamp, text });
            }
        }
    }

    ui.showTimelineModal(
        timelineEvents.sort((a, b) => a.timestamp - b.timestamp),
        (secs) => player.currentTime(secs / 1000 - EVENT_DELAY),
    );
}

function formatTimestamp(timestamp: number): string {
    let secs = timestamp / 1000;

    let minutes = Math.floor(secs / 60);
    secs -= minutes * 60;

    const hours = Math.floor(minutes / 60);
    minutes -= hours * 60;

    return `${hours.toString().padStart(2, "0")}:${minutes.toString().padStart(2, "0")}:${Math.floor(secs).toString().padStart(2, "0")}`;
}

// --- KEYBOARD SHORTCUTS ---

function handleKeyboardEvents(event: KeyboardEvent) {
    if (ui.modalIsOpen()) {
        switch (event.key) {
            case "Escape":
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
            case " ":
            case "Enter":
                player.paused() ? player.play() : player.pause();
                break;
            case "ArrowRight":
                event.shiftKey ? player.markers().next() : player.currentTime(player.currentTime()! + 5);
                break;
            case "ArrowLeft":
                event.shiftKey ? player.markers().prev() : player.currentTime(player.currentTime()! - 5);
                break;
            case "ArrowUp":
                player.volume(player.volume()! + 0.1);
                break;
            case "ArrowDown":
                player.volume(player.volume()! - 0.1);
                break;
            case "f":
            case "F":
                // this only makes the videojs player fill the whole window
                // the listener for the 'fullscreenchange' event handles keeping the tauri window fullscreen status in sync
                player.isFullscreen() ? player.exitFullscreen() : player.requestFullscreen();
                break;
            case "m":
            case "M":
                player.muted(!player.muted());
                break;
            case "<":
                if (player.playbackRate()! > 0.25) player.playbackRate(player.playbackRate()! - 0.25);
                break;
            case ">":
                if (player.playbackRate()! < 3) player.playbackRate(player.playbackRate()! + 0.25);
                break;
            default:
                // return early to not call preventDefault()
                return;
        }
        event.preventDefault();
    }
}
