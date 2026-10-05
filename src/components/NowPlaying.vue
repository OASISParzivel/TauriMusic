<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api, type LyricLine } from "../api";
import { player, current, toggle, next, prev, seek, fmtTime } from "../stores/player";
import ModeIcon from "./ModeIcon.vue";
import { ui } from "../stores/ui";

const coverUrl = computed(() => {
  // 全屏播放页是大图消费方,优先 512px 档;旧记录没有大图时退回小图
  const src = current.value?.coverLarge ?? current.value?.cover;
  return src ? convertFileSrc(src) : null;
});
const bgStyle = computed(() => ({
  "--np-cover": coverUrl.value ? `url("${coverUrl.value}")` : "none",
}));
const progress = computed(() => (player.duration > 0 ? (player.position / player.duration) * 100 : 0));

const synced = ref<LyricLine[] | null>(null);
const plain = ref<string | null>(null);
const activeIdx = ref(-1);
const lyricsEl = ref<HTMLElement | null>(null);
const noLyrics = ref(false);
const loadingLyrics = ref(false);

watch(current, loadLyrics, { immediate: true });

async function loadLyrics(): Promise<void> {
  synced.value = null;
  plain.value = null;
  activeIdx.value = -1;
  noLyrics.value = false;
  loadingLyrics.value = true;
  const track = current.value;
  if (!track) {
    loadingLyrics.value = false;
    return;
  }
  try {
    const payload = await api.getLyrics(track.id);
    if (current.value?.id !== track.id) return;
    if (!payload) {
      noLyrics.value = true;
      return;
    }
    synced.value = payload.synced;
    plain.value = payload.plain;
  } catch (err) {
    console.error("歌词加载失败", err);
    noLyrics.value = true;
  } finally {
    loadingLyrics.value = false;
  }
}

watch(
  () => player.position,
  (pos) => {
    const lines = synced.value;
    if (!lines || lines.length === 0) return;
    const t = pos * 1000 + 200;
    // 二分查找当前行(线性扫描在长歌词下每秒多次全量遍历)
    let lo = 0;
    let hi = lines.length - 1;
    let idx = -1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (lines[mid].timeMs <= t) {
        idx = mid;
        lo = mid + 1;
      } else {
        hi = mid - 1;
      }
    }
    if (idx !== activeIdx.value) {
      activeIdx.value = idx;
      if (idx >= 0) scrollToLine(idx);
    }
  },
);

function scrollToLine(idx: number): void {
  const el = lyricsEl.value;
  if (!el) return;
  const child = el.children[idx] as HTMLElement | undefined;
  if (!child) return;
  el.scrollTo({
    top: child.offsetTop - el.clientHeight / 2 + child.clientHeight / 2,
    behavior: "smooth",
  });
}

function close(): void {
  ui.nowPlayingOpen = false;
}

function onSeek(e: Event): void {
  seek(Number((e.target as HTMLInputElement).value));
}
</script>

<template>
  <div class="np" :style="bgStyle">
    <button class="close" title="关闭" @click="close">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="m6 9.2 6 6 6-6" />
      </svg>
    </button>

    <div class="stage">
      <div class="art">
        <img v-if="coverUrl" :src="coverUrl" alt="" />
        <div v-else class="ph">♪</div>
      </div>

      <div class="info">
        <div class="title">{{ current?.title }}</div>
        <div class="artist">{{ current?.artist }}{{ current?.album ? " — " + current.album : "" }}</div>
      </div>

      <div class="lyrics-wrap">
        <div v-if="synced && synced.length" ref="lyricsEl" class="lyrics synced">
          <p
            v-for="(line, i) in synced"
            :key="i"
            :class="{ active: i === activeIdx, near: Math.abs(i - activeIdx) === 1, far: Math.abs(i - activeIdx) > 1 }"
            @click="seek(line.timeMs / 1000)"
          >
            {{ line.text || "♪" }}
          </p>
        </div>
        <div v-else-if="plain" class="lyrics plain">
          <p v-for="(line, i) in plain.split('\n')" :key="i">{{ line }}</p>
        </div>
        <div v-else-if="loadingLyrics" class="lyrics none">歌词加载中…</div>
        <div v-else class="lyrics none">暂无歌词</div>
      </div>
    </div>

    <div class="bottom">
      <div class="controls">
        <ModeIcon class="t-btn" />
        <button class="t-btn" title="上一曲" @click="prev()">
          <svg viewBox="0 0 24 24">
            <path d="M7 5.8v12.4" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" />
            <path d="M18.5 6.3v11.4L9.4 12z" fill="currentColor" />
          </svg>
        </button>
        <button class="t-btn play" :title="player.playing ? '暂停' : '播放'" @click="toggle()">
          <svg v-if="!player.playing" viewBox="0 0 24 24">
            <path d="M8.2 5.5v13L19 12z" fill="currentColor" />
          </svg>
          <svg v-else viewBox="0 0 24 24">
            <path d="M7.6 5.5h3.3v13H7.6zM13.2 5.5h3.3v13h-3.3z" fill="currentColor" />
          </svg>
        </button>
        <button class="t-btn" title="下一曲" @click="next()">
          <svg viewBox="0 0 24 24">
            <path d="M17 5.8v12.4" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" />
            <path d="M5.5 6.3v11.4L14.6 12z" fill="currentColor" />
          </svg>
        </button>
      </div>
      <div class="seek">
        <span class="time">{{ fmtTime(player.position) }}</span>
        <input
          class="range"
          type="range"
          min="0"
          :max="player.duration || 1"
          step="0.1"
          :value="player.position"
          :style="{ '--fill': progress + '%' }"
          @input="onSeek"
        />
        <span class="time">{{ fmtTime(player.duration) }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.np {
  position: absolute;
  inset: 0;
  z-index: 60;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: #fff;
  background: #242426;
}
.np::before {
  content: "";
  position: absolute;
  inset: -90px;
  background-image: var(--np-cover);
  background-size: cover;
  background-position: center;
  filter: blur(80px) saturate(1.5) brightness(0.5);
  transform: scale(1.1);
}

.close {
  position: absolute;
  top: 60px;
  right: 18px;
  z-index: 5;
  width: 34px;
  height: 34px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.85);
  background: rgba(255, 255, 255, 0.14);
  backdrop-filter: blur(10px);
  transition: background 0.25s var(--ease-out-soft), transform 0.35s var(--ease-spring);
}
.close:hover {
  background: rgba(255, 255, 255, 0.26);
}
.close svg {
  width: 18px;
  height: 18px;
}

.stage {
  position: relative;
  flex: 1;
  display: grid;
  grid-template-columns: minmax(260px, 380px) 1fr;
  gap: 40px;
  align-items: center;
  padding: 48px 56px 12px;
  min-height: 0;
}

.art {
  position: relative;
  aspect-ratio: 1;
  max-height: 100%;
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 18px 60px rgba(0, 0, 0, 0.55);
  background: rgba(255, 255, 255, 0.08);
}
.art img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.art .ph {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 64px;
  color: rgba(255, 255, 255, 0.5);
}

.info {
  position: absolute;
  top: 26px;
  left: 56px;
  max-width: 420px;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.4);
}
.info .title {
  font-size: 22px;
  font-weight: 700;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.info .artist {
  font-size: 14px;
  opacity: 0.82;
  margin-top: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lyrics-wrap {
  height: 100%;
  min-height: 0;
  display: flex;
  align-items: stretch;
  mask-image: linear-gradient(to bottom, transparent, #000 12%, #000 86%, transparent);
}
.lyrics {
  flex: 1;
  overflow-y: auto;
  padding: 42% 8px 42%;
  scrollbar-width: none;
}
.lyrics::-webkit-scrollbar {
  display: none;
}
.lyrics.synced p {
  font-size: 22px;
  font-weight: 700;
  line-height: 1.45;
  padding: 7px 0;
  color: rgba(255, 255, 255, 0.42);
  cursor: pointer;
  transition: color 0.4s var(--ease-out-soft), transform 0.4s var(--ease-out-soft);
  transform-origin: left center;
}
.lyrics.synced p.near {
  color: rgba(255, 255, 255, 0.62);
}
.lyrics.synced p.active {
  color: #fff;
  transform: scale(1.02);
}
.lyrics.synced p:hover {
  color: rgba(255, 255, 255, 0.9);
}
.lyrics.plain p {
  font-size: 16px;
  line-height: 1.9;
  color: rgba(255, 255, 255, 0.8);
  white-space: pre-wrap;
}
.lyrics.none {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
  color: rgba(255, 255, 255, 0.55);
}

.bottom {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 10px 0 26px;
}
.controls {
  display: flex;
  align-items: center;
  gap: 26px;
}
.t-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.75);
  transition: color 0.15s ease, transform 0.3s var(--ease-spring);
}
.t-btn:hover {
  color: #fff;
}
.t-btn:active {
  transform: scale(0.86);
  transition-duration: 0.08s;
}
.t-btn.on {
  color: #fff;
}
.t-btn svg {
  width: 21px;
  height: 21px;
}
.t-btn.play {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.18);
  backdrop-filter: blur(8px);
}
.t-btn.play:hover {
  background: rgba(255, 255, 255, 0.3);
}
.t-btn.play svg {
  width: 26px;
  height: 26px;
}

.seek {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 560px;
  max-width: 90vw;
}
.time {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.75);
  font-variant-numeric: tabular-nums;
  min-width: 36px;
}
.time:last-child {
  text-align: right;
}
.seek .range {
  flex: 1;
  background: linear-gradient(to right, #fff var(--fill, 0%), rgba(255, 255, 255, 0.28) var(--fill, 0%));
}
.seek .range::-webkit-slider-thumb {
  background: #fff;
}
</style>
