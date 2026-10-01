import { computed, reactive } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Track } from "../api";

export type RepeatMode = "off" | "all" | "one";

interface PlayerState {
  queue: Track[];
  index: number;
  playing: boolean;
  position: number;
  duration: number;
  volume: number;
  shuffle: boolean;
  repeat: RepeatMode;
}

export const player = reactive<PlayerState>({
  queue: [],
  index: -1,
  playing: false,
  position: 0,
  duration: 0,
  volume: Number(localStorage.getItem("tm-vol") ?? "1"),
  shuffle: false,
  repeat: "off",
});

export const current = computed<Track | null>(() => player.queue[player.index] ?? null);

let audio: HTMLAudioElement | null = null;
let raf = 0;

function ensureAudio(): HTMLAudioElement {
  if (audio) return audio;
  audio = new Audio();
  audio.volume = Math.min(1, Math.max(0, player.volume));

  const tick = (): void => {
    if (audio && !audio.paused) {
      player.position = audio.currentTime;
      raf = requestAnimationFrame(tick);
    }
  };
  audio.addEventListener("play", () => {
    player.playing = true;
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

/** 以一组曲目作为队列播放,start 为起始下标 */
export function playTracks(tracks: Track[], start = 0): void {
  if (tracks.length === 0) return;
  player.queue = tracks;
  player.index = Math.max(0, Math.min(start, tracks.length - 1));
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

export function next(manual = true): void {
  const n = player.queue.length;
  if (n === 0) return;

  if (player.repeat === "one" && !manual) {
    replay();
    return;
  }
  if (n === 1) {
    if (manual) replay();
    return;
  }

  let idx: number;
  if (player.shuffle) {
    do {
      idx = Math.floor(Math.random() * n);
    } while (idx === player.index);
  } else {
    idx = player.index + 1;
    if (idx >= n) {
      if (player.repeat === "all" || manual) idx = 0;
      else {
        const a = ensureAudio();
        a.pause();
        a.currentTime = 0;
        return;
      }
    }
  }
  player.index = idx;
  load(player.queue[idx]);
}

export function prev(): void {
  const n = player.queue.length;
  if (n === 0) return;
  const a = ensureAudio();
  if (a.currentTime > 3) {
    a.currentTime = 0;
    return;
  }
  let idx = player.index - 1;
  if (idx < 0) idx = player.repeat === "all" ? n - 1 : 0;
  player.index = idx;
  load(player.queue[idx]);
}

function onEnded(): void {
  if (player.repeat === "one") {
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

export function setVolume(v: number): void {
  const clamped = Math.min(1, Math.max(0, v));
  player.volume = clamped;
  localStorage.setItem("tm-vol", String(clamped));
  if (audio) audio.volume = clamped;
}

export function cycleRepeat(): void {
  player.repeat = player.repeat === "off" ? "all" : player.repeat === "all" ? "one" : "off";
}

export function fmtTime(s: number): string {
  if (!isFinite(s) || s < 0) s = 0;
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${String(sec).padStart(2, "0")}`;
}
