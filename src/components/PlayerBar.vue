<script setup lang="ts">
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { player, current, toggle, next, prev, seek, setVolume, cycleRepeat, fmtTime } from "../stores/player";
import { ui } from "../stores/ui";

const coverUrl = computed(() => (current.value?.cover ? convertFileSrc(current.value.cover) : null));
const progress = computed(() => (player.duration > 0 ? (player.position / player.duration) * 100 : 0));
const volFill = computed(() => `${player.volume * 100}%`);

function onSeek(e: Event): void {
  seek(Number((e.target as HTMLInputElement).value));
}
</script>

<template>
  <footer class="playerbar">
    <div class="left" :class="{ clickable: current }" @click="current && (ui.nowPlayingOpen = true)">
      <div class="thumb">
        <img v-if="coverUrl" :src="coverUrl" alt="" />
        <div v-else class="ph">♪</div>
      </div>
      <div v-if="current" class="meta">
        <div class="title" :title="current.title">{{ current.title }}</div>
        <div class="artist" :title="current.artist">{{ current.artist }}</div>
      </div>
      <div v-else class="meta">
        <div class="title muted">未在播放</div>
      </div>
    </div>

    <div class="center">
      <div class="controls">
        <button class="t-btn" :class="{ on: player.shuffle }" title="随机播放" @click="player.shuffle = !player.shuffle">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M2.5 18h1.6c1.3 0 2.6-.6 3.4-1.7l6-8.6c.8-1.1 2.1-1.7 3.4-1.7h3.1" />
            <path d="m18.5 2.5 3 3.5-3 3.5" />
            <path d="M2.5 6h1.6c1.5 0 2.9.9 3.7 2.2" />
            <path d="M20 18h-4.1c-1.3 0-2.5-.7-3.3-1.8l-.6-.8" />
            <path d="m18.5 14.5 3 3.5-3 3.5" />
          </svg>
        </button>
        <button class="t-btn big" title="上一曲" @click="prev()">
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
        <button class="t-btn big" title="下一曲" @click="next()">
          <svg viewBox="0 0 24 24">
            <path d="M17 5.8v12.4" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" />
            <path d="M5.5 6.3v11.4L14.6 12z" fill="currentColor" />
          </svg>
        </button>
        <button class="t-btn" :class="{ on: player.repeat !== 'off' }" title="循环模式" @click="cycleRepeat()">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="m17 2.5 3.5 3.5-3.5 3.5" />
            <path d="M3.5 11.5v-1a4.5 4.5 0 0 1 4.5-4.5h12" />
            <path d="m7 21.5-3.5-3.5L7 14.5" />
            <path d="M20.5 12.5v1a4.5 4.5 0 0 1-4.5 4.5H4" />
          </svg>
          <span v-if="player.repeat === 'one'" class="badge">1</span>
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
          :disabled="!current"
          @input="onSeek"
        />
        <span class="time">{{ fmtTime(player.duration) }}</span>
      </div>
    </div>

    <div class="right">
      <button class="t-btn" title="歌词" :disabled="!current" @click="ui.nowPlayingOpen = true">
        <svg viewBox="0 0 24 24">
          <path
            d="M4.6 12.2c0-3.4 2-5.7 4.9-6.5l.5 1.5c-1.7.6-2.7 1.7-3 3 .4-.2.8-.3 1.3-.3 1.3 0 2.3 1 2.3 2.4s-1.1 2.5-2.6 2.5c-2 0-3.4-1.2-3.4-2.6zm7 0c0-3.4 2-5.7 4.9-6.5l.5 1.5c-1.7.6-2.7 1.7-3 3 .4-.2.8-.3 1.3-.3 1.3 0 2.3 1 2.3 2.4s-1.1 2.5-2.6 2.5c-2 0-3.4-1.2-3.4-2.6z"
            fill="currentColor"
          />
        </svg>
      </button>
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
  </footer>
</template>

<style scoped>
.playerbar {
  grid-column: 1 / 3;
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  background: var(--bar-bg);
  backdrop-filter: blur(24px);
  border-top: 1px solid var(--hairline);
  padding: 0 20px;
  min-height: 0;
}

.left {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
  padding: 10px 0;
}
.left.clickable {
  cursor: pointer;
}
.thumb {
  width: 52px;
  height: 52px;
  border-radius: 6px;
  overflow: hidden;
  box-shadow: var(--card-shadow);
  flex: none;
  background: var(--bg-3);
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.thumb .ph {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  color: var(--text-3);
}
.meta {
  min-width: 0;
}
.title {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.title.muted {
  color: var(--text-3);
  font-weight: 500;
}
.artist {
  font-size: 12px;
  color: var(--text-2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}

.center {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px 24px;
}

.controls {
  display: flex;
  align-items: center;
  gap: 18px;
}
.t-btn {
  position: relative;
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-2);
  transition: color 0.12s ease, transform 0.08s ease;
}
.t-btn:hover:not(:disabled) {
  color: var(--text);
}
.t-btn:active:not(:disabled) {
  transform: scale(0.92);
}
.t-btn:disabled {
  opacity: 0.35;
  cursor: default;
}
.t-btn.on {
  color: var(--accent);
}
.t-btn svg {
  width: 20px;
  height: 20px;
}
.t-btn.big svg {
  width: 22px;
  height: 22px;
}
.t-btn.play {
  width: 34px;
  height: 34px;
}
.t-btn.play svg {
  width: 28px;
  height: 28px;
}
.badge {
  position: absolute;
  top: 1px;
  right: 0;
  font-size: 8px;
  font-weight: 700;
  color: var(--accent);
}

.seek {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 460px;
  max-width: 100%;
}
.time {
  font-size: 11px;
  color: var(--text-2);
  font-variant-numeric: tabular-nums;
  min-width: 34px;
}
.time:last-child {
  text-align: right;
}
.seek .range {
  flex: 1;
}

.right {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
}
.vol-ico {
  width: 17px;
  height: 17px;
  color: var(--text-2);
  flex: none;
}
.vol {
  width: 92px;
}
</style>
