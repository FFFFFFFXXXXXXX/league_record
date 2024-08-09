export function toVideoName(videoId: string): string {
    return videoId.substring(0, videoId.lastIndexOf('.'));
}

export function toVideoId(videoName: string): string {
    return videoName + '.mp4';
}

export function splitRight(string: string, separator: string): string {
    return string.substring(string.lastIndexOf(separator) + 1);
}

// return this error in 'default' switch branches to make the switch statement exhaustive
export class UnreachableError extends Error {
    constructor(val: never) {
        super(`unreachable case: ${JSON.stringify(val)}`)
    }
}
