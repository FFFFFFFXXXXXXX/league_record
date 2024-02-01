const windowManager = new __TAURI__.window.WindowManager('main');

async function setFullscreen(fullscreen) {
    await windowManager.setFullscreen(fullscreen);
}

function setWindowTitle(title) {
    windowManager.setTitle('League Record - ' + title);
}

const modal = document.getElementById('modal');
const modalContent = document.getElementById('modal-content');
const sidebar = document.getElementById('sidebar-content');
const videoFolderBtn = document.getElementById('vid-folder-btn');
const recordingsSize = document.getElementById('size-inner');
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

function setRecordingsFolderBtnOnClickHandler(handler) {
    videoFolderBtn.onclick = handler;
}

function setCheckboxOnClickHandler(handler) {
    checkboxKill.onclick = handler;
    checkboxDeath.onclick = handler;
    checkboxAssist.onclick = handler;
    checkboxTurret.onclick = handler;
    checkboxInhibitor.onclick = handler;
    checkboxDragon.onclick = handler;
    checkboxHerald.onclick = handler;
    checkboxBaron.onclick = handler;
}

function updateSideBar(recordingsSizeGb, filenames, onVideo, onRename, onDelete) {
    filenames = filenames.map(el => {
        // call event.stopPropagation(); to stop the onclick event from also effecting the element under the clicked X button
        const renameBtn = videojs.dom.createEl(
            'span',
            { 'onclick': ev => { ev.stopPropagation(); onRename(el) } },
            { 'class': 'rename' },
            '✎'
        );
        const deleteBtn = videojs.dom.createEl(
            'span',
            { 'onclick': ev => { ev.stopPropagation(); onDelete(el) } },
            { 'class': 'delete' },
            '×'
        );
        return videojs.dom.createEl(
            'li',
            { 'onclick': () => onVideo(el) },
            { 'id': el },
            [el.slice(0, -4), renameBtn, deleteBtn]
        );
    });

    videojs.dom.insertContent(sidebar, filenames);
    videojs.dom.insertContent(recordingsSize, recordingsSizeGb);
}

function showModal(content) {
    videojs.dom.insertContent(modalContent, content);
    modal.style.display = 'block';
}

function hideModal() {
    modal.style.display = 'none';
}

async function showErrorModal(text) {
    showModal([
        videojs.dom.createEl('p', {}, {}, text),
        videojs.dom.createEl('p', {}, {}, videojs.dom.createEl('button', { 'onclick': hideModal }, { 'class': 'btn' }, 'Close')),
    ]);
}

async function showRenameModal(videoId, filenames, rename) {
    const videoName = videoId.slice(0, -4);

    const input = videojs.dom.createEl(
        'input',
        {},
        {
            'type': 'text',
            'id': 'new-name',
            'value': videoName,
            'placeholder': 'new name',
            'spellcheck': 'false',
            'autocomplete': 'off'
        }
    );
    const saveButton = videojs.dom.createEl(
        'button',
        {
            'onclick': e => {
                if (input.validity.valid) {
                    e.preventDefault();
                    hideModal();
                    rename(videoId, input.value);
                }
            }
        },
        { 'class': 'btn', 'disabled': true },
        'Save'
    );
    const cancelButton = videojs.dom.createEl(
        'button',
        { 'onclick': hideModal },
        { 'class': 'btn' },
        'Cancel'
    );

    showModal([
        videojs.dom.createEl('p', {}, {}, ['Change name of: ', videojs.dom.createEl('u', {}, {}, videoName)]),
        videojs.dom.createEl('p', {}, {}, input),
        videojs.dom.createEl('p', {}, {}, [saveButton, cancelButton])
    ]);

    input.addEventListener('input', _ => {
        if (filenames.includes(input.value + '.mp4')) {
            input.setCustomValidity('there is already a file with this name');
            saveButton.setAttribute('disabled', 'true');
        } else {
            input.setCustomValidity('');
            saveButton.removeAttribute('disabled');
        }

        input.reportValidity();
    })

    input.setSelectionRange(input.value.length, input.value.length);
    input.focus();
}

async function showDeleteModal(videoId, deleteVideo) {
    const videoName = videoId.slice(0, -4);

    const prompt = videojs.dom.createEl('p', {}, {}, ['Delete recording: ', videojs.dom.createEl('u', {}, {}, videoName), '?']);
    const buttons = videojs.dom.createEl('p', {}, {}, [
        videojs.dom.createEl('button', {
            'onclick': _ => {
                hideModal();
                deleteVideo(videoId);
            }
        }, { 'class': 'btn' }, 'Delete'),
        videojs.dom.createEl('button', { 'onclick': hideModal }, { 'class': 'btn' }, 'Cancel'),
    ]);

    showModal([prompt, buttons]);
}

function getActiveVideoId() {
    return sidebar.querySelector('li.active')?.id;
}

function setActiveVideoId(videoId) {
    sidebar.querySelector('li.active')?.classList.remove('active');
    const videoLi = sidebar.querySelector(`[id='${videoId}']`);
    videoLi?.classList.add('active');
    return videoLi !== null;
}

function setVideoDescription(left, center) {
    videojs.dom.insertContent(descriptionLeft, left);
    videojs.dom.insertContent(descriptionCenter, center);
}

function setVideoDescriptionStats(md) {
    const stats = md['stats'];

    const summoner = videojs.dom.createEl(
        'span',
        {},
        { 'class': 'summoner-name' },
        md['gameInfo']['summonerName']
    );
    const score1 = `${md['gameInfo']['championName']} - ${stats['kills']}/${stats['deaths']}/${stats['assists']}`;
    const score2 = `${stats['minionsKilled'] + stats['neutralMinionsKilled']} CS | ${stats['wardScore'].toString().substring(0, 4)} WS`;

    const gameMode = `Game Mode: ${md['gameInfo']['gameMode']}`;
    const result = md['win'] !== null && (
        md['win'] ?
            videojs.dom.createEl('span', {}, { 'class': 'win' }, 'Victory')
            : videojs.dom.createEl('span', {}, { 'class': 'loss' }, 'Defeat')
    );

    setVideoDescription(
        [summoner, videojs.dom.createEl('br'), score1, videojs.dom.createEl('br'), score2],
        [gameMode, videojs.dom.createEl('br'), result]
    );
}

function setCheckboxes(settings) {
    checkboxKill.checked = settings.kill;
    checkboxDeath.checked = settings.death;
    checkboxAssist.checked = settings.assist;
    checkboxTurret.checked = settings.turret;
    checkboxInhibitor.checked = settings.inhibitor;
    checkboxDragon.checked = settings.dragon;
    checkboxHerald.checked = settings.herald;
    checkboxBaron.checked = settings.baron;
}

function getCheckboxes() {
    return {
        kill: checkboxKill.checked,
        death: checkboxDeath.checked,
        assist: checkboxAssist.checked,
        turret: checkboxTurret.checked,
        inhibitor: checkboxInhibitor.checked,
        dragon: checkboxDragon.checked,
        herald: checkboxHerald.checked,
        baron: checkboxBaron.checked,
    };
}

export default {
    setWindowTitle: setWindowTitle,
    setFullscreen: setFullscreen,

    setRecordingsFolderBtnOnClickHandler: setRecordingsFolderBtnOnClickHandler,
    setCheckboxOnClickHandler: setCheckboxOnClickHandler,

    showErrorModal: showErrorModal,
    showRenameModal: showRenameModal,
    showDeleteModal: showDeleteModal,
    updateSideBar: updateSideBar,
    getActiveVideoId: getActiveVideoId,
    setActiveVideoId: setActiveVideoId,
    setVideoDescription: setVideoDescription,
    setVideoDescriptionStats: setVideoDescriptionStats,
    setCheckboxes: setCheckboxes,
    getCheckboxes: getCheckboxes
};
