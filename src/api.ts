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

/** 播放列表条目:曲库引用 + 元数据快照 */
export interface PlaylistEntry {
  id: string;
  path: string;
  title: string;
  artist: string;
  album: string;
  albumArtist: string;
  duration: number;
}

export interface Playlist {
  id: string;
  name: string;
  createdAt: number;
  entries: PlaylistEntry[];
}

export interface AddToPlaylistReport {
  added: number;
  skipped: number;
  playlists: Playlist[];
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
  pickTmcFile: () => invoke<string | null>("pick_tmc_file"),
  pickAudioFiles: () => invoke<string[] | null>("pick_audio_files"),
  pickExportDir: () => invoke<string | null>("pick_export_dir"),
  // 两阶段导出:先打包到 Temp 暂存区(弹窗显示进度),就绪后用户点「导出」再落位完整文件
  makeStageDir: () => invoke<string>("make_stage_dir"),
  stageTmc: (id: string, staging: string) => invoke<string>("stage_tmc", { id, staging }),
  stageTmcl: (playlistId: string, staging: string) =>
    invoke<string>("stage_tmcl", { playlistId, staging }),
  placeStaged: (files: string[], destDir: string) =>
    invoke<string[]>("place_staged", { files, destDir }),
  cleanupStage: (dir: string) => invoke<void>("cleanup_stage", { dir }),
  neteaseEnrichAlbum: (albumKey: string) => invoke<NeteaseReport>("netease_enrich_album", { albumKey }),
  getAssociations: () => invoke<AssocState[]>("get_associations"),
  setAssociation: (ext: string, enable: boolean) => invoke<void>("set_association", { ext, enable }),
  deleteTracks: (ids: string[]) =>
    invoke<{ deleted: number; failed: number }>("delete_tracks", { ids }),
  createPlaylist: (name: string) => invoke<Playlist[]>("create_playlist", { name }),
  renamePlaylist: (id: string, name: string) => invoke<Playlist[]>("rename_playlist", { id, name }),
  deletePlaylist: (id: string) => invoke<Playlist[]>("delete_playlist", { id }),
  addTracksToPlaylist: (playlistId: string, trackIds: string[]) =>
    invoke<AddToPlaylistReport>("add_tracks_to_playlist", { playlistId, trackIds }),
  removePlaylistEntry: (playlistId: string, index: number) =>
    invoke<Playlist[]>("remove_playlist_entry", { playlistId, index }),
  movePlaylistEntry: (playlistId: string, from: number, to: number) =>
    invoke<Playlist[]>("move_playlist_entry", { playlistId, from, to }),
  importPlaylistTmcl: (path: string) => invoke<string>("import_playlist_tmcl", { path }),
  getResourceUsage: () => invoke<ResourceUsage>("get_resource_usage"),
};
