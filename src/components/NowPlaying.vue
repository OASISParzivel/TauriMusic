<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api, type LyricLine } from "../api";
import { player, current, toggle, next, prev, seek, setVolume, fmtTime } from "../stores/player";
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
const volFill = computed(() => `${player.volume * 100}%`);

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
      <!-- 三区一行:模式键回到播放控制旁(左),控制组居中,音量在右 -->
      <div class="controls">
        <div class="zone left">
          <ModeIcon class="t-btn" />
        </div>
        <div class="zone mid">
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
        <div class="zone right">
          <div class="np-volume">
            <svg class="vol-ico" viewBox="0 0 24 24">
              <path d="M11.2 4.8 6.8 8.3H4v7.4h2.8l4.4 3.5z" fill="currentColor" />
              <path d="M14.5 9.3a3.9 3.9 0 0 1 0 5.4M17 7a7.2 7.2 0 0 1 0 10" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
            </svg>
            <input
              class="range vol"
              type="range"
              min="0"
              max="1"
              step="0.01"
              :value="player.volume"
              :style="{ '--fill': volFill }"
              @input="setVolume(Number(($event.target as HTMLInputElement).value))"
            />
          </div>
        </div>
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
  /* 播放页局部令牌:纸质皮肤在 style.css 末尾整组覆写(印刷节目单) */
  --np-bg: #242426;
  --np-fg: #ffffff;
  --np-fg-2: rgba(255, 255, 255, 0.62);
  --np-fg-3: rgba(255, 255, 255, 0.42);
  --np-fg-soft: rgba(255, 255, 255, 0.75);
  --np-fg-strong: rgba(255, 255, 255, 0.9);
  --np-fg-mute: rgba(255, 255, 255, 0.55);
  --np-plain: rgba(255, 255, 255, 0.8);
  --np-btn-bg: rgba(255, 255, 255, 0.18);
  --np-btn-bg-hover: rgba(255, 255, 255, 0.3);
  --np-close-bg: rgba(255, 255, 255, 0.14);
  --np-close-hover: rgba(255, 255, 255, 0.26);
  --np-close-fg: rgba(255, 255, 255, 0.85);
  --np-track: rgba(255, 255, 255, 0.28);
  --np-thumb: #ffffff;
  --np-art-shadow: 0 18px 60px rgba(0, 0, 0, 0.55);
  --np-art-bg: rgba(255, 255, 255, 0.08);
  --np-text-shadow: 0 1px 8px rgba(0, 0, 0, 0.4);
  position: absolute;
  inset: 0;
  z-index: 60;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: var(--np-fg);
  background: var(--np-bg);
  /* 顶栏悬浮在最上层(z-100):为它预留 48px,标题/封面/歌词不再钻到顶栏底下 */
  padding-top: 48px;
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
  color: var(--np-close-fg);
  background: var(--np-close-bg);
  backdrop-filter: blur(10px);
  transition: background 0.25s var(--ease-out-soft), transform 0.35s var(--ease-spring);
}
.close:hover {
  background: var(--np-close-hover);
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
  box-shadow: var(--np-art-shadow);
  background: var(--np-art-bg);
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
  color: var(--np-fg-3);
}

.info {
  position: absolute;
  top: 26px;
  left: 56px;
  max-width: 420px;
  text-shadow: var(--np-text-shadow);
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
  color: var(--np-fg-3);
  cursor: pointer;
  transition: color 0.4s var(--ease-out-soft), transform 0.4s var(--ease-out-soft);
  transform-origin: left center;
}
.lyrics.synced p.near {
  color: var(--np-fg-2);
}
.lyrics.synced p.active {
  color: var(--np-fg);
  transform: scale(1.02);
}
.lyrics.synced p:hover {
  color: var(--np-fg-strong);
}
.lyrics.plain p {
  font-size: 16px;
  line-height: 1.9;
  color: var(--np-plain);
  white-space: pre-wrap;
}
.lyrics.none {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
  color: var(--np-fg-mute);
}

.bottom {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 10px 0 26px;
}
/* 音量:与顶栏同款控件,在播放页深色底上用白色轨道/滑块 */
.np-volume {
  display: flex;
  align-items: center;
  gap: 10px;
}
.np-volume .vol-ico {
  width: 16px;
  height: 16px;
  color: var(--np-fg-soft);
}
.np-volume .vol {
  width: 130px;
  background: linear-gradient(
      to right,
      var(--np-fg) var(--fill, 0%),
      var(--np-track) var(--fill, 0%)
  );
}
.np-volume .range::-webkit-slider-thumb {
  background: var(--np-thumb);
}
/* 控制行:与进度条同宽同列——三区(模式键/播放控制/音量)都在进度条的宽度内分布,
   不再被 stretch 拉到窗口两角;窄窗口时左右区让位给中间的控制组 */
.controls {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  width: 560px;
  max-width: 90vw;
}
.zone.left {
  justify-self: start;
}
.zone.mid {
  display: flex;
  align-items: center;
  gap: 26px;
}
.zone.right {
  justify-self: end;
}
.t-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--np-fg-soft);
  transition: color 0.15s ease, transform 0.3s var(--ease-spring);
}
.t-btn:hover {
  color: var(--np-fg);
}
.t-btn:active {
  transform: scale(0.86);
  transition-duration: 0.08s;
}
.t-btn.on {
  color: var(--np-fg);
}
.t-btn svg {
  width: 21px;
  height: 21px;
}
.t-btn.play {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: var(--np-btn-bg);
  backdrop-filter: blur(8px);
}
.t-btn.play:hover {
  background: var(--np-btn-bg-hover);
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
  color: var(--np-fg-soft);
  font-variant-numeric: tabular-nums;
  min-width: 36px;
}
.time:last-child {
  text-align: right;
}
.seek .range {
  flex: 1;
  background: linear-gradient(to right, var(--np-fg) var(--fill, 0%), var(--np-track) var(--fill, 0%));
}
.seek .range::-webkit-slider-thumb {
  background: var(--np-thumb);
}

/* ===== 纸质皮肤:播放页变"印刷节目单"——纸底、墨字、衬线歌词、封面装裱 ===== */
html[data-skin="paper"] .np {
  --np-bg: var(--bg);
  --np-fg: var(--text);
  --np-fg-2: var(--text-2);
  --np-fg-3: var(--text-3);
  --np-fg-soft: var(--ink);
  --np-fg-strong: var(--ink);
  --np-fg-mute: var(--ink-2);
  --np-plain: var(--text);
  --np-btn-bg: transparent;
  --np-btn-bg-hover: var(--hover);
  --np-close-bg: transparent;
  --np-close-hover: var(--hover);
  --np-close-fg: var(--ink);
  --np-track: rgba(31, 30, 29, 0.18);
  --np-thumb: var(--ink);
  --np-art-shadow: var(--paper-shadow);
  --np-art-bg: var(--bg-3);
  --np-text-shadow: none;
  background: var(--bg);
}
html[data-skin="paper"] .np::before {
  display: none; /* 纸上不放模糊封面:节目单是干净的纸面 */
}
html[data-skin="paper"] .np .info {
  text-shadow: none;
}
html[data-skin="paper"] .np .info .title {
  font-family: var(--font-display);
}
html[data-skin="paper"] .np .lyrics {
  font-family: var(--font-display);
}
html[data-skin="paper"] .np .lyrics.synced p.active {
  color: var(--accent);
}
html[data-skin="paper"] .np .art {
  border: 1px solid var(--ink);
  box-shadow: var(--paper-shadow);
}
html[data-skin="paper"] .np .t-btn.play {
  border: 1px solid var(--ink);
}
html[data-skin="paper"] .np .close {
  border: 1px solid var(--ink);
}
</style>