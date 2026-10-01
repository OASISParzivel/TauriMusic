<script setup lang="ts">
import { computed } from "vue";
import { albums } from "../stores/library";
import { openAlbum } from "../stores/ui";
import AlbumCard from "../components/AlbumCard.vue";

const sorted = computed(() =>
  [...albums.value].sort(
    (a, b) => a.artist.localeCompare(b.artist, "zh") || a.album.localeCompare(b.album, "zh"),
  ),
);
</script>

<template>
  <div class="view">
    <header class="view-head">
      <h1>专辑</h1>
      <span class="count">{{ sorted.length }} 张</span>
    </header>
    <div class="grid">
      <AlbumCard v-for="a in sorted" :key="a.key" :album="a" @open="openAlbum(a.key)" />
    </div>
  </div>
</template>

<style scoped>
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(168px, 1fr));
  gap: 24px 20px;
}
</style>
