<script setup lang="ts">
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { albumByKey } from "../stores/library";
import { playTracks, player } from "../stores/player";
import { ui, go } from "../stores/ui";
import TrackList from "../components/TrackList.vue";

const album = computed(() => albumByKey(ui.albumKey ?? ""));
const years = computed(() => {
  const ys = new Set(album.value?.tracks.map((t) => t.year).filter((y): y is number => y != null) ?? []);
  return ys.size === 1 ? String([...ys][0]) : null;
});
const genres = computed(() => {
  const gs = new Set(album.value?.tracks.map((t) => t.genre).filter((g): g is string => !!g) ?? []);
  return gs.size === 1 ? [...gs][0] : null;
});
const meta = computed(() =>
  [years.value, genres.value, album.value ? `${album.value.tracks.length} 首歌曲` : null]
    .filter(Boolean)
    .join(" · "),
);

function shuffleAll(): void {
  if (!album.value) return;
  player.shuffle = true;
  playTracks(album.value.tracks, Math.floor(Math.random() * album.value.tracks.length));
}
</script>

<template>
  <div v-if="album" class="view">
    <header class="detail-head">
      <div class="cover">
        <img v-if="album.cover" :src="convertFileSrc(album.cover)" alt="" />
        <div v-else class="ph">♪</div>
      </div>
      <div class="meta-col">
        <div class="kind">专辑</div>
        <h1>{{ album.album }}</h1>
        <div class="artist" @click="go('artists')">{{ album.artist }}</div>
        <div class="sub">{{ meta }}</div>
        <div class="btns">
          <button class="primary-pill" @click="playTracks(album.tracks, 0)">播放</button>
          <button class="ghost-pill" @click="shuffleAll">随机播放</button>
        </div>
      </div>
    </header>

    <TrackList :tracks="album.tracks" show-index />
  </div>
  <div v-else class="view">
    <div class="empty">
      <h2>未找到该专辑</h2>
      <button class="primary-pill" @click="go('albums')">返回专辑列表</button>
    </div>
  </div>
</template>

<style scoped>
.detail-head {
  display: flex;
  gap: 28px;
  align-items: flex-end;
  margin-bottom: 28px;
}
.cover {
  width: 210px;
  height: 210px;
  flex: none;
  border-radius: 10px;
  overflow: hidden;
  box-shadow: var(--card-shadow);
  background: var(--bg-3);
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
  font-size: 56px;
  color: var(--text-3);
}

.meta-col {
  min-width: 0;
  padding-bottom: 4px;
}
.kind {
  font-size: 12px;
  font-weight: 700;
  color: var(--accent);
  letter-spacing: 0.4px;
}
h1 {
  font-size: 26px;
  font-weight: 700;
  margin-top: 4px;
}
.artist {
  font-size: 15px;
  font-weight: 600;
  color: var(--accent);
  margin-top: 6px;
  cursor: pointer;
  width: fit-content;
}
.artist:hover {
  text-decoration: underline;
}
.sub {
  font-size: 13px;
  color: var(--text-2);
  margin-top: 6px;
}
.btns {
  display: flex;
  gap: 12px;
  margin-top: 16px;
}
.ghost-pill {
  background: var(--pill-bg);
  color: var(--text);
  border-radius: 999px;
  padding: 8px 22px;
  font-size: 14px;
  font-weight: 600;
  transition: filter 0.15s ease;
}
.ghost-pill:hover {
  filter: brightness(1.06);
}
</style>
