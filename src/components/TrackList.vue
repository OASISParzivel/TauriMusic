<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Track } from "../api";
import { current, fmtTime, playTrack } from "../stores/player";
import { exportTrackTmc } from "../stores/library";

const props = defineProps<{
  tracks: Track[];
  /** 显示序号(专辑内曲目) */
  showIndex?: boolean;
  /** 显示封面缩略图 */
  showCover?: boolean;
  /** 显示专辑列 */
  showAlbum?: boolean;
}>();

function play(t: Track): void {
  playTrack(t, props.tracks);
}

function exportOne(t: Track): void {
  void exportTrackTmc(t.id, t.title, t.artist);
}
</script>

<template>
  <div class="tl">
    <div
      v-for="(t, i) in tracks"
      :key="t.id + '-' + i"
      class="row"
      :class="{ current: t.id === current?.id }"
      @click="play(t)"
    >
      <span v-if="showIndex" class="idx">
        <span class="num">{{ t.trackNo ?? i + 1 }}</span>
        <svg class="play-ico" viewBox="0 0 24 24"><path d="M8.6 6.5v11L18 12z" fill="currentColor" /></svg>
      </span>
      <span v-if="showCover" class="cov">
        <img v-if="t.cover" :src="convertFileSrc(t.cover)" loading="lazy" alt="" />
        <span v-else class="ph">♪</span>
        <svg class="play-ico" viewBox="0 0 24 24"><path d="M8.6 6.5v11L18 12z" fill="currentColor" /></svg>
      </span>
      <span class="main">
        <span class="t" :title="t.title">{{ t.title }}</span>
        <span class="a" :title="t.artist">{{ t.artist }}</span>
      </span>
      <span v-if="showAlbum" class="al" :title="t.album">{{ t.album }}</span>
      <span v-if="t.hasLyrics" class="badge">词</span>
      <button class="exp" title="导出为 TMC 音乐包" @click.stop="exportOne(t)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 4v9M8.5 9.5 12 13l3.5-3.5M5 17.5h14" />
        </svg>
      </button>
      <span class="dur">{{ fmtTime(t.duration) }}</span>
    </div>
  </div>
</template>

<style scoped>
.tl {
  display: flex;
  flex-direction: column;
}

.row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 10px;
  border-radius: 8px;
  cursor: pointer;
  min-width: 0;
  transition: background 0.25s var(--ease-out-soft);
}
.row:hover {
  background: var(--hover);
}
.row.current .t {
  color: var(--accent);
}

.idx {
  width: 24px;
  flex: none;
  text-align: center;
  color: var(--text-2);
  font-variant-numeric: tabular-nums;
  display: flex;
  align-items: center;
  justify-content: center;
}
.idx .play-ico {
  display: none;
  width: 16px;
  height: 16px;
  color: var(--text);
}
.row:hover .idx .num {
  display: none;
}
.row:hover .idx .play-ico {
  display: block;
}

.cov {
  position: relative;
  width: 40px;
  height: 40px;
  flex: none;
  border-radius: 5px;
  overflow: hidden;
  background: var(--bg-3);
}
.cov img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.cov .ph {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-3);
  font-size: 16px;
}
.cov .play-ico {
  position: absolute;
  inset: 0;
  margin: auto;
  width: 18px;
  height: 18px;
  color: #fff;
  filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.6));
  opacity: 0;
  transition: opacity 0.12s ease;
}
.row:hover .cov .play-ico {
  opacity: 1;
}

.main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.t {
  font-size: 13.5px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.a {
  font-size: 12px;
  color: var(--text-2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.al {
  width: 26%;
  flex: none;
  font-size: 12.5px;
  color: var(--text-2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.badge {
  flex: none;
  font-size: 10px;
  color: var(--accent);
  border: 1px solid var(--accent);
  border-radius: 4px;
  padding: 0 4px;
  line-height: 15px;
  opacity: 0.85;
}

.exp {
  flex: none;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-3);
  opacity: 0;
  transition: opacity 0.2s ease, background 0.2s ease, color 0.2s ease;
}
.exp svg {
  width: 15px;
  height: 15px;
}
.row:hover .exp {
  opacity: 1;
}
.exp:hover {
  background: var(--hover);
  color: var(--text);
}

.dur {
  flex: none;
  width: 40px;
  text-align: right;
  font-size: 12px;
  color: var(--text-2);
  font-variant-numeric: tabular-nums;
}
</style>
