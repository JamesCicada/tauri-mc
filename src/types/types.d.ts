export type McVersionType =
    | "release"
    | "snapshot"
    | "old_alpha"
    | "old_beta";

export interface McVersion {
    id: string;
    type: McVersionType;
    releaseTime: string;
    url: string;
}

export interface VersionManifest {
    latest: {
        release: string;
        snapshot: string;
    };
    versions: McVersion[];
}

export interface Instance {
    id: string;
    name: string;
    version: string;
    icon?: string;
    lastPlayed?: string;
    state: "ready" | "installing" | "running" | "error";
}