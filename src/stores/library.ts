import { computed, reactive, shallowReactive } from "vue";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api, type Playlist, type Track } from "../api";
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
  playlists: Playlist[];
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

export const lib = shallowReactive<LibState>({
  tracks: [],
  folders: [],
  playlists: [],
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

/** 应用后端返回的曲库数据(曲目/文件夹/播放列表统一入口) */
export function applyLibraryData(data: { tracks: Track[]; folders: string[]; playlists?: Playlist[] }): void {
  lib.tracks = data.tracks;
  lib.folders = data.folders;
  if (data.playlists) lib.playlists = data.playlists;
}

let listenersInited = false;

/** 注册 Tauri 事件监听(仅一次;HMR/重复调用不会叠加) */
function initLibraryListeners(): void {
  if (listenersInited) return;
  listenersInited = true;
  void listen<{ current: number; total: number }>("scan-progress", (e) => {
    lib.scanCurrent = e.payload.current;
    lib.scanTotal = e.payload.total;
  });

  // 后端处理"打开方式"/命令行导入完成后刷新曲库,并对新增曲目自动在线匹配
  void listen("library-changed", async () => {
    try {
      const before = new Set(lib.tracks.map((t) => t.id));
      const data = await api.getLibrary();
      applyLibraryData(data);
      await enrichNewTracks(before, "已导入");
    } catch (err) {
      console.error("刷新曲库失败", err);
    }
  });

  // 命令行/双击"打开方式"导入的失败清单(后端聚合)
  void listen<string>("import-error", (e) => {
    flashStatus(`导入失败: ${e.payload}`);
  });

  // TMCL 大包的写入进度(后端按资源数上报,驱动导出弹窗里的细进度条)
  void listen<{ done: number; total: number }>("export-progress", (e) => {
    exportState.assetDone = e.payload.done;
    exportState.assetTotal = e.payload.total;
  });
}

export async function initLibrary(): Promise<void> {
  initLibraryListeners();
  try {
    const data = await api.getLibrary();
    applyLibraryData(data);
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
  const tmcl = paths.filter((p) => p.toLowerCase().endsWith(".tmcl"));
  const tmc = paths.filter((p) => p.toLowerCase().endsWith(".tmc"));
  const rest = paths.filter(
    (p) => !p.toLowerCase().endsWith(".tmc") && !p.toLowerCase().endsWith(".tmcl"),
  );
  const parts: string[] = [];
  if (tmcl.length) {
    let ok = 0;
    let fail = 0;
    const names: string[] = [];
    for (const p of tmcl) {
      try {
        names.push(await api.importPlaylistTmcl(p));
        ok++;
      } catch (err) {
        console.error("导入 TMCL 失败", err);
        fail++;
        parts.push(`播放列表导入失败: ${String(err)}`);
      }
    }
    if (ok) parts.push(`导入播放列表「${names.join("」「")}」${fail ? ` · ${fail} 个失败` : ""}`);
  }
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

/** 单首歌的导出描述(id + 展示名) */
interface ExportItem {
  id: string;
  name: string;
}

/** 两阶段导出的阶段:packing 打包中 → ready 包裹就绪 → placing 落位中 → done 完成 */
export type ExportPhase = "packing" | "ready" | "placing" | "done";

/** 导出进度弹窗的实时状态(与设置/关于同款弹窗,由 ExportModal 渲染) */
export const exportState = reactive({
  open: false,
  phase: "packing" as ExportPhase,
  /** 来源标签(如「整张专辑」/「播放列表 · xxx」) */
  label: "",
  /** 包裹总数与已打包完成数 */
  total: 0,
  done: 0,
  /** 当前正在打包的歌名 */
  current: "",
  /** 单个包裹内部进度(TMCL 大包由后端 export-progress 事件驱动) */
  assetDone: 0,
  assetTotal: 0,
  /** 打包失败被跳过的数量 */
  packFailed: 0,
  /** 已打包好的暂存文件绝对路径 */
  files: [] as string[],
  stageDir: "",
  /** 落位结果 */
  placed: 0,
  failed: 0,
  err: "",
});

/** 统一两阶段导出:先全部打包到 Temp 暂存区(弹窗显示进度),
 *  包裹就绪后由用户点「导出」选择位置,完整文件瞬间落位,目标路径不会出现半成品 */
async function runExport(
  label: string,
  total: number,
  pack: () => Promise<string[]>,
): Promise<void> {
  exportState.open = true;
  exportState.phase = "packing";
  exportState.label = label;
  exportState.total = total;
  exportState.done = 0;
  exportState.current = "";
  exportState.assetDone = 0;
  exportState.assetTotal = 0;
  exportState.packFailed = 0;
  exportState.files = [];
  exportState.stageDir = "";
  exportState.placed = 0;
  exportState.failed = 0;
  exportState.err = "";
  try {
    exportState.stageDir = await api.makeStageDir();
    exportState.files = await pack();
    // 弹窗已被用户取消:保持关闭,不进入就绪态
    if (!exportState.open) return;
    exportState.phase = "ready";
  } catch (err) {
    console.error("导出打包失败", err);
    exportState.err = err instanceof Error ? err.message : String(err);
  }
}

/** 关闭导出弹窗:尚未导出的包裹连同暂存目录一并清理 */
export function closeExport(): void {
  if (exportState.stageDir && exportState.phase !== "done") {
    void api.cleanupStage(exportState.stageDir);
  }
  exportState.open = false;
}

/** 弹窗内点「导出」:选目标文件夹,把暂存好的完整包裹瞬间落位 */
export async function confirmExport(): Promise<void> {
  if (exportState.phase !== "ready") return;
  const dir = await api.pickExportDir();
  if (!dir) return;
  exportState.phase = "placing";
  try {
    const placed = await api.placeStaged(exportState.files, dir);
    exportState.placed = placed.length;
    exportState.failed = exportState.files.length - placed.length;
  } catch (err) {
    console.error("导出落位失败", err);
    exportState.placed = 0;
    exportState.failed = exportState.files.length;
    exportState.err = err instanceof Error ? err.message : String(err);
  }
  void api.cleanupStage(exportState.stageDir);
  exportState.phase = "done";
  const prefix = exportState.label ? `${exportState.label} · ` : "";
  flashStatus(
    exportState.failed === 0
      ? `${prefix}已导出 ${exportState.placed} 个音乐包`
      : `${prefix}已导出 ${exportState.placed}/${exportState.files.length} 个(部分失败)`,
  );
}

/** 把一批歌逐个打包进暂存区(每首一个 .tmc;单个失败跳过不中断) */
function packTmcItems(items: ExportItem[]): Promise<string[]> {
  return (async () => {
    const files: string[] = [];
    for (const it of items) {
      // 弹窗被关闭:中止后续打包,并清掉在途打包可能重建的暂存目录
      if (!exportState.open) {
        void api.cleanupStage(exportState.stageDir);
        return files;
      }
      exportState.current = it.name;
      try {
        files.push(await api.stageTmc(it.id, exportState.stageDir));
      } catch (err) {
        console.error("打包 TMC 失败", it.name, err);
        exportState.packFailed++;
      }
      exportState.done++;
    }
    if (files.length === 0) throw new Error("所有包裹打包失败");
    return files;
  })();
}

/** 单首导出入口(列表行/右键):与其他导出一样走进度弹窗 */
export async function exportTrackTmc(id: string, title: string, artist: string): Promise<void> {
  await runExport("", 1, () => packTmcItems([{ id, name: `${title} - ${artist}` }]));
}

/** 批量导出多首歌为 .tmc 音乐包(多选操作条 / 专辑导出入口) */
export async function exportTracksTmc(
  ids: string[],
  tracks: Track[],
  label?: string,
): Promise<void> {
  if (ids.length === 0) return;
  const items = tracks
    .filter((t) => ids.includes(t.id))
    .map((t) => ({ id: t.id, name: `${t.title} - ${t.artist}` }));
  if (items.length === 0) return;
  const text = label || (items.length > 1 ? `已选 ${items.length} 首` : "");
  await runExport(text, items.length, () => packTmcItems(items));
}

/** 导出播放列表为 .tmcl:单个大包,写入进度由后端 export-progress 事件上报 */
export async function exportPlaylistTmcl(playlistId: string, name: string): Promise<void> {
  await runExport(`播放列表 · ${name}`, 1, async () => {
    exportState.current = name;
    const f = await api.stageTmcl(playlistId, exportState.stageDir);
    exportState.done = 1;
    return [f];
  });
}

/** 通过网易云公开接口为整张专辑在线补全封面与歌词(仅元数据,播放仍走本地文件) */
export async function enrichAlbumNetease(key: string): Promise<void> {
  flashStatus("正在在线匹配封面与歌词…");
  try {
    const r = await api.neteaseEnrichAlbum(key);
    const parts: string[] = [];
    if (r.cover) parts.push("封面已更新");
    if (r.lyrics) {
      parts.push(
        r.lyricsLrclib
          ? `补全 ${r.lyrics} 首歌词(LRCLIB 兜底 ${r.lyricsLrclib} 首)`
          : `补全 ${r.lyrics} 首歌词`,
      );
    }
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
    const r = await api.deleteTracks(ids);
    purgeDeleted(ids);
    ok = r.deleted > 0;
    if (r.failed > 0) {
      flashStatus(
        r.deleted > 0
          ? `已删除 ${r.deleted} 首,${r.failed} 首失败(文件被占用)`
          : `删除失败:${r.failed} 首文件被占用`,
      );
    } else {
      flashStatus(r.deleted > 0 ? `已删除 ${r.deleted} 首(移入回收站)` : "删除失败");
    }
  } catch (err) {
    console.error("删除失败", err);
    flashStatus("删除失败");
  }
  try {
    const data = await api.getLibrary();
    applyLibraryData(data);
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
    applyLibraryData(data);
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

/** 曲目所属专辑的唯一键(albumArtist + album) */
export const trackAlbumKey = keyOf;

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
