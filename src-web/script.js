// CONSTANTS AND GLOBAL VARIABLES

// sets the time a marker jumps to before the actual event happens
// jump to (eventTime - EVENT_DELAY) when a marker is clicked
const EVENT_DELAY = 3;

const tauriWindowManager = new __TAURI__.window.WindowManager();

const modal = document.getElementById('modal');
const modalContent = document.getElementById('modal-content');
const sidebar = document.getElementById('sidebar-content');
const recordingsSize = document.getElementById('size');
const descriptionLeft = document.getElementById('description-left');
const descriptionCenter = document.getElementById('description-center');

const checkboxKill = document.getElementById('kill');
const checkboxDeath = document.getElementById('death');
const checkboxAssist = document.getElementById('assist');
const checkboxTurret = document.getElementById('turret');
const checkboxInhibitor = document.getElementById('inhibitor');
const checkboxDragon = document.getElementById('dragon');
const checkboxHerald = document.getElementById('herald');
const checkboxBaron = document.getElementById('baron');

let fullscreen = false;
let currentEvents = [];
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
addEventListener('fullscreenchange', () => {
    fullscreen = !fullscreen;
    tauriWindowManager.setFullscreen(fullscreen);
});

addEventListener('keydown', event => {
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
            // return early to not call preventDefault()
            return;
    }
    event.preventDefault();
});

// set event markers on video load
player.tech_.contentEl().onloadedmetadata = changeMarkers;

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
__TAURI__.event.listen('new_recording', async () => {
    let activeVideo = document.querySelector('.active')?.id;

    let rec = await getRecordingsNames();
    sidebar.innerHTML = '';
    rec.forEach(el => sidebar.innerHTML += createSidebarElement(el));

    document.getElementById(activeVideo)?.classList.add('active');

    await setRecordingsSize();
});

// listen for settings change
__TAURI__.event.listen('reload_ui', async () => {
    await init();
});
// ------------------------------


// FUNCTIONS --------------------
const entityMap = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;',
    '/': '&#x2F;',
    '`': '&#x60;',
    '=': '&#x3D;'
};

// inspired by https://github.com/janl/mustache.js/blob/master/mustache.js#L55
function escape(string) {
    return String(string)
        .replace(/[&<>"'`=\/]/g, s => entityMap[s])
        .replace(/[\r\n]+/g, ' ');
}

function resetPlayer() {
    player.reset();
    player.tech_.contentEl().onloadedmetadata = changeMarkers;
}

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

function hideModal() {
    modal.style.display = 'none';
}

async function getVideoPath(video) {
    let port = await __TAURI__.invoke('get_asset_port');
    return `http://127.0.0.1:${port}/${video}`;
}

function openRecordingsFolder() {
    __TAURI__.invoke('open_recordings_folder');
}

async function getRecordingsNames() {
    return (await __TAURI__.invoke('get_recordings_list')).map(escape);
}

async function setRecordingsSize() {
    let size = await __TAURI__.invoke('get_recordings_size');
    recordingsSize.innerHTML = `Size: ${size.toString().substring(0, 4)} GB`;
}

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

async function getDefaultMarkerSettings() {
    return await __TAURI__.invoke('get_default_marker_flags');
}

async function getCurrentMarkerSettings() {
    return await __TAURI__.invoke('get_current_marker_flags');
}

function setCurrentMarkerSettings(markers) {
    __TAURI__.invoke('set_current_marker_flags', {markerFlags: markers});
}

function clearData() {
    player.markers.removeAll();
    currentEvents = [];
    descriptionLeft.innerHTML = '';
    descriptionCenter.innerHTML = 'No Data';
}

function setVideo(name) {
    document.querySelector('.active')?.classList.remove('active');
    document.getElementById(name)?.classList.add('active');

    if (name) {
        tauriWindowManager.setTitle('League Record - ' + name);
    } else {
        tauriWindowManager.setTitle('League Record');
        return;
    }

    __TAURI__.invoke('get_metadata', {video: name}).then(md => {
        if (md) {
            try {
                currentEvents = md['events'];

                let descLeft = `<span class="summoner-name">${escape(md['gameInfo']['summonerName'])}</span><br>`;
                descLeft += `${escape(md['gameInfo']['championName'])} - ${escape(md['stats']['kills'])}/${escape(md['stats']['deaths'])}/${escape(md['stats']['assists'])}<br>`;
                descLeft += `${escape(md['stats']['creepScore'])} CS | ${escape(md['stats']['wardScore'].toString().substring(0, 4))} WS`;
                // descLeft += `Map: ${md['gameInfo']['gameMode']}`;
                descriptionLeft.innerHTML = descLeft;

                let descCenter = `Map: ${escape(md['gameInfo']['gameMode'])}<br>`;
                if (md['win'] != null) {
                    descCenter += md['win'] ? '<span class="win">Victory</span><br>' : '<span class="loss">Defeat</span>';
                }
                descriptionCenter.innerHTML = descCenter;
            } catch {
                clearData();
            }
        } else {
            clearData();
        }
    });

    getVideoPath(name).then(path => player.src({type: 'video/mp4', src: path}));
}

// ! possibly change this when moving back to Tauri Asset Protocol
async function deleteVideo(video) {
    let deleteCurrentVideo = video === document.querySelector('.active').id;
    if (deleteCurrentVideo) {
        // make sure the video is not in use before deleting it
        resetPlayer();
        await sleep(100);
    }

    let ok = await __TAURI__.invoke('delete_video', {'video': video});
    if (ok) {
        await setRecordingsSize();
        document.getElementById(video).remove();

        // only set new active video if old active video was deleted
        if (deleteCurrentVideo) {
            let remainingVideos = sidebar.querySelectorAll('li');
            if (remainingVideos.length > 0) setVideo(remainingVideos[0].id);
        }
    } else {
        let content = '<p>Error deleting video!</p>';
        content += '<p><button class="btn" onclick="hideModal();">Close</button></p>';
        showModal(content);
    }
}

function createSidebarElement(el) {
    // call event.stopPropagation(); to stop the onclick event from also effecting the element under the clicked X button
    let deleteBtn = `<span class="delete" onclick="event.stopPropagation();showDeleteModal('${el}')">&times;</span>`;
    return `<li id="${el}" onclick="setVideo('${el}')">${escape(el.substring(0, el.length - 4))}${deleteBtn}</li>`;
}

function changeMarkers() {
    player.markers.removeAll();
    let arr = [];
    currentEvents.forEach(e => {
        let visible = false;
        switch (e['name']) {
            case 'Kill':
                visible = checkboxKill.checked;
                break;
            case 'Death':
                visible = checkboxDeath.checked;
                break;
            case 'Assist':
                visible = checkboxAssist.checked;
                break;
            case 'Turret':
                visible = checkboxTurret.checked;
                break;
            case 'Inhibitor':
                visible = checkboxInhibitor.checked;
                break;
            case 'Infernal-Dragon':
            case 'Ocean-Dragon':
            case 'Mountain-Dragon':
            case 'Cloud-Dragon':
            case 'Hextech-Dragon':
            case 'Chemtech-Dragon':
            case 'Elder-Dragon':
                visible = checkboxDragon.checked;
                break;
            case 'Herald':
                visible = checkboxHerald.checked;
                break;
            case 'Baron':
                visible = checkboxBaron.checked;
                break;
            default:
                break;
        }
        if (visible) {
            arr.push({
                'time': e['time'] - EVENT_DELAY,
                'text': e['name'],
                'class': e['name']?.toLowerCase(),
                'duration': 4
            });
        }
    });
    player.markers.add(arr);
    setCurrentMarkerSettings({
        kill: checkboxKill.checked,
        death: checkboxDeath.checked,
        assist: checkboxAssist.checked,
        turret: checkboxTurret.checked,
        inhibitor: checkboxInhibitor.checked,
        dragon: checkboxDragon.checked,
        herald: checkboxHerald.checked,
        baron: checkboxBaron.checked,
    });
}

async function init() {
    let rec = await getRecordingsNames();
    sidebar.innerHTML = '';
    rec.forEach(el => sidebar.innerHTML += createSidebarElement(el));
    setVideo(rec[0]);

    let settings = await getCurrentMarkerSettings() ?? await getDefaultMarkerSettings();
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
    await __TAURI__.invoke('show_app_window');
}

// ------------------------------


// MAIN -------------------------
// load the initial content
init().then(() => console.log('window loaded'));
