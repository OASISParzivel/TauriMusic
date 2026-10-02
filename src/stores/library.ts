import { computed, reactive } from "vue";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
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
  /** 应用自带的导入目录绝对路径 */
  importDir: string;
  /** 最近一次拖拽导入的结果提示(自动消失) */
  importStatus: string;
}

export const lib = reactive<LibState>({
  tracks: [],
  folders: [],
  loaded: false,
  scanning: false,
  scanCurrent: 0,
  scanTotal: 0,
  importDir: "",
  importStatus: "",
});

let importStatusTimer = 0;

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

  try {
    lib.importDir = await api.getImportDir();
  } catch (err) {
    console.error("获取导入目录失败", err);
  }

  void listen<{ current: number; total: number }>("scan-progress", (e) => {
    lib.scanCurrent = e.payload.current;
    lib.scanTotal = e.payload.total;
  });

  if (lib.folders.length > 0) {
    void rescan();
  }
}

/** 监听文件拖进窗口:.tmc 音乐包走解包导入,文件夹登记扫描,散装音频/歌词复制进导入目录 */
export async function initDragImport(): Promise<void> {
  try {
    await getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === "drop" && e.payload.paths.length) {
        void importDropped(e.payload.paths);
      }
    });
  } catch (err) {
    console.error("拖拽导入初始化失败", err);
  }
}

function flashStatus(text: string): void {
  lib.importStatus = text;
  window.clearTimeout(importStatusTimer);
  importStatusTimer = window.setTimeout(() => (lib.importStatus = ""), 6000);
}

export async function importDropped(paths: string[]): Promise<void> {
  const tmc = paths.filter((p) => p.toLowerCase().endsWith(".tmc"));
  const rest = paths.filter((p) => !p.toLowerCase().endsWith(".tmc"));
  const parts: string[] = [];
  if (tmc.length) {
    let ok = 0;
    let fail = 0;
    for (const p of tmc) {
      try {
        await api.importTmc(p);
        ok++;
      } catch (err) {
        console.error("导入 TMC 失败", err);
        fail++;
      }
    }
    parts.push(ok ? `导入 ${ok} 个音乐包${fail ? ` · ${fail} 个失败` : ""}` : "音乐包导入失败");
  }
  if (rest.length) {
    try {
      const r = await api.importPaths(rest);
      if (r.filesCopied) parts.push(`导入 ${r.filesCopied} 个文件`);
      if (r.foldersAdded) parts.push(`添加 ${r.foldersAdded} 个文件夹`);
      if (r.skipped) parts.push(`跳过 ${r.skipped} 个不支持`);
    } catch (err) {
      console.error("导入失败", err);
      parts.push("导入失败");
    }
  }
  flashStatus(parts.length ? parts.join(" · ") : "没有可导入的文件");
  await rescan();
}

/** 从文件选择器导入 .tmc 音乐包(设置弹窗入口) */
export async function importTmcPick(): Promise<void> {
  const path = await api.pickTmcFile();
  if (!path) return;
  await importDropped([path]);
}

/** 把一首歌导出为 .tmc 音乐包(歌曲列表行内入口) */
export async function exportTrackTmc(id: string, title: string, artist: string): Promise<void> {
  const dest = await api.pickTmcDest(`${title} - ${artist}`);
  if (!dest) return;
  try {
    const real = await api.exportTmc(id, dest);
    const name = real.split(/[\\/]/).pop() ?? real;
    flashStatus(`已导出 ${name}`);
  } catch (err) {
    console.error("导出 TMC 失败", err);
    flashStatus("导出失败");
  }
}

export async function openImportDir(): Promise<void> {
  try {
    lib.importDir = await api.openImportDir();
  } catch (err) {
    console.error("打开导入目录失败", err);
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
