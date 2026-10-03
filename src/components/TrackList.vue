<script setup lang="ts">
import { ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Track } from "../api";
import { current, fmtTime, playTrack } from "../stores/player";
import { exportTrackTmc, deleteTracks } from "../stores/library";
import { openAlbum, openArtist } from "../stores/ui";
import { openCtx } from "../stores/context";

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

/** 删除采用两段确认:第一次点进入确认态,3 秒内再点执行 */
const confirmingId = ref<string | null>(null);
let confirmTimer = 0;

function onDelete(t: Track): void {
  if (confirmingId.value !== t.id) {
    confirmingId.value = t.id;
    window.clearTimeout(confirmTimer);
    confirmTimer = window.setTimeout(() => (confirmingId.value = null), 3000);
    return;
  }
  confirmingId.value = null;
  void deleteTracks([t.id]);
}

/** 歌曲右键菜单 */
function rowMenu(e: MouseEvent, t: Track): void {
  openCtx(e, [
    { label: "播放", icon: "play", action: () => play(t) },
    { label: "查看专辑", icon: "album", action: () => openAlbum(`${t.albumArtist || t.artist}\u{1}${t.album}`) },
    { label: "查看艺人", icon: "artist", action: () => openArtist(t.artist) },
    { label: "导出为 TMC 音乐包", icon: "export", action: () => exportOne(t) },
    { label: "删除(移入回收站)", icon: "delete", danger: true, action: () => void deleteTracks([t.id]) },
  ]);
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
      @contextmenu="rowMenu($event, t)"
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
          <path d="M12 13.5V4M8.5 7.5 12 4l3.5 3.5" />
          <path d="M5 15v2.6c0 .77.63 1.4 1.4 1.4h11.2c.77 0 1.4-.63 1.4-1.4V15" />
        </svg>
      </button>
      <button
        class="del"
        :class="{ confirm: confirmingId === t.id }"
        :title="confirmingId === t.id ? '再次点击确认删除(移入回收站)' : '删除这首歌(移入回收站)'"
        @click.stop="onDelete(t)"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M4.5 7h15M9.5 7V5.2c0-.66.54-1.2 1.2-1.2h2.6c.66 0 1.2.54 1.2 1.2V7M6.5 7l.8 11.3c.06.77.7 1.2 1.4 1.2h6.6c.7 0 1.34-.43 1.4-1.2L17.5 7" />
          <path d="M10 11v5M14 11v5" />
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
  /* 大曲库性能:视口外的行跳过布局与绘制 */
  content-visibility: auto;
  contain-intrinsic-size: auto 54px;
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
.del {
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
.del svg {
  width: 15px;
  height: 15px;
}
.row:hover .del {
  opacity: 1;
}
.del:hover {
  background: var(--hover);
  color: var(--text);
}
.del.confirm {
  opacity: 1;
  background: rgba(232, 17, 35, 0.12);
  color: #e81123;
}
.del.confirm:hover {
  background: rgba(232, 17, 35, 0.2);
  color: #e81123;
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
