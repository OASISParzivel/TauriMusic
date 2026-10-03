import { computed, reactive } from "vue";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api, type Track } from "../api";
import { purgeDeleted } from "./player";

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
  /** 启动读库失败(区别于空库,给用户重试入口) */
  loadError: string | null;
  scanning: boolean;
  scanCurrent: number;
  scanTotal: number;
  /** 应用自带的导入目录绝对路径 */
  importDir: string;
  /** 最近一次拖拽导入的结果提示(自动消失) */
  importStatus: string;
  /** 全部在线匹配进行中 */
  matching: boolean;
}

export const lib = reactive<LibState>({
  tracks: [],
  folders: [],
  loaded: false,
  loadError: null,
  scanning: false,
  scanCurrent: 0,
  scanTotal: 0,
  importDir: "",
  importStatus: "",
  matching: false,
});

let importStatusTimer = 0;

export async function initLibrary(): Promise<void> {
  try {
    const data = await api.getLibrary();
    lib.tracks = data.tracks;
    lib.folders = data.folders;
    lib.loadError = null;
  } catch (err) {
    console.error("读取曲库失败", err);
    lib.loadError = String(err);
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

  // 后端处理"打开方式"/命令行导入完成后刷新曲库,并对新增曲目自动在线匹配
  void listen("library-changed", async () => {
    try {
      const before = new Set(lib.tracks.map((t) => t.id));
      const data = await api.getLibrary();
      lib.tracks = data.tracks;
      lib.folders = data.folders;
      await enrichNewTracks(before, "已导入");
    } catch (err) {
      console.error("刷新曲库失败", err);
    }
  });

  // 命令行/双击"打开方式"导入的失败清单(后端聚合)
  void listen<string>("import-error", (e) => {
    flashStatus(`导入失败: ${e.payload}`);
  });

  // 读库失败时提供重试;成功才走启动扫描
  if (lib.loadError) return;
  if (lib.folders.length > 0) {
    // 手动放进音乐目录的新文件没有经过导入入口,启动扫描后对新增曲目自动在线匹配;
    // before 为上次退出时的曲库,匹配失败的旧曲目不会重复请求
    const before = new Set(lib.tracks.map((t) => t.id));
    await rescan();
    await enrichNewTracks(before, "曲库已同步", true);
  }
}

/** 读库失败后的重试入口 */
export async function reloadLibrary(): Promise<void> {
  lib.loaded = false;
  lib.loadError = null;
  await initLibrary();
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

/** 状态条提示(6 秒自动消失);其他模块也需要报错反馈,故导出 */
export function flashStatus(text: string): void {
  lib.importStatus = text;
  window.clearTimeout(importStatusTimer);
  importStatusTimer = window.setTimeout(() => (lib.importStatus = ""), 6000);
}

export async function importDropped(paths: string[]): Promise<void> {
  const before = new Set(lib.tracks.map((t) => t.id));
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
      if (r.duplicates) parts.push(`跳过 ${r.duplicates} 个重复`);
      if (r.skipped) parts.push(`跳过 ${r.skipped} 个不支持`);
    } catch (err) {
      console.error("导入失败", err);
      parts.push("导入失败");
    }
  }
  flashStatus(parts.length ? parts.join(" · ") : "没有可导入的文件");
  await rescan();
  // 导入后自动在线补全新增曲目的封面与歌词(拖拽文件夹导入的同样生效)
  await enrichNewTracks(before, parts.length ? parts.join(" · ") : "已导入");
}

/** 从文件选择器导入 .tmc 音乐包(设置弹窗入口) */
export async function importTmcPick(): Promise<void> {
  const path = await api.pickTmcFile();
  if (!path) return;
  await importDropped([path]);
}

/** 逐张专辑在线补全,返回有更新的专辑数 */
async function enrichKeys(keys: Set<string>): Promise<number> {
  let enriched = 0;
  for (const key of keys) {
    try {
      const r = await api.neteaseEnrichAlbum(key);
      if (r.cover || r.lyrics > 0) enriched++;
    } catch (err) {
      console.error("在线匹配失败", err);
    }
  }
  return enriched;
}

/** 找出 before 之后新增曲目中缺封面/歌词的专辑 key */
function keysOfNewTracks(before: Set<string>): Set<string> {
  const keys = new Set<string>();
  for (const t of lib.tracks) {
    if (!before.has(t.id) && (!t.cover || !t.lrcPath)) {
      keys.add(keyOf(t));
    }
  }
  return keys;
}

/** 导入通用收尾:对新增曲目自动在线匹配封面与歌词,有更新则重扫并刷新提示;quiet 时无新增不提示 */
async function enrichNewTracks(before: Set<string>, doneMsg: string, quiet = false): Promise<void> {
  const keys = keysOfNewTracks(before);
  if (keys.size === 0) {
    if (!quiet) flashStatus(doneMsg);
    return;
  }
  lib.matching = true;
  try {
    flashStatus("正在自动匹配封面与歌词…");
    const enriched = await enrichKeys(keys);
    if (enriched > 0) await rescan();
    flashStatus(enriched > 0 ? `${doneMsg} · 已为 ${enriched} 张专辑补全封面/歌词` : doneMsg);
  } finally {
    lib.matching = false;
  }
}

/** 歌曲页:为全库缺封面/歌词的专辑在线匹配(网易云公开接口) */
export async function enrichAllNetease(): Promise<void> {
  if (lib.matching) return;
  const keys = new Set<string>();
  for (const t of lib.tracks) {
    if (!t.cover || !t.lrcPath) {
      keys.add(`${t.albumArtist || t.artist}\u{1}${t.album}`);
    }
  }
  if (keys.size === 0) {
    flashStatus("所有歌曲都已有封面与歌词");
    return;
  }
  lib.matching = true;
  flashStatus(`正在在线匹配 ${keys.size} 张专辑…`);
  try {
    const done = await enrichKeys(keys);
    if (done > 0) await rescan();
    flashStatus(done > 0 ? `已为 ${done} 张专辑补全封面/歌词` : "没有找到可匹配的内容");
  } finally {
    lib.matching = false;
  }
}

/** 直接导入音乐文件:多选 → 复制进导入目录 → 扫描 → 自动在线匹配封面与歌词 */
export async function importMusicPick(): Promise<void> {
  const paths = await api.pickAudioFiles();
  if (!paths || paths.length === 0) return;
  const before = new Set(lib.tracks.map((t) => t.id));
  flashStatus(`正在导入 ${paths.length} 个文件…`);
  try {
    const r = await api.importPaths(paths);
    if (!r.filesCopied) {
      flashStatus(r.duplicates ? `全部为重复文件,已跳过 ${r.duplicates} 个` : "没有新文件可导入");
      return;
    }
    await rescan();
    await enrichNewTracks(before, `已导入 ${r.filesCopied} 个文件`);
  } catch (err) {
    console.error("导入音乐文件失败", err);
    flashStatus("导入失败");
  }
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

/** 批量导出多首歌为 .tmc 音乐包(多选操作条 / 专辑导出入口) */
export async function exportTracksTmc(
  ids: string[],
  tracks: Track[],
  label?: string,
): Promise<void> {
  if (ids.length === 0) return;
  const dir = await api.pickExportDir();
  if (!dir) return;
  flashStatus(`正在导出 ${ids.length} 个音乐包…`);
  let ok = 0;
  for (const t of tracks) {
    if (!ids.includes(t.id)) continue;
    try {
      await api.exportTmc(t.id, `${dir}\\${t.title} - ${t.artist}`);
      ok++;
    } catch (err) {
      console.error("导出 TMC 失败", t.title, err);
    }
  }
  const prefix = label ? `${label} · ` : "";
  flashStatus(
    ok === ids.length
      ? `${prefix}已导出 ${ok} 个音乐包`
      : `${prefix}已导出 ${ok}/${ids.length} 个(部分失败)`,
  );
}

/** 通过网易云公开接口为整张专辑在线补全封面与歌词(仅元数据,播放仍走本地文件) */
export async function enrichAlbumNetease(key: string): Promise<void> {
  flashStatus("正在在线匹配封面与歌词…");
  try {
    const r = await api.neteaseEnrichAlbum(key);
    const parts: string[] = [];
    if (r.cover) parts.push("封面已更新");
    if (r.lyrics) parts.push(`补全 ${r.lyrics} 首歌词`);
    if (!parts.length) parts.push(r.skipped ? "没有找到可匹配的内容" : "元数据已是最新");
    flashStatus(parts.join(" · "));
  } catch (err) {
    console.error("在线匹配失败", err);
    flashStatus("在线匹配失败");
  }
  await rescan();
}

/** 删除曲目:音频与歌词移入回收站,曲库同步移除;删除当前播放曲时清空播放器。
 *  返回是否全部删除成功(失败时调用方应留在原页面并提示)。 */
export async function deleteTracks(ids: string[]): Promise<boolean> {
  if (ids.length === 0) return false;
  let ok = false;
  try {
    const n = await api.deleteTracks(ids);
    purgeDeleted(ids);
    ok = n > 0;
    flashStatus(n > 0 ? `已删除 ${n} 首(移入回收站)` : "删除失败:文件被占用");
  } catch (err) {
    console.error("删除失败", err);
    flashStatus("删除失败");
  }
  try {
    const data = await api.getLibrary();
    lib.tracks = data.tracks;
    lib.folders = data.folders;
  } catch (err) {
    console.error("刷新曲库失败", err);
  }
  return ok;
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
  const before = new Set(lib.tracks.map((t) => t.id));
  try {
    await api.addFolder(path);
  } catch (err) {
    console.error("添加文件夹失败", err);
    flashStatus(`添加文件夹失败: ${String(err)}`);
    return;
  }
  if (!lib.folders.includes(path)) lib.folders.push(path);
  await rescan();
  const added = lib.tracks.filter((t) => !before.has(t.id)).length;
  await enrichNewTracks(before, `已添加文件夹 · 新增 ${added} 首`);
}

export async function removeFolder(path: string): Promise<void> {
  try {
    await api.removeFolder(path);
  } catch (err) {
    console.error("移除文件夹失败", err);
    return;
  }
  lib.folders = lib.folders.filter((f) => f !== path);
  // 组件级前缀:不能字符串 starts_with,否则移除 F:\music 会误伤 F:\music2
  const prefix = path.endsWith("\\") ? path : `${path}\\`;
  lib.tracks = lib.tracks.filter((t) => t.path !== path && !t.path.startsWith(prefix));
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
