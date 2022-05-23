// CONSTANTS AND GLOBAL VARIABLES
const invoke = __TAURI__.invoke;
const { emit, listen } = __TAURI__.event;
const open = __TAURI__.shell.open;
const wmng = new __TAURI__.window.WindowManager();

// this needs to be the same as SLEEP_SECS in lol_rec::main
const EVENT_DELAY = 1;

let modal = document.getElementById('modal');
let modalContent = document.getElementById('modal-content');
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
listen('new_recording', init);
// ------------------------------


// FUNCTIONS --------------------
function showDeleteModal(video) {
    let html = `<p>Do you really want to delete ${video}?</p>`;
    html += '<p>';
    html += `<button class="btn" onclick="hideModal();deleteVideo('${video}');">Yes</button>`;
    html += `<button class="btn" onclick="hideModal()">No</button>`;
    html += '</p>';

    showModal(html);
}
function showModal(content) {
    modalContent.innerHTML = content;
    modal.style.display = 'block';
}
function hideModal(event) {
    modal.style.display = 'none';
}
async function getVideoPath(video) {
    let port = await invoke('get_asset_port');
    return `http://localhost:${port}/${video}`;
}
async function openRecordingsFolder() {
    open(await invoke('get_recordings_folder'));
}
function getRecordingsNames() {
    return invoke('get_recordings_list');
}
async function setRecordingsSize() {
    let size = await invoke('get_recordings_size');
    recordingsSize.innerHTML = `Size: ${size.toString().substring(0, 4)} GB`;
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
    document.querySelector('.active')?.classList.remove('active');
    document.getElementById(name)?.classList.add('active');

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
async function deleteVideo(video) {
    let deleteCurrentVideo = video === document.querySelector('.active').id;
    if (deleteCurrentVideo) {
        // make sure the video is not in use before deleting it
        player.src({});
        await sleep(250);
    }

    let ok = await invoke('delete_video', { 'video': video });
    if (ok) {
        setRecordingsSize();
        document.getElementById(video).remove();
        if (deleteCurrentVideo) {
            // only set new active video if old active video was deleted
            let newVideo = sidebar.querySelector('li')?.id;
            setVideo(newVideo);
        }
    } else {
        let content = '<p>Error deleting video!</p>';
        content += '<p><button class="btn" onclick="hideModal();">Close</button></p>';
        showModal(content);
    }
}
function test(e) {
    console.log(e);
    e.stopPropagation();
}
function createSidebarElement(el) {
    let deleteBtn = `<span class="delete" onclick="event.stopPropagation();showDeleteModal('${el}')">&times;</span>`;
    return `<li id="${el}" onclick="setVideo('${el}')">${el.substring(0, el.length - 4)}${deleteBtn}</li>`;
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
            case 'Fire Dragon':
            case 'Water Dragon':
            case 'Earth Dragon':
            case 'Air Dragon':
            case 'Hextech Dragon':
            case 'Elder Dragon':
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

async function init() {
    let rec = await getRecordingsNames();
    sidebar.innerHTML = '';
    rec.forEach(el => sidebar.innerHTML += createSidebarElement(el));
    setVideo(rec[0]);

    let settings = await getMarkerSettings();
    checkboxKill.checked = settings.kill;
    checkboxDeath.checked = settings.death;
    checkboxAssist.checked = settings.assist;
    checkboxTurret.checked = settings.turret;
    checkboxInhibitor.checked = settings.inhibitor;
    checkboxDragon.checked = settings.dragon;
    checkboxHerald.checked = settings.herald;
    checkboxBaron.checked = settings.baron;

    await setRecordingsSize();

    await sleep(150); // delay so the initial blank screen when creating a window doesn't show
    wmng.show();
}
// ------------------------------


// MAIN -------------------------
// load the inital content
init();
// ------------------------------
