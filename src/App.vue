<script setup lang="ts">
import { computed, onMounted } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import Sidebar from "./components/Sidebar.vue";
import PlayerBar from "./components/PlayerBar.vue";
import NowPlaying from "./components/NowPlaying.vue";
import HomeView from "./views/HomeView.vue";
import AlbumsView from "./views/AlbumsView.vue";
import AlbumDetailView from "./views/AlbumDetailView.vue";
import ArtistsView from "./views/ArtistsView.vue";
import SongsView from "./views/SongsView.vue";
import SearchView from "./views/SearchView.vue";
import { ui, initTheme, initGlass } from "./stores/ui";
import { initLibrary } from "./stores/library";
import { current } from "./stores/player";

const views = {
  home: HomeView,
  albums: AlbumsView,
  album: AlbumDetailView,
  artists: ArtistsView,
  songs: SongsView,
  search: SearchView,
} as const;

const view = computed(() => views[ui.view]);

const ambientImg = computed(() =>
  current.value?.cover ? `url("${convertFileSrc(current.value.cover)}")` : "",
);
const ambientKey = computed(() => current.value?.id ?? "none");

onMounted(() => {
  initTheme();
  initGlass();
  void initLibrary();
});
</script>

<template>
  <div class="app">
    <div v-if="ui.glass" class="ambient">
      <Transition name="amb">
        <div :key="ambientKey" class="ambient-img" :style="{ backgroundImage: ambientImg }"></div>
      </Transition>
      <div class="ambient-scrim"></div>
    </div>

    <div class="shell">
      <Sidebar />
      <main class="content">
        <component :is="view" />
      </main>
      <PlayerBar />
    </div>

    <Transition name="np">
      <NowPlaying v-if="ui.nowPlayingOpen && current" />
    </Transition>
  </div>
</template>

<style scoped>
.app {
  height: 100%;
  position: relative;
  overflow: hidden;
}
.shell {
  display: grid;
  grid-template-columns: 232px 1fr;
  grid-template-rows: 1fr 86px;
  height: 100%;
}
.content {
  position: relative;
  overflow: hidden;
  background: var(--bg);
  min-width: 0;
}

.np-enter-active,
.np-leave-active {
  transition: opacity 0.28s ease, transform 0.28s ease;
}
.np-enter-from,
.np-leave-to {
  opacity: 0;
  transform: translateY(30px);
}
</style>
