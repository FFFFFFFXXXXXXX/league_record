const invoke = window.__TAURI__.invoke
const event = window.__TAURI__.event;
const convertFileSrc = window.__TAURI__.tauri.convertFileSrc;
const open = __TAURI__.shell.open;
const wmng = new __TAURI__.window.WindowManager();
let init = true;
let fullscreen = false;

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
event.listen('close_pause', () => player.pause());

// listen to fullscreenchange and set window fullscreen
addEventListener('fullscreenchange', e => {
    fullscreen = !fullscreen;
    wmng.setFullscreen(fullscreen);
});

function setVideo(name) {
    player.src({ type: 'video/mp4', src: convertFileSrc(name, 'video') });
    wmng.setTitle('League Record - ' + name);
}

function startRecording() {
    invoke('record').then(m => console.log(m));
}

function stopRecording() {
    event.emit("stop_record", {});
}

function openRecordingsFolder() {
    invoke('get_recordings_folder').then(folder => open(folder));
}

function deleteVideo(video) {
    if (window.confirm(`Do you really want to delete ${video}`)) {
        invoke('delete_video', { video: video }).then(b => {
            if (b)
                generateSidebarContent();
            else
                window.alert('Error deleting video!');
        });
    }
}

function generateSidebarContent() {
    let sidebar = document.getElementById('sidebar-content');
    sidebar.innerHTML = '';
    invoke('get_recordings_list')
        .then(rec => {
            rec.forEach(el => sidebar.innerHTML += `<a onclick="setVideo('${el}')">${el.substring(0, el.length - 4)}</a><hr>`);
            if (init) {
                setVideo(rec[0]);
                init = false;
            }
        })
        .then(createContextMenus);
}

function createContextMenus() {
    document.querySelectorAll('#sidebar-content a').forEach(el => {
        new VanillaContextMenu({
            scope: el,
            menuItems: [
                {
                    label: 'Delete',
                    callback: () => deleteVideo(el.innerHTML + '.mp4'),
                }
            ],
            transitionDuration: 0
        });
    });
}

// listen if a new video has been recorded
event.listen('recordings_changed', () => generateSidebarContent());
// load the inital content
generateSidebarContent();