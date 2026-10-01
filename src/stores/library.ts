import { computed, reactive } from "vue";
import { listen } from "@tauri-apps/api/event";
import { api, type Track } from "../api";

export interface Album {
  /** albumArtist + album 组成的唯一键 */
  key: string;
  album: string;
  artist: string;
  year: number | null;
  cover: string | null;
  tracks: Track[];
}

interface LibState {
  tracks: Track[];
  folders: string[];
  loaded: boolean;
  scanning: boolean;
  scanCurrent: number;
  scanTotal: number;
}

export const lib = reactive<LibState>({
  tracks: [],
  folders: [],
  loaded: false,
  scanning: false,
  scanCurrent: 0,
  scanTotal: 0,
});

export async function initLibrary(): Promise<void> {
  try {
    const data = await api.getLibrary();
    lib.tracks = data.tracks;
    lib.folders = data.folders;
  } catch (err) {
    console.error("读取曲库失败", err);
  } finally {
    lib.loaded = true;
  }

  void listen<{ current: number; total: number }>("scan-progress", (e) => {
    lib.scanCurrent = e.payload.current;
    lib.scanTotal = e.payload.total;
  });

  if (lib.folders.length > 0) {
    void rescan();
  }
}

export async function rescan(): Promise<void> {
  if (lib.scanning) return;
  lib.scanning = true;
  lib.scanCurrent = 0;
  lib.scanTotal = 0;
  try {
    await api.scan();
    const data = await api.getLibrary();
    lib.tracks = data.tracks;
    lib.folders = data.folders;
  } catch (err) {
    console.error("扫描失败", err);
  } finally {
    lib.scanning = false;
  }
}

export async function addFolder(): Promise<void> {
  const path = await api.pickFolder();
  if (!path) return;
  await api.addFolder(path);
  if (!lib.folders.includes(path)) lib.folders.push(path);
  await rescan();
}

export async function removeFolder(path: string): Promise<void> {
  try {
    await api.removeFolder(path);
  } catch (err) {
    console.error("移除文件夹失败", err);
    return;
  }
  lib.folders = lib.folders.filter((f) => f !== path);
  lib.tracks = lib.tracks.filter((t) => !t.path.startsWith(path));
}

const keyOf = (t: Track) => `${t.albumArtist || t.artist}\u{1}${t.album}`;

export const albums = computed<Album[]>(() => {
  const map = new Map<string, Album>();
  for (const t of lib.tracks) {
    const key = keyOf(t);
    let a = map.get(key);
    if (!a) {
      a = { key, album: t.album, artist: t.albumArtist || t.artist, year: t.year, cover: t.cover, tracks: [] };
      map.set(key, a);
    }
    a.tracks.push(t);
    if (!a.cover && t.cover) a.cover = t.cover;
    if (a.year == null && t.year != null) a.year = t.year;
  }
  for (const a of map.values()) {
    a.tracks.sort(
      (x, y) =>
        (x.discNo ?? 1) - (y.discNo ?? 1) ||
        (x.trackNo ?? 0) - (y.trackNo ?? 0) ||
        x.title.localeCompare(y.title, "zh"),
    );
  }
  return [...map.values()];
});

export const artists = computed<string[]>(() =>
  [...new Set(lib.tracks.map((t) => t.artist || "未知艺人"))].sort((a, b) =>
    a.localeCompare(b, "zh"),
  ),
);

export const allSongs = computed<Track[]>(() =>
  [...lib.tracks].sort((a, b) => a.title.localeCompare(b.title, "zh")),
);

export function albumByKey(key: string): Album | undefined {
  return albums.value.find((a) => a.key === key);
}

/** 专辑“最近添加”排序依据:取专辑内最新的入库/修改时间 */
export function albumRecency(a: Album): number {
  let m = 0;
  for (const t of a.tracks) m = Math.max(m, t.addedAt, t.mtime);
  return m;
}
