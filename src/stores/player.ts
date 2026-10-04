import { computed, shallowReactive } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Track } from "../api";
import { StorageKeys, storageGetString, storageSet } from "./storage";

/** 播放模式:互斥单选,一键循环切换 */
export type PlayMode = "seq" | "loop" | "one" | "shuffle";

export const MODE_ORDER: PlayMode[] = ["seq", "loop", "one", "shuffle"];
export const MODE_LABEL: Record<PlayMode, string> = {
  seq: "顺序播放",
  loop: "列表循环",
  one: "单曲循环",
  shuffle: "随机播放",
};

interface PlayerState {
  queue: Track[];
  index: number;
  playing: boolean;
  position: number;
  duration: number;
  volume: number;
  mode: PlayMode;
}

export const player = shallowReactive<PlayerState>({
  queue: [],
  index: -1,
  playing: false,
  position: 0,
  duration: 0,
  volume: Number(storageGetString(StorageKeys.volume, "1")),
  mode: loadSavedMode(),
});

function loadSavedMode(): PlayMode {
  const saved = storageGetString(StorageKeys.mode, "seq");
  return (MODE_ORDER as string[]).includes(saved) ? (saved as PlayMode) : "seq";
}

/** 切换播放模式并持久化(重启后保持) */
export function setMode(m: PlayMode): void {
  player.mode = m;
  storageSet(StorageKeys.mode, m);
}

export const current = computed<Track | null>(() => player.queue[player.index] ?? null);

/** 进度超过该秒数时按"上一曲"先回到开头 */
const PREV_RESTART_THRESHOLD = 3;

let audio: HTMLAudioElement | null = null;
let raf = 0;
/** 连续加载失败计数,成功播放后清零(防全坏队列无限跳曲) */
let errorStreak = 0;

/** 实际播放过的下标历史:prev() 据此回到"真正播过的那首"(随机模式下尤其重要) */
let playHistory: number[] = [];
/** 洗牌袋:当前队列下标的一个随机排列,播完一轮才重新洗牌(避免有放回抽样的重复感) */
let bag: number[] = [];
/** 袋子对应的队列引用:队列被替换(playTracks/删除清理)时自动重建 */
let bagQueue: Track[] | null = null;

function pushHistory(idx: number): void {
  if (idx < 0) return;
  playHistory.push(idx);
  if (playHistory.length > 200) playHistory.shift();
}

function rebuildBag(n: number): void {
  bag = Array.from({ length: n }, (_, i) => i);
  for (let i = bag.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [bag[i], bag[j]] = [bag[j], bag[i]];
  }
}

/** 从洗牌袋取下一个下标:整袋播完才重洗,且绝不与当前曲目立即重复 */
function takeShuffleIndex(n: number, current: number): number {
  if (bagQueue !== player.queue) {
    bag = [];
    bagQueue = player.queue;
  }
  for (;;) {
    if (bag.length === 0) rebuildBag(n);
    if (n > 1 && bag[bag.length - 1] === current) {
      // 袋顶恰是当前曲目:与非当前的随机位置交换;若袋里只剩它则重洗并剔除
      if (bag.length > 1) {
        const j = Math.floor(Math.random() * (bag.length - 1));
        [bag[bag.length - 1], bag[j]] = [bag[j], bag[bag.length - 1]];
      } else {
        rebuildBag(n);
        bag = bag.filter((x) => x !== current);
      }
      continue;
    }
    const idx = bag.pop();
    if (idx === undefined) return current; // n<=1 情形,调用方已拦截,兜底
    return idx;
  }
}

function ensureAudio(): HTMLAudioElement {
  if (audio) return audio;
  audio = new Audio();
  audio.volume = Math.min(1, Math.max(0, player.volume));

  const tick = (): void => {
    if (audio && !audio.paused) {
      // ~5Hz 更新足够 UI 使用,避免每帧 60 次触发全链路响应式更新
      const t = audio.currentTime;
      if (Math.abs(t - player.position) > 0.2) player.position = t;
      raf = requestAnimationFrame(tick);
    }
  };
  audio.addEventListener("play", () => {
    player.playing = true;
    errorStreak = 0;
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(tick);
  });
  audio.addEventListener("pause", () => {
    player.playing = false;
    cancelAnimationFrame(raf);
    if (audio) player.position = audio.currentTime;
  });
  audio.addEventListener("ended", () => onEnded());
  audio.addEventListener("loadedmetadata", () => {
    if (audio && isFinite(audio.duration)) player.duration = audio.duration;
  });
  audio.addEventListener("error", () => {
    console.error("播放失败:", audio?.src);
    player.playing = false;
    // 队列里还有别的歌时自动跳下一曲,连续失败达到队列长度则停止,避免死循环
    if (player.queue.length > 1 && errorStreak < player.queue.length) {
      errorStreak += 1;
      next(false);
    }
  });
  return audio;
}

function load(track: Track): void {
  const a = ensureAudio();
  player.position = 0;
  player.duration = track.duration || 0;
  a.src = convertFileSrc(track.path);
  void a.play().catch(() => {});
}

/** 删除曲目后同步清理播放队列:修正 index,正在播的被删则清空播放器释放句柄 */
export function purgeDeleted(ids: string[]): void {
  if (player.queue.length === 0) return;
  const deletedSet = new Set(ids);
  const removedCurrent = !!current.value && deletedSet.has(current.value.id);
  const kept: Track[] = [];
  for (const t of player.queue) {
    if (deletedSet.has(t.id)) continue;
    kept.push(t);
  }
  if (kept.length === player.queue.length) return; // 队列里没有被删的曲子
  if (removedCurrent || kept.length === 0) {
    // 正在播的被删:清空播放器
    const a = ensureAudio();
    a.pause();
    a.removeAttribute("src");
    a.load();
    player.queue = kept;
    player.index = -1;
    player.playing = false;
    player.position = 0;
    player.duration = 0;
    return;
  }
  // 非当前播放:重建队列并尽量落在原来的曲子上
  const before = current.value?.id ?? "";
  player.queue = kept;
  let idx = kept.findIndex((t) => t.id === before);
  if (idx < 0) idx = Math.max(0, Math.min(player.index, kept.length - 1));
  player.index = idx;
}

/** 以一组曲目作为队列播放,start 为起始下标。新队列重置播放历史与洗牌袋 */
export function playTracks(tracks: Track[], start = 0): void {
  if (tracks.length === 0) return;
  player.queue = tracks;
  player.index = Math.max(0, Math.min(start, tracks.length - 1));
  playHistory = [];
  bag = [];
  bagQueue = tracks;
  load(player.queue[player.index]);
}

/** 在上下文列表中播放某一首(上下文缺省时单曲播放) */
export function playTrack(track: Track, context?: Track[]): void {
  const list = context && context.length > 0 ? context : [track];
  const idx = list.findIndex((t) => t.id === track.id);
  playTracks(list, Math.max(0, idx));
}

export function toggle(): void {
  if (!current.value) return;
  const a = ensureAudio();
  if (a.paused) void a.play().catch(() => {});
  else a.pause();
}

function replay(): void {
  const a = ensureAudio();
  a.currentTime = 0;
  void a.play().catch(() => {});
}

function stop(): void {
  const a = ensureAudio();
  a.pause();
  a.currentTime = 0;
}

export function next(manual = true): void {
  const n = player.queue.length;
  if (n === 0) return;
  const mode = player.mode;

  if (mode === "one" && !manual) {
    replay();
    return;
  }
  if (n === 1) {
    if (manual || mode === "loop") replay();
    return;
  }

  let idx: number;
  if (mode === "shuffle") {
    idx = takeShuffleIndex(n, player.index);
  } else {
    idx = player.index + 1;
    if (idx >= n) {
      if (mode === "loop" || manual) idx = 0;
      else {
        stop();
        return;
      }
    }
  }
  pushHistory(player.index);
  player.index = idx;
  load(player.queue[idx]);
}

export function prev(): void {
  const n = player.queue.length;
  if (n === 0) return;
  const a = ensureAudio();
  if (a.currentTime > PREV_RESTART_THRESHOLD) {
    a.currentTime = 0;
    return;
  }
  // 优先回到实际播放过的上一首(随机模式下这是唯一正确的语义);
  // 历史为空时退化为队列顺序的上一首
  let idx = playHistory.pop();
  if (idx === undefined || idx < 0 || idx >= n) {
    idx = player.index - 1;
    if (idx < 0) idx = player.mode === "loop" ? n - 1 : 0;
  }
  player.index = idx;
  load(player.queue[idx]);
}

function onEnded(): void {
  if (player.mode === "one") {
    replay();
    return;
  }
  next(false);
}

export function seek(t: number): void {
  const a = ensureAudio();
  if (isFinite(t) && t >= 0 && (player.duration === 0 || t <= player.duration + 1)) {
    a.currentTime = t;
    player.position = t;
  }
}

let volSaveTimer = 0;

export function setVolume(v: number): void {
  const clamped = Math.min(1, Math.max(0, v));
  player.volume = clamped;
  // 拖动过程每帧都会调用,落盘做防抖
  window.clearTimeout(volSaveTimer);
  volSaveTimer = window.setTimeout(() => storageSet(StorageKeys.volume, String(clamped)), 300);
  if (audio) audio.volume = clamped;
}

/** 顺序播放 → 列表循环 → 单曲循环 → 随机播放 → 顺序播放(切换即持久化) */
export function cycleMode(): void {
  const i = MODE_ORDER.indexOf(player.mode);
  setMode(MODE_ORDER[(i + 1) % MODE_ORDER.length]);
}

export function fmtTime(s: number): string {
  if (!isFinite(s) || s < 0) s = 0;
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${String(sec).padStart(2, "0")}`;
}
