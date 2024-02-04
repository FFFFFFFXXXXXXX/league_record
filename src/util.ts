export function sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
}

export function toVideoName(videoId: string): string {
    return videoId.substring(0, videoId.lastIndexOf('.'));
}

export function toVideoId(videoName: string): string {
    return videoName + '.mp4';
}

export function splitRight(string: string, separator: string): string {
    return string.substring(string.lastIndexOf(separator) + 1);
}
