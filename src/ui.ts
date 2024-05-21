import type videojs from 'video.js';
import type { ContentDescriptor } from 'video.js/dist/types/utils/dom';
import type { MarkerFlags, GameMetadata, Recording, MetadataFile } from '@fffffffxxxxxxx/league_record_types';
import { toVideoId, toVideoName } from './util';
import { appWindow } from '@tauri-apps/api/window';

export default class UI {
    private readonly modal;
    private readonly modalContent;
    private readonly sidebar;
    private readonly videoFolderBtn;
    private readonly recordingsSize;
    private readonly descriptionLeft;
    private readonly descriptionCenter;

    private readonly checkboxKill;
    private readonly checkboxDeath;
    private readonly checkboxAssist;
    private readonly checkboxTurret;
    private readonly checkboxInhibitor;
    private readonly checkboxDragon;
    private readonly checkboxHerald;
    private readonly checkboxBaron;

    private readonly vjs: typeof videojs;

    private readonly boundHideModal;

    constructor(vjs: typeof videojs) {
        this.vjs = vjs;
        this.boundHideModal = this.hideModal.bind(this);

        this.modal = document.querySelector<HTMLDivElement>('[id="modal"]')!;
        this.modalContent = document.querySelector<HTMLDivElement>('[id="modal-content"]')!;
        this.sidebar = document.querySelector<HTMLUListElement>('[id="sidebar-content"]')!;
        this.videoFolderBtn = document.querySelector<HTMLButtonElement>('[id="vid-folder-btn"]')!;
        this.recordingsSize = document.querySelector<HTMLSpanElement>('[id="size-inner"]')!;
        this.descriptionLeft = document.querySelector<HTMLDivElement>('[id="description-left"]')!;
        this.descriptionCenter = document.querySelector<HTMLDivElement>('[id="description-center"]')!;

        this.checkboxKill = document.querySelector<HTMLInputElement>('[id="kill"]')!;
        this.checkboxDeath = document.querySelector<HTMLInputElement>('[id="death"]')!;
        this.checkboxAssist = document.querySelector<HTMLInputElement>('[id="assist"]')!;
        this.checkboxTurret = document.querySelector<HTMLInputElement>('[id="turret"]')!;
        this.checkboxInhibitor = document.querySelector<HTMLInputElement>('[id="inhibitor"]')!;
        this.checkboxDragon = document.querySelector<HTMLInputElement>('[id="dragon"]')!;
        this.checkboxHerald = document.querySelector<HTMLInputElement>('[id="herald"]')!;
        this.checkboxBaron = document.querySelector<HTMLInputElement>('[id="baron"]')!;
    }

    public setWindowTitle(title: string) {
        appWindow.setTitle('League Record - ' + title);
    }

    public showWindow() {
        appWindow.show();
    }

    public setFullscreen(fullscreen: boolean) {
        appWindow.setFullscreen(fullscreen);
    }

    public setRecordingsFolderBtnOnClickHandler(handler: (e: MouseEvent) => void) {
        this.videoFolderBtn.onclick = handler;
    }

    public setCheckboxOnClickHandler(handler: (e: MouseEvent) => void) {
        this.checkboxKill.onclick = handler;
        this.checkboxDeath.onclick = handler;
        this.checkboxAssist.onclick = handler;
        this.checkboxTurret.onclick = handler;
        this.checkboxInhibitor.onclick = handler;
        this.checkboxDragon.onclick = handler;
        this.checkboxHerald.onclick = handler;
        this.checkboxBaron.onclick = handler;
    }

    public updateSideBar(
        recordingsSizeGb: number,
        recordings: ReadonlyArray<Recording>,
        onVideo: (videoId: string) => void,
        onFavorite: (videoId: string) => Promise<boolean | null>,
        onRename: (videoId: string) => void,
        onDelete: (videoId: string) => void
    ) {
        function isFavorite(metadataFile: MetadataFile | null): boolean {
            if (!metadataFile) return false;
            if ('Metadata' in metadataFile) return metadataFile.Metadata.favorite;
            if ('Deferred' in metadataFile) return metadataFile.Deferred.favorite;
            return false;
        }

        const videoLiElements = recordings.map(recording => {
            const videoName = toVideoName(recording.videoId);

            // call event.stopPropagation(); to stop the onclick event from also effecting the element under the clicked X button
            const favorite = isFavorite(recording.metadata);
            const favoriteBtn = this.vjs.dom.createEl(
                'span',
                {
                    onclick: (e: MouseEvent) => {
                        e.stopPropagation();
                        onFavorite(recording.videoId).then(fav => {
                            if (fav !== null) {
                                favoriteBtn.innerHTML = fav ? '★' : '☆'
                                favoriteBtn.style.color = fav ? 'gold' : ''
                            }
                        })
                    }
                },
                {
                    class: 'favorite',
                    ...(favorite ? { style: 'color: gold' } : {})
                },
                favorite ? '★' : '☆'
            ) as HTMLSpanElement;

            const renameBtn = this.vjs.dom.createEl(
                'span',
                {
                    onclick: (e: MouseEvent) => {
                        e.stopPropagation();
                        onRename(recording.videoId);
                    }
                },
                { class: 'rename' },
                '✎'
            );
            const deleteBtn = this.vjs.dom.createEl(
                'span',
                {
                    onclick: (e: MouseEvent) => {
                        e.stopPropagation();
                        onDelete(recording.videoId);
                    }
                },
                { class: 'delete' },
                '×'
            );
            return this.vjs.dom.createEl(
                'li',
                { onclick: () => onVideo(recording.videoId) },
                { id: recording.videoId },
                [
                    this.vjs.dom.createEl('span', {}, { class: 'video-name' }, videoName),
                    favoriteBtn,
                    renameBtn,
                    deleteBtn
                ]
            );
        });

        this.vjs.dom.insertContent(this.sidebar, videoLiElements);
        this.vjs.dom.insertContent(this.recordingsSize, recordingsSizeGb.toFixed(2).toString());
    }

    public showModal(content: ContentDescriptor) {
        this.vjs.dom.insertContent(this.modalContent, content);
        this.modal.style.display = 'block';
    }

    public hideModal() {
        this.vjs.dom.emptyEl(this.modalContent);
        this.modal.style.display = 'none';
    }

    public modalIsOpen() {
        return this.modal.style.display === 'block';
    }

    public async showErrorModal(text: string) {
        this.showModal([
            this.vjs.dom.createEl('p', {}, {}, text),
            this.vjs.dom.createEl('p', {}, {}, this.vjs.dom.createEl('button', { onclick: this.boundHideModal }, { class: 'btn' }, 'Close')),
        ]);
    }

    public async showRenameModal(
        videoId: string,
        videoIds: ReadonlyArray<string>,
        rename: (videoId: string, newVideoId: string) => void
    ) {
        const videoName = toVideoName(videoId);

        const input = this.vjs.dom.createEl(
            'input',
            {},
            {
                type: 'text',
                id: 'new-name',
                value: videoName,
                placeholder: 'new name',
                spellcheck: 'false',
                autocomplete: 'off'
            }
        ) as HTMLInputElement;

        // set validity checker initial value and add 'input' event listener
        const validityChecker = (_e: Event) => {
            if (videoIds.includes(toVideoId(input.value))) {
                input.setCustomValidity('there is already a file with this name');
                saveButton.setAttribute('disabled', 'true');
            } else {
                input.setCustomValidity('');
                saveButton.removeAttribute('disabled');
            }

            input.reportValidity();
        };
        input.addEventListener('input', validityChecker)
        input.setCustomValidity('there is already a file with this name');
        input.reportValidity();

        const renameHandler = (e: KeyboardEvent | MouseEvent) => {
            // if the event is a KeyboardEvent also check if the key pressed was 'enter'
            const keyboardEvent = 'key' in e;
            if (input.checkValidity() && (!keyboardEvent || e.key === 'Enter')) {
                e.preventDefault();
                this.boundHideModal();
                rename(videoId, toVideoId(input.value));

                // clean up eventlisteners for this renameHandler and the validityChecker
                input.removeEventListener('keydown', renameHandler);
                input.removeEventListener('input', validityChecker);
            }
        };
        input.addEventListener('keydown', renameHandler)

        const saveButton = this.vjs.dom.createEl(
            'button',
            {
                onclick: renameHandler
            },
            { class: 'btn', disabled: true },
            'Save'
        ) as HTMLButtonElement;
        const cancelButton = this.vjs.dom.createEl(
            'button',
            { onclick: this.boundHideModal },
            { class: 'btn' },
            'Cancel'
        ) as HTMLButtonElement;

        this.showModal([
            this.vjs.dom.createEl('p', {}, {}, ['Change name of: ', this.vjs.dom.createEl('u', {}, {}, videoName)]),
            this.vjs.dom.createEl('p', {}, {}, input),
            this.vjs.dom.createEl('p', {}, {}, [saveButton, cancelButton])
        ]);

        input.setSelectionRange(input.value.length, input.value.length);
        input.focus();
    }

    public async showDeleteModal(videoId: string, deleteVideo: (videoId: string) => void) {
        const videoName = toVideoName(videoId);

        const prompt = this.vjs.dom.createEl('p', {}, {}, ['Delete recording: ', this.vjs.dom.createEl('u', {}, {}, videoName), '?']);
        const buttons = this.vjs.dom.createEl('p', {}, {}, [
            this.vjs.dom.createEl('button', {
                onclick: (_: MouseEvent) => {
                    this.boundHideModal();
                    deleteVideo(videoId);
                }
            }, { class: 'btn' }, 'Delete'),
            this.vjs.dom.createEl('button', { onclick: this.boundHideModal }, { class: 'btn' }, 'Cancel'),
        ]);

        this.showModal([prompt, buttons]);
    }

    public getActiveVideoId(): string | null {
        return this.sidebar.querySelector<HTMLLIElement>('li.active')?.id ?? null;
    }

    public setActiveVideoId(videoId: string | null) {
        this.sidebar.querySelector<HTMLLIElement>('li.active')?.classList.remove('active');
        if (videoId !== null) {
            const videoLi = this.sidebar.querySelector<HTMLLIElement>(`[id='${videoId}']`);
            videoLi?.classList.add('active');
            return videoLi !== null;
        } else {
            return true;
        }
    }

    public setVideoDescription(left: ContentDescriptor, center: ContentDescriptor) {
        this.vjs.dom.insertContent(this.descriptionLeft, left);
        this.vjs.dom.insertContent(this.descriptionCenter, center);
    }

    public setVideoDescriptionMetadata(data: GameMetadata) {
        const summoner = this.vjs.dom.createEl(
            'span',
            {},
            { class: 'summoner-name' },
            data.player.gameName
        );
        const score1 = `${data.championName} - ${data.stats.kills}/${data.stats.deaths}/${data.stats.assists} `;
        const score2 = `${data.stats.totalMinionsKilled} CS | ${data.stats.visionScore} WS`;

        const gameMode = `Game Mode: ${data.queue.name} `;
        const result = data.stats.win ?
            this.vjs.dom.createEl('span', {}, { class: 'win' }, 'Victory')
            : this.vjs.dom.createEl('span', {}, { class: 'loss' }, 'Defeat');

        this.setVideoDescription(
            [
                summoner,
                this.vjs.dom.createEl('br'),
                score1,
                this.vjs.dom.createEl('br'),
                score2
            ],
            [
                gameMode,
                this.vjs.dom.createEl('br'),
                result
            ]
        );
    }

    public showBigPlayButton(show: boolean) {
        const bpb = document.querySelector<HTMLButtonElement>('.vjs-big-play-button');
        if (bpb !== null) {
            bpb.style.display = show ? 'block !important' : 'none !important';
        }
    }

    public setCheckboxes(settings: MarkerFlags) {
        this.checkboxKill.checked = settings.kill;
        this.checkboxDeath.checked = settings.death;
        this.checkboxAssist.checked = settings.assist;
        this.checkboxTurret.checked = settings.turret;
        this.checkboxInhibitor.checked = settings.inhibitor;
        this.checkboxDragon.checked = settings.dragon;
        this.checkboxHerald.checked = settings.herald;
        this.checkboxBaron.checked = settings.baron;
    }

    public getMarkerFlags(): MarkerFlags {
        return {
            kill: this.checkboxKill.checked,
            death: this.checkboxDeath.checked,
            assist: this.checkboxAssist.checked,
            turret: this.checkboxTurret.checked,
            inhibitor: this.checkboxInhibitor.checked,
            dragon: this.checkboxDragon.checked,
            herald: this.checkboxHerald.checked,
            baron: this.checkboxBaron.checked,
        };
    }

}
