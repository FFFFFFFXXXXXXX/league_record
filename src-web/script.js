// CONSTANTS AND GLOBAL VARIABLES
const invoke = __TAURI__.invoke;
const join = __TAURI__.path.join;
const { emit, listen } = __TAURI__.event;
const open = __TAURI__.shell.open;
const wmng = new __TAURI__.window.WindowManager();

const EVENT_DELAY = 3;

let sidebar = document.getElementById('sidebar-content');
let recordingsSize = document.getElementById('size');
let descriptionName = document.getElementById('description-name');
let descriptionContent = document.getElementById('description-content');

let checkboxKill = document.getElementById('kill');
let checkboxDeath = document.getElementById('death');
let checkboxAssist = document.getElementById('assist');
let checkboxTurret = document.getElementById('turret');
let checkboxInhibitor = document.getElementById('inhibitor');
let checkboxDragon = document.getElementById('dragon');
let checkboxHerald = document.getElementById('herald');
let checkboxBaron = document.getElementById('baron');

let init = true;
let fullscreen = false;
let currentEvents = [];
let currentDataDelay = 0;
// ------------------------------


// SETUP ------------------------
// init video player
const player = videojs('video_player', {
    'aspectRatio': '16:9',
    'playbackRates': [0.5, 1, 1.5, 2],
    'autoplay': false,
    'controls': true,
    'preload': 'auto'
});

// set marker settings
player.markers({
    'markerTip': {
        'display': true,
        'text': (marker) => marker.text,
        'time': (marker) => marker.time,
    },
    'markers': []
});

// pause video on closing window to tray
listen('close_pause', () => player.pause());

// listen to fullscreenchange and set window fullscreen
addEventListener('fullscreenchange', e => {
    fullscreen = !fullscreen;
    wmng.setFullscreen(fullscreen);
});

addEventListener('keydown', event => {
    let preventDefault = true;
    switch (event.key) {
        case ' ':
            player.paused() ? player.play() : player.pause();
            break;
        case 'ArrowRight':
            event.shiftKey ? player.markers.next() : player.currentTime(player.currentTime() + 5);
            break;
        case 'ArrowLeft':
            event.shiftKey ? player.markers.prev() : player.currentTime(player.currentTime() - 5);
            break;
        case 'ArrowUp':
            player.volume(player.volume() + 0.1)
            break;
        case 'ArrowDown':
            player.volume(player.volume() - 0.1)
            break;
        case 'f':
            player.isFullscreen() ? player.exitFullscreen() : player.requestFullscreen();
            break;
        case 'm':
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
            preventDefault = false;
            break;
    }
    if (preventDefault)
        event.preventDefault();
});

// add events to html elements
document.getElementById('vid-folder-btn').onclick = openRecordingsFolder;
checkboxKill.onclick = changeMarkers;
checkboxDeath.onclick = changeMarkers;
checkboxAssist.onclick = changeMarkers;
checkboxTurret.onclick = changeMarkers;
checkboxInhibitor.onclick = changeMarkers;
checkboxDragon.onclick = changeMarkers;
checkboxHerald.onclick = changeMarkers;
checkboxBaron.onclick = changeMarkers;

// disable right click menu
addEventListener('contextmenu', event => event.preventDefault());
// prevent player from losing focus which causes keyboard controls to stop working
addEventListener('focusin', event => {
    event.preventDefault();
    player.focus();
});

// listen for new recordings
listen('new_recording', event => {
    setRecordingsSize();
    sidebar.innerHTML = createSidebarElement(event?.payload) + sidebar.innerHTML;
});
// ------------------------------


// FUNCTIONS --------------------
async function getVideoPath(video) {
    let port = await invoke('get_asset_port');
    return await join(`http://localhost:${port}/`, video);
}
function openRecordingsFolder() {
    invoke('get_recordings_folder').then(folder => open(folder));
}
function getRecordingsNames() {
    return invoke('get_recordings_list');
}
function setRecordingsSize() {
    invoke('get_recordings_size')
        .then(size => recordingsSize.innerHTML = `Size: ${size.toString().substring(0, 4)} GB`);
}
function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}
async function getMarkerSettings() {
    return await invoke('get_marker_flags');
}
function createMarker(event, dataDelay) {
    let delay = dataDelay ? dataDelay : 0;
    return {
        'time': event['eventTime'] + delay - EVENT_DELAY,
        'text': event['eventName'],
        'class': event['eventName']?.toLowerCase(),
        'duration': 4
    };
}
async function setVideo(name) {
    if (!name) {
        wmng.setTitle('League Record');
        return;
    } else {
        wmng.setTitle('League Record - ' + name);
    }
    let path = await getVideoPath(name);
    player.src({ type: 'video/mp4', src: path });

    let md = await invoke('get_metadata', { video: name });
    if (md) {
        currentEvents = md['events'];
        currentDataDelay = md['dataDelay'];

        let descName = `<span class="summoner-name">${md['playerName']}</span><br>`;
        descName += `${md['gameMode']}<br>`;
        descriptionName.innerHTML = descName;

        let result = '';
        switch (md['result']) {
            case 'Win':
                result = '<span class="win">Victory</span><br>';
                break;
            case 'Lose':
                result = '<span class="loss">Defeat</span><br>';
                break;
            default:
                break;
        }
        let descContent = result;
        descContent += `${md['championName']} - ${md['stats']['kills']}/${md['stats']['deaths']}/${md['stats']['assists']}<br>`;
        descContent += `${md['stats']['creepScore']} CS | ${md['stats']['wardScore'].toString().substring(0, 4)} WS`;
        descriptionContent.innerHTML = descContent;

        // wait for player src change to finish before adding markers
        setTimeout(changeMarkers, 250);
    } else {
        player.markers.removeAll();
        descriptionName.innerHTML = ''
        descriptionContent.innerHTML = 'No Data';
    }
}
function deleteVideo(video) {
    window.confirm(`Do you really want to delete ${video}`)
        .then(ok => {
            if (ok) {
                invoke('delete_video', { video: video }).then(b => {
                    if (b) {
                        setRecordingsSize();
                        document.getElementById(video).remove();
                        let newVideo = document.querySelector('#sidebar-content li')?.id;
                        setVideo(newVideo);
                    } else {
                        window.alert('Error deleting video!');
                    }
                });
            }
        });
}
function createSidebarElement(el) {
    return `<li id="${el}" onclick="setVideo('${el}')">${el.substring(0, el.length - 4)}<span class="close" onclick="deleteVideo('${el}')">&times;</span></li>`;
}
function changeMarkers() {
    player.markers.removeAll();
    let arr = [];
    currentEvents.forEach(e => {
        let ok = false;
        switch (e['eventName']) {
            case 'Kill':
                ok = checkboxKill.checked;
                break;
            case 'Death':
                ok = checkboxDeath.checked;
                break;
            case 'Assist':
                ok = checkboxAssist.checked;
                break;
            case 'Turret':
                ok = checkboxTurret.checked;
                break;
            case 'Inhibitor':
                ok = checkboxInhibitor.checked;
                break;
            case 'Dragon':
                ok = checkboxDragon.checked;
                break;
            case 'Herald':
                ok = checkboxHerald.checked;
                break;
            case 'Baron':
                ok = checkboxBaron.checked;
                break;
            default:
                break;
        }
        if (ok) arr.push(createMarker(e, currentDataDelay));
    });
    player.markers.add(arr);
}
// ------------------------------


// MAIN -------------------------
// load the inital content
getRecordingsNames().then(rec => {
    sidebar.innerHTML = '';
    rec.forEach(el => sidebar.innerHTML += createSidebarElement(el));
    setVideo(rec[0]);
});
getMarkerSettings().then(settings => {
    checkboxKill.checked = settings.kill;
    checkboxDeath.checked = settings.death;
    checkboxAssist.checked = settings.assist;
    checkboxTurret.checked = settings.turret;
    checkboxInhibitor.checked = settings.inhibitor;
    checkboxDragon.checked = settings.dragon;
    checkboxHerald.checked = settings.herald;
    checkboxBaron.checked = settings.baron;
});
setRecordingsSize();
// ------------------------------
