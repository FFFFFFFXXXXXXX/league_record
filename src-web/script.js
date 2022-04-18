const invoke = window.__TAURI__.invoke
const { emit, listen } = window.__TAURI__.event;
const convertFileSrc = window.__TAURI__.tauri.convertFileSrc;
const open = __TAURI__.shell.open;
const wmng = new __TAURI__.window.WindowManager();
let init = true;
let fullscreen = false;

let events = [];

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

function setVideo(name) {
    player.src({ type: 'video/mp4', src: convertFileSrc(name, 'video') });
    wmng.setTitle('League Record - ' + name);
    invoke('get_metadata').then(metadata => {
        document.getElementById('description').innerHTML = metadata;
    });
}

function startRecording() {
    invoke('record').then(m => { if (m) console.error(m) });
}

function stopRecording() {
    emit("stop_record", {});
}

function openRecordingsFolder() {
    invoke('get_recordings_folder').then(folder => open(folder));
}

function setRecordingsSize() {
    invoke('get_recordings_size')
        .then(size => document.getElementById('size').innerHTML = `Size: ${size.toString().substring(0, 4)} GB`);
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

function generateSidebarContent() {
    let sidebar = document.getElementById('sidebar-content');
    sidebar.innerHTML = '';
    invoke('get_recordings_list')
        .then(rec => {
            rec.forEach(el => sidebar.innerHTML += `<li id="${el}" onclick="setVideo('${el}')">${el.substring(0, el.length - 4)}<span class="close" onclick="deleteVideo('${el}')">&times;</span></li>`);
            if (init) {
                setVideo(rec[0]);
                init = false;
            }
        });
    setRecordingsSize();
}

function updateEvents() {
    invoke('get_league_events').then(e => {
        if (events.length === 0 && e.length > 0) {
            startRecording();
        } else if ((events.length > 0 && e.length === 0) || e.includes('GameEnd')) {
            stopRecording();
            // todo save events
        }
        events = e;
    });
}

// disable right click menu
addEventListener('contextmenu', event => event.preventDefault());

// listen if a new video has been recorded
listen('recordings_changed', () => generateSidebarContent());

// load the inital content
generateSidebarContent();

// check regularly if league game is started
let interval = setInterval(updateEvents, 5000);
// clearInterval(interval);