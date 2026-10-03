<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Album } from "../stores/library";
import { playTracks } from "../stores/player";
import { deleteTracks } from "../stores/library";
import { openCtx } from "../stores/context";

defineProps<{ album: Album }>();

const emit = defineEmits<{ open: [] }>();

function albumMenu(e: MouseEvent, album: Album): void {
  openCtx(e, [
    { label: "播放专辑", icon: "play", action: () => playTracks(album.tracks, 0) },
    { label: "打开专辑", icon: "album", action: () => emit("open") },
    {
      label: "删除(移入回收站)",
      icon: "delete",
      danger: true,
      action: () => void deleteTracks(album.tracks.map((t) => t.id)),
    },
  ]);
}
</script>

<template>
  <div class="card" @click="emit('open')" @contextmenu="albumMenu($event, album)">
    <div class="cover">
      <img v-if="album.cover" :src="convertFileSrc(album.cover)" loading="lazy" alt="" />
      <div v-else class="ph">♪</div>
      <button class="playbtn" title="播放专辑" @click.stop="playTracks(album.tracks, 0)">
        <svg viewBox="0 0 24 24">
          <path d="M8.6 6.5v11L18 12z" fill="currentColor" />
        </svg>
      </button>
    </div>
    <div class="name" :title="album.album">{{ album.album }}</div>
    <div class="artist" :title="album.artist">{{ album.artist }}</div>
  </div>
</template>

<style scoped>
.card {
  cursor: pointer;
  min-width: 0;
  /* 大曲库性能:视口外的卡片跳过布局与绘制 */
  content-visibility: auto;
  contain-intrinsic-size: auto 240px;
}
.cover {
  position: relative;
  aspect-ratio: 1;
  border-radius: 8px;
  overflow: hidden;
  box-shadow: var(--card-shadow);
  background: var(--bg-3);
  transition: transform 0.45s var(--ease-spring);
}
.card:hover .cover {
  transform: translateY(-4px) scale(1.015);
}
.cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.cover .ph {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 34px;
  color: var(--text-3);
}
.playbtn {
  position: absolute;
  right: 8px;
  bottom: 8px;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: rgba(250, 35, 59, 0.94);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transform: translateY(6px);
  transition: opacity 0.2s ease, transform 0.4s var(--ease-spring);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
}
.playbtn svg {
  width: 20px;
  height: 20px;
}
.card:hover .playbtn {
  opacity: 1;
  transform: translateY(0);
}
.playbtn:hover {
  background: var(--accent-hover);
}

.name {
  margin-top: 10px;
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.artist {
  margin-top: 2px;
  font-size: 12px;
  color: var(--text-2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
