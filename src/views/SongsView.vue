<script setup lang="ts">
import { allSongs, lib, enrichAllNetease } from "../stores/library";
import TrackList from "../components/TrackList.vue";
</script>

<template>
  <div class="view">
    <header class="view-head">
      <h1>歌曲</h1>
      <div class="right">
        <span class="count">{{ allSongs.length }} 首</span>
        <button class="match-btn" :disabled="lib.matching" @click="enrichAllNetease">
          {{ lib.matching ? "匹配中…" : "全部在线匹配" }}
        </button>
      </div>
    </header>
    <TrackList :tracks="allSongs" show-album show-cover />
  </div>
</template>

<style scoped>
.right {
  display: flex;
  align-items: center;
  gap: 16px;
}
.match-btn {
  font-size: 12.5px;
  font-weight: 600;
  color: #fff;
  background: var(--accent);
  border-radius: 999px;
  padding: 6px 16px;
  transition: background 0.2s ease, transform 0.35s var(--ease-spring), opacity 0.2s ease;
}
.match-btn:hover:not(:disabled) {
  background: var(--accent-hover);
}
.match-btn:active:not(:disabled) {
  transform: scale(0.97);
  transition-duration: 0.09s;
}
.match-btn:disabled {
  opacity: 0.55;
  cursor: default;
}
</style>
