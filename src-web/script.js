// CONSTANTS AND GLOBAL VARIABLES
const invoke = __TAURI__.invoke;
const join = __TAURI__.path.join;
const { emit, listen } = __TAURI__.event;
const open = __TAURI__.shell.open;
const wmng = new __TAURI__.window.WindowManager();

const POLLING_INTERVAL_MS = 5;

let sidebar = document.getElementById('sidebar-content');
let recordingsSize = document.getElementById('size');
let description = document.getElementById('description');

let init = true;
let fullscreen = false;
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
    'markerStyle': {
        'width': '8px',
        'border-radius': '5%'
    },
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

document.getElementById('vid-folder-btn').onclick = openRecordingsFolder;

// disable right click menu
addEventListener('contextmenu', event => event.preventDefault());
// prevent player from losing focus which causes keyboard controls to stop working
addEventListener('focusin', event => {
    event.preventDefault();
    player.focus();
});

// listen for new recordings
listen('new_recording', event => sidebar.innerHTML = createSidebarElement(event?.payload) + sidebar.innerHTML);
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
function createMarker(event) {
    return {
        'time': event['eventTime'] - POLLING_INTERVAL_MS,
        'text': event['eventName'],
        'class': event['eventName']?.toLowerCase()
    };
}
function setVideo(name) {
    if (!name) {
        wmng.setTitle('League Record');
        return;
    }
    getVideoPath(name)
        .then(path => player.src({ type: 'video/mp4', src: path }));
    wmng.setTitle('League Record - ' + name);
    invoke('get_metadata', { video: name }).then(md => {
        description.innerHTML = "No Data";
        player.markers.removeAll();
        if (md) {
            let desc = `${md['playerName']}<br>`;
            desc += `${md['gameMode']}<br>`;
            desc += `${md['championName']} - ${md['stats']['kills']}/${md['stats']['deaths']}/${md['stats']['assists']}<br>`;
            desc += `${md['stats']['creepScore']} CS | ${md['stats']['wardScore'].toString().substring(0, 4)} WS`;
            description.innerHTML = desc;

            // delay to wait for video src change to finish
            sleep(100).then(() => {
                let arr = [];
                md['events'].forEach(e => arr.push(createMarker(e)));
                player.markers.add(arr);
            });
        }
    });
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
// ------------------------------


// MAIN -------------------------
// load the inital content
sidebar.innerHTML = '';
getRecordingsNames().then(rec => {
    rec.forEach(el => sidebar.innerHTML += createSidebarElement(el));
    setVideo(rec[0]);
});
setRecordingsSize();
// ------------------------------
