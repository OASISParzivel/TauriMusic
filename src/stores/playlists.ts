import { computed, reactive } from "vue";
import { api, type Playlist, type PlaylistEntry, type Track } from "../api";
import { lib, flashStatus } from "./library";
import { ui } from "./ui";

/** 路径规范化:统一分隔符与大小写(mac/导入包的 '/' 与 Windows 的 '\' 混用) */
const normPath = (p: string): string => p.replace(/\//g, "\\").toLowerCase();

/** 曲库索引:按 id / 路径双索引,播放列表条目据此解析为当前有效的曲目 */
export const trackIndex = computed(() => {
  const byId = new Map<string, Track>();
  const byPath = new Map<string, Track>();
  for (const t of lib.tracks) {
    byId.set(t.id, t);
    byPath.set(normPath(t.path), t);
  }
  return { byId, byPath };
});

export interface ResolvedEntry {
  entry: PlaylistEntry;
  /** 解析到的最新曲目数据;null = 文件缺失 */
  track: Track | null;
  /** 在播放列表中的原始下标 */
  index: number;
}

/** 播放列表条目 → 当前曲库曲目(缺文件的条目保留展示但不可播放) */
export function resolvePlaylist(p: Playlist): ResolvedEntry[] {
  const { byId, byPath } = trackIndex.value;
  return p.entries.map((entry, index) => ({
    entry,
    index,
    track: byId.get(entry.id) ?? byPath.get(normPath(entry.path)) ?? null,
  }));
}

export function playlistById(id: string | null): Playlist | undefined {
  if (!id) return undefined;
  return lib.playlists.find((p) => p.id === id);
}

function setPlaylists(list: Playlist[]): void {
  lib.playlists = list;
}

// ===== 加入歌单选择器(歌曲行右键 / 多选操作条 / 专辑卡片共用) =====

export const picker = reactive<{ trackIds: string[] | null }>({ trackIds: null });

export function openPlaylistPicker(trackIds: string[]): void {
  if (trackIds.length === 0) return;
  picker.trackIds = trackIds;
}

export function closePlaylistPicker(): void {
  picker.trackIds = null;
}

export async function addToPlaylist(playlistId: string): Promise<void> {
  const ids = picker.trackIds ?? [];
  if (ids.length === 0) return;
  const name = playlistById(playlistId)?.name ?? "播放列表";
  try {
    const r = await api.addTracksToPlaylist(playlistId, ids);
    setPlaylists(r.playlists);
    const parts = [`已添加 ${r.added} 首到「${name}」`];
    if (r.skipped) parts.push(`跳过 ${r.skipped} 首已在列表中的`);
    flashStatus(parts.join(" · "));
  } catch (err) {
    console.error("加入播放列表失败", err);
    flashStatus(`加入失败: ${String(err)}`);
  }
  closePlaylistPicker();
}

/** 从选择器里新建并直接加入 */
export async function createAndAdd(name: string): Promise<void> {
  const trimmed = name.trim();
  if (!trimmed) return;
  try {
    const lists = await api.createPlaylist(trimmed);
    setPlaylists(lists);
    const created = lists.find((p) => p.name === trimmed);
    if (created) {
      const ids = picker.trackIds ?? [];
      if (ids.length) {
        const r = await api.addTracksToPlaylist(created.id, ids);
        setPlaylists(r.playlists);
        flashStatus(`已新建「${created.name}」并添加 ${r.added} 首`);
      } else {
        flashStatus(`已新建播放列表「${created.name}」`);
      }
    }
  } catch (err) {
    console.error("新建播放列表失败", err);
    flashStatus(`新建失败: ${String(err)}`);
  }
  closePlaylistPicker();
}

// ===== 名称输入弹窗(新建 / 重命名共用) =====

export const nameDialog = reactive({
  open: false,
  title: "新建播放列表",
  initial: "",
  /** rename 时为目标 id */
  targetId: "",
});

export function askCreatePlaylist(): void {
  nameDialog.open = true;
  nameDialog.title = "新建播放列表";
  nameDialog.initial = "";
  nameDialog.targetId = "";
}

export function askRenamePlaylist(p: Playlist): void {
  nameDialog.open = true;
  nameDialog.title = "重命名播放列表";
  nameDialog.initial = p.name;
  nameDialog.targetId = p.id;
}

export async function submitPlaylistName(name: string): Promise<void> {
  const trimmed = name.trim();
  if (!trimmed) return;
  nameDialog.open = false;
  try {
    if (nameDialog.targetId) {
      setPlaylists(await api.renamePlaylist(nameDialog.targetId, trimmed));
      flashStatus(`已重命名为「${trimmed}」`);
    } else {
      setPlaylists(await api.createPlaylist(trimmed));
      flashStatus(`已新建播放列表「${trimmed}」`);
    }
  } catch (err) {
    console.error("保存播放列表名称失败", err);
    flashStatus(String(err));
  }
}

// ===== 详情页操作 =====

export async function removeEntry(playlistId: string, index: number): Promise<void> {
  try {
    setPlaylists(await api.removePlaylistEntry(playlistId, index));
  } catch (err) {
    console.error("移除曲目失败", err);
    flashStatus("移除失败");
  }
}

export async function moveEntry(playlistId: string, from: number, to: number): Promise<void> {
  try {
    setPlaylists(await api.movePlaylistEntry(playlistId, from, to));
  } catch (err) {
    console.error("调整顺序失败", err);
    flashStatus("调整顺序失败");
  }
}

export async function deletePlaylist(p: Playlist): Promise<void> {
  try {
    setPlaylists(await api.deletePlaylist(p.id));
    flashStatus(`已删除播放列表「${p.name}」`);
  } catch (err) {
    console.error("删除播放列表失败", err);
    flashStatus("删除失败");
    return;
  }
  if (ui.view === "playlist" && ui.playlistId === p.id) ui.view = "playlists";
}

/** 导出为 .tmcl(7z:音乐源文件 + 歌词 + 封面 + playlist.json) */
export async function exportPlaylist(p: Playlist): Promise<void> {
  const dest = await api.pickTmclDest(p.name);
  if (!dest) return;
  flashStatus(`正在导出「${p.name}」…`);
  try {
    const real = await api.exportPlaylistTmcl(p.id, dest);
    const file = real.split(/[\\/]/).pop() ?? real;
    flashStatus(`已导出 ${file}`);
  } catch (err) {
    console.error("导出 TMCL 失败", err);
    flashStatus(`导出失败: ${String(err)}`);
  }
}
