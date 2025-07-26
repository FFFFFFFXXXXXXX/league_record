import type { MetadataFile } from "./bindings";

export function toVideoName(videoId: string): string {
    return videoId.slice(0, videoId.lastIndexOf("."));
}

export function toVideoId(videoName: string): string {
    return videoName + ".mp4";
}

export function splitRight(string: string, separator: string): string {
    return string.slice(string.lastIndexOf(separator) + 1);
}

export function isFavorite(metadataFile: MetadataFile | null): boolean {
    if (!metadataFile) return false;
    if ("Metadata" in metadataFile) return metadataFile.Metadata.favorite;
    if ("Deferred" in metadataFile) return metadataFile.Deferred.favorite;
    return false;
}

// return this error in 'default' switch branches to make the switch statement exhaustive
export class UnreachableError extends Error {
    constructor(val: never) {
        super(`unreachable case: ${JSON.stringify(val)}`);
    }
}
