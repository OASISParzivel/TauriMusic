import { invoke } from "@tauri-apps/api/core";

export interface Track {
  id: string;
  title: string;
  artist: string;
  album: string;
  albumArtist: string;
  trackNo: number | null;
  discNo: number | null;
  year: number | null;
  genre: string | null;
  duration: number;
  path: string;
  cover: string | null;
  hasLyrics: boolean;
  lrcPath: string | null;
  addedAt: number;
  mtime: number;
  size: number;
}

export interface Library {
  version: number;
  folders: string[];
  tracks: Track[];
}

export interface ScanReport {
  added: number;
  updated: number;
  removed: number;
  total: number;
  errors: number;
}

export interface LyricLine {
  timeMs: number;
  text: string;
}

export interface LyricsPayload {
  synced: LyricLine[] | null;
  plain: string | null;
}

export interface ImportReport {
  foldersAdded: number;
  filesCopied: number;
  skipped: number;
  duplicates: number;
}

export interface NeteaseReport {
  cover: boolean;
  lyrics: number;
  skipped: number;
}

export interface AssocState {
  ext: string;
  label: string;
  registered: boolean;
}

export interface ResourceUsage {
  /** 主进程内存占用(MB) */
  memory_mb: number;
  /** CPU 占用(%),两次查询间的均值 */
  cpu: number;
}

export const api = {
  getLibrary: () => invoke<Library>("get_library"),
  pickFolder: () => invoke<string | null>("pick_music_folder"),
  addFolder: (path: string) => invoke<void>("add_folder", { path }),
  removeFolder: (path: string) => invoke<void>("remove_folder", { path }),
  scan: () => invoke<ScanReport>("scan_library"),
  getLyrics: (id: string) => invoke<LyricsPayload | null>("get_lyrics", { id }),
  getImportDir: () => invoke<string>("get_import_dir"),
  openImportDir: () => invoke<string>("open_import_dir"),
  importPaths: (paths: string[]) => invoke<ImportReport>("import_paths", { paths }),
  importTmc: (path: string) => invoke<string>("import_tmc", { path }),
  exportTmc: (id: string, dest: string) => invoke<string>("export_tmc", { id, dest }),
  pickTmcFile: () => invoke<string | null>("pick_tmc_file"),
  pickAudioFiles: () => invoke<string[] | null>("pick_audio_files"),
  pickTmcDest: (defaultName: string) => invoke<string | null>("pick_tmc_dest", { defaultName }),
  neteaseEnrichAlbum: (albumKey: string) => invoke<NeteaseReport>("netease_enrich_album", { albumKey }),
  getAssociations: () => invoke<AssocState[]>("get_associations"),
  setAssociation: (ext: string, enable: boolean) => invoke<void>("set_association", { ext, enable }),
  deleteTracks: (ids: string[]) => invoke<number>("delete_tracks", { ids }),
  getResourceUsage: () => invoke<ResourceUsage>("get_resource_usage"),
};
