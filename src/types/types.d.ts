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
    java_path?: string;
    java_path_override?: string;
    max_memory?: number;
    min_memory?: number;
    java_args?: string;
    java_warning_ignored: boolean;
    loader?: string;
    loader_version?: string;
    mc_version?: string;
}

export interface Settings {
    max_memory: number;
    min_memory: number;
    close_on_launch: boolean;
    keep_logs_open: boolean;
    global_java_args: string;
    global_java_path?: string;
    skip_java_check: boolean;
}

/* Modrinth Types */
export interface ModrinthSearchResult {
    hits: ModrinthProjectHit[];
    total_hits: number;
}

export interface ModrinthProjectHit {
    project_id: string;
    title: string;
    description: string;
    icon_url?: string;
    author: string;
    categories: string[];
    project_type: "mod" | "modpack" | "resourcepack" | "shader";
    latest_version: string;
}

export interface ModrinthVersion {
    id: string;
    project_id: string;
    name: string;
    version_number: string;
    game_versions: string[];
    loaders: string[];
    files: ModrinthFile[];
}

export interface LoaderCandidate {
    project_id: string;
    project_title: string;
    version_id: string;
    version_number: string;
    game_versions: string[];
}

export interface ModrinthFile {
    url: string;
    filename: string;
    primary: boolean;
    size: number;
}
