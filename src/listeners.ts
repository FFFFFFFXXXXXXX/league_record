import type { AppEvent } from "@fffffffxxxxxxx/league_record_types";
import { listen, TauriEvent, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";
import { appWindow } from '@tauri-apps/api/window';

export default class ListenerManager {
    private readonly unlistenFns: UnlistenFn[];

    constructor() {
        this.unlistenFns = [];
        this.listen_tauri(TauriEvent.WINDOW_CLOSE_REQUESTED, () => {
            this.unlistenFns.forEach(unlisten => unlisten());
            void appWindow.close();
        });
    }

    public listen_app = <T extends AppEvent["type"]>(event: T, callback: EventCallback<Extract<AppEvent, { type: T }>["payload"]>) => {
        listen(event, callback).then(fn => this.unlistenFns.push(fn));
    }

    public listen_tauri = <T>(event: TauriEvent, callback: EventCallback<T>) => {
        listen<T>(event, callback).then(fn => this.unlistenFns.push(fn));
    }
}
