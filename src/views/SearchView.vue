<script setup lang="ts">
import { computed } from "vue";
import type { Track } from "../api";
import { albums, artists, lib, type Album } from "../stores/library";
import { openAlbum, openArtist, ui } from "../stores/ui";
import TrackList from "../components/TrackList.vue";
import AlbumCard from "../components/AlbumCard.vue";

const query = computed(() => ui.search.trim().toLowerCase());

const songs = computed<Track[]>(() => {
  const q = query.value;
  if (!q) return [];
  return lib.tracks.filter(
    (t) =>
      t.title.toLowerCase().includes(q) ||
      t.artist.toLowerCase().includes(q) ||
      t.album.toLowerCase().includes(q),
  );
});

const matchedAlbums = computed<Album[]>(() => {
  const q = query.value;
  if (!q) return [];
  return albums.value.filter(
    (a) => a.album.toLowerCase().includes(q) || a.artist.toLowerCase().includes(q),
  );
});

const matchedArtists = computed<string[]>(() => {
  const q = query.value;
  if (!q) return [];
  return artists.value.filter((a) => a.toLowerCase().includes(q));
});

const nothing = computed(
  () =>
    query.value &&
    songs.value.length === 0 &&
    matchedAlbums.value.length === 0 &&
    matchedArtists.value.length === 0,
);
</script>

<template>
  <div class="view">
    <template v-if="query">
      <section v-if="songs.length" class="section">
        <h2>歌曲</h2>
        <TrackList :tracks="songs" show-album show-cover />
      </section>

      <section v-if="matchedAlbums.length" class="section">
        <h2>专辑</h2>
        <div class="grid">
          <AlbumCard v-for="a in matchedAlbums" :key="a.key" :album="a" @open="openAlbum(a.key)" />
        </div>
      </section>

      <section v-if="matchedArtists.length" class="section">
        <h2>艺人</h2>
        <div class="artist-grid">
          <button v-for="a in matchedArtists" :key="a" class="artist-chip" @click="openArtist(a)">
            <span class="avatar">{{ a.slice(0, 1) }}</span>
            {{ a }}
          </button>
        </div>
      </section>

      <div v-if="nothing" class="empty">
        <h2>没有找到“{{ ui.search }}”相关内容</h2>
      </div>
    </template>

    <div v-else class="empty">
      <h2>搜索你的音乐</h2>
      <p>按歌曲、专辑或艺人名称搜索</p>
    </div>
  </div>
</template>

<style scoped>
.section {
  margin-bottom: 30px;
}
.section h2 {
  font-size: 17px;
  font-weight: 700;
  margin-bottom: 12px;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 20px 18px;
}
.artist-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
.artist-chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 5px 12px 5px 5px;
  border-radius: 999px;
  background: var(--pill-bg);
  font-size: 13px;
}
.artist-chip:hover {
  filter: brightness(1.05);
  background: var(--active);
}
.avatar {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: var(--accent);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
}
</style>
