// CONSTANTS AND GLOBAL VARIABLES
const invoke = window.__TAURI__.invoke
const { emit, listen } = window.__TAURI__.event;
const convertFileSrc = window.__TAURI__.tauri.convertFileSrc;
const open = __TAURI__.shell.open;
const wmng = new __TAURI__.window.WindowManager();
let init = true;
let fullscreen = false;

let recording = false;
let game_data = null;
// ------------------------------


// SETUP ------------------------
// init video player
const player = videojs('video_player', {
    aspectRatio: '16:9',
    autoplay: false,
    controls: true,
    preload: 'auto'
});

// set marker settings
player.markers({
    markerStyle: {
        'width': '10px',
        'background-color': 'red'
    },
    markers: []
});

// pause video on closing window to tray
listen('close_pause', () => player.pause());

// listen to fullscreenchange and set window fullscreen
addEventListener('fullscreenchange', e => {
    fullscreen = !fullscreen;
    wmng.setFullscreen(fullscreen);
});

// disable right click menu
addEventListener('contextmenu', event => event.preventDefault());
// ------------------------------


// RUST COMMUNICATION WRAPPERS --
function startRecording() {
    return invoke('record');
}
function stopRecording() {
    emit("stop_record");
}
function openRecordingsFolder() {
    invoke('get_recordings_folder').then(folder => open(folder));
}
function getRecordingsNames() {
    return invoke('get_recordings_list');
}
function setRecordingsSize() {
    invoke('get_recordings_size')
        .then(size => document.getElementById('size').innerHTML = `Size: ${size.toString().substring(0, 4)} GB`);
}
function getIngameData() {
    return invoke('get_league_data');
}
function saveMetadata(filename) {
    invoke('save_metadata', { "filename": filename, "json": game_data });
}
function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}
async function setVideo(name) {
    player.src({ type: 'video/mp4', src: convertFileSrc(name, 'video') });
    wmng.setTitle('League Record - ' + name);
    invoke('get_metadata', { video: name }).then(md => {
        document.getElementById('description').innerHTML = "No Data";
        player.markers.removeAll();
        if (md) {
            let desc = `${md['playerName']}<br>`;
            desc += `${md['gameMode']}<br>`;
            desc += `${md['championName']} - ${md['stats']['kills']}/${md['stats']['deaths']}/${md['stats']['assists']}<br>`;
            desc += `${md['stats']['creepScore']} CS | ${md['stats']['wardScore'].toString().substring(0, 4)} WS`;
            document.getElementById('description').innerHTML = desc;

            // delay to wait for video src change to finish
            sleep(100).then(() => {
                let arr = [];
                md['events'].forEach(e => arr.push({ time: e['EventTime'] - 5, text: e['EventName'] }));
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
                        document.querySelector('#sidebar-content li').click();
                    } else {
                        window.alert('Error deleting video!');
                    }
                });
            }
        });
}
// ------------------------------

// MAIN FUNCTIONS ---------------
function generateSidebarContent() {
    let sidebar = document.getElementById('sidebar-content');
    sidebar.innerHTML = '';
    getRecordingsNames().then(rec => {
        rec.forEach(el => sidebar.innerHTML += `<li id="${el}" onclick="setVideo('${el}')">${el.substring(0, el.length - 4)}<span class="close" onclick="deleteVideo('${el}')">&times;</span></li>`);
        if (init) {
            setVideo(rec[0]);
            init = false;
        }
    });
    setRecordingsSize();
}

function updateEvents() {
    getIngameData().then(data => {
        if (!recording && data) {
            recording = true;
            startRecording().then(filename => saveMetadata(filename));
        } else if (recording && !data) {
            recording = false;
            stopRecording();
        }
        if (data) game_data = data;
    });
}

// listen if a new video has been recorded
listen('recordings_changed', () => generateSidebarContent());
// load the inital content
generateSidebarContent();

// check regularly if league game is started
let interval = setInterval(updateEvents, 5000);
// ------------------------------
