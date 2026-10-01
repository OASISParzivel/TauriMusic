<script setup lang="ts">
import { allSongs } from "../stores/library";
import TrackList from "../components/TrackList.vue";
</script>

<template>
  <div class="view">
    <header class="view-head">
      <h1>歌曲</h1>
      <span class="count">{{ allSongs.length }} 首</span>
    </header>
    <TrackList :tracks="allSongs" show-album show-cover />
  </div>
</template>
