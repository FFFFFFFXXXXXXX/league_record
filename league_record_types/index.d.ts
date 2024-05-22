// This file has been generated by Specta. DO NOT EDIT.

export type Recording = { videoId: string; metadata: MetadataFile | null }

export type Deferred = { matchId: MatchId; ingameTimeRecStartOffset: number; favorite: boolean }

export type Stats = { kills: number; deaths: number; assists: number; largestMultiKill: number; neutralMinionsKilled: number; neutralMinionsKilledEnemyJungle: number; neutralMinionsKilledTeamJungle: number; totalMinionsKilled: number; visionScore: number; visionWardsBoughtInGame: number; wardsPlaced: number; wardsKilled: number; win: boolean }

export type GameMetadata = { matchId: MatchId; ingameTimeRecStartOffset: number; queue: Queue; player: Player; championName: string; stats: Stats; participantId: number; events: GameEvent[]; favorite: boolean }

export type Event = { ChampionKill: { victim_id: number; killer_id: number; assisting_participant_ids: number[]; position: Position } } | { BuildingKill: { team_id: Team; killer_id: number; building_type: BuildingType; assisting_participant_ids: number[] } } | { EliteMonsterKill: { killer_id: number; monster_type: MonsterType; assisting_participant_ids: number[] } }

export type RecorderSettings = { window: Window | null; input_resolution: Resolution | null; output_resolution: Resolution | null; framerate: Framerate | null; rate_control: RateControl | null; record_audio: AudioSource | null; output_path: string | null; encoder: Encoder | null }

export type Queue = { id: number; name: string; isRanked: boolean }

export type Framerate = [number, number]

export type Settings = { markerFlags: MarkerFlags; checkForUpdates: boolean; debugLog: boolean; recordingsFolder: string; filenameFormat: string; encodingQuality: number; outputResolution: StdResolution | null; framerate: Framerate; recordAudio: AudioSource; onlyRecordRanked: boolean; autostart: boolean; maxRecordingAgeDays: number | null; maxRecordingsSizeGb: number | null }

export type AudioSource = "NONE" | "APPLICATION" | "SYSTEM" | "ALL"

export type MatchId = { gameId: number; platformId: string }

export type AppEvent = { type: "RecordingsChanged"; payload: null } | { type: "MetadataChanged"; payload: string[] } | { type: "MarkerflagsChanged"; payload: null }

export type Encoder = "JIM_NVENC" | "FFMPEG_NVENC" | "JIM_AV1" | "AMD_AMF_H264" | "AMD_AMF_AV1" | "OBS_QSV11_H264" | "OBS_QSV11_AV1" | "OBS_X264"

export type RateControl = { CBR: number } | { VBR: number } | { CQP: number } | { CRF: number } | { ICQ: number }

export type BuildingType = { buildingType: "INHIBITOR_BUILDING"; lane_type: LaneType } | { buildingType: "TOWER_BUILDING"; lane_type: LaneType; tower_type: TowerType }

/**
 * most common resolutions for the aspect ratios 4:3, 5:4, 16:9, 16:10, 21:9, 43:18, 24:10, 32:9, 32:10
 */
export type StdResolution = "1024x768p" | "1600x1200p" | "1280x1024p" | "1280x720p" | "1366x768p" | "1600x900p" | "1920x1080p" | "2560x1440p" | "3840x2160p" | "5120x2880p" | "1280x800p" | "1440x900p" | "1680x1050p" | "1920x1200p" | "2240x1400p" | "2560x1600p" | "2560x1080p" | "5120x2160p" | "2580x1080p" | "3440x1440p" | "3840x1600p" | "3840x1080p" | "5120x1440p" | "3840x1200p"

export type TowerType = "OUTER_TURRET" | "INNER_TURRET" | "BASE_TURRET" | "NEXUS_TURRET"

export type Player = { gameName: string; tagLine: string; summonerId?: number | null }

export type Window = { name: string; class: string | null; process: string | null }

export type Team = "BLUE" | "RED"

export type NoData = { favorite: boolean }

export type GameEvent = { event: Event; timestamp: number }

export type DragonType = "FIRE_DRAGON" | "EARTH_DRAGON" | "WATER_DRAGON" | "AIR_DRAGON" | "HEXTECH_DRAGON" | "CHEMTECH_DRAGON" | "ELDER_DRAGON"

export type MetadataFile = { Metadata: GameMetadata } | { Deferred: Deferred } | { NoData: NoData }

export type MarkerFlags = { kill: boolean; death: boolean; assist: boolean; turret: boolean; inhibitor: boolean; dragon: boolean; herald: boolean; baron: boolean }

export type Resolution = { width: number; height: number }

export type Position = { x: number; y: number }

export type LaneType = "TOP_LANE" | "MID_LANE" | "BOT_LANE"

export type MonsterType = { monsterType: "HORDE" } | { monsterType: "RIFTHERALD" } | { monsterType: "BARON_NASHOR" } | { monsterType: "DRAGON"; monsterSubType: DragonType }

