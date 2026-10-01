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
/* 视图或其参数变化都重新触发页面过渡 */
const pageKey = computed(() => `${ui.view}|${ui.albumKey ?? ""}|${ui.artist ?? ""}`);

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
    <!-- SVG 折射滤镜:供玻璃卡片 backdrop-filter 引用(隐藏元素) -->
    <svg class="fx-defs" width="0" height="0" aria-hidden="true" focusable="false">
      <filter id="glass-refract" x="-20%" y="-20%" width="140%" height="140%" color-interpolation-filters="sRGB">
        <feTurbulence type="fractalNoise" baseFrequency="0.006 0.009" numOctaves="2" seed="7" result="noise" />
        <feGaussianBlur in="noise" stdDeviation="3" result="soft" />
        <feDisplacementMap in="SourceGraphic" in2="soft" scale="46" xChannelSelector="R" yChannelSelector="G" />
      </filter>
    </svg>

    <div v-if="ui.glass" class="ambient">
      <Transition name="amb">
        <div :key="ambientKey" class="ambient-img" :style="{ backgroundImage: ambientImg }"></div>
      </Transition>
      <div class="ambient-scrim"></div>
    </div>

    <div class="shell">
      <Sidebar />
      <main class="content">
        <Transition name="page" mode="out-in">
          <component :is="view" :key="pageKey" />
        </Transition>
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
.fx-defs {
  position: absolute;
  width: 0;
  height: 0;
}

/* 页面过渡:旧页快速虚化下沉,新页带模糊上浮落定 —— 玻璃的"流动感" */
.page-enter-active {
  transition:
    opacity 0.34s var(--ease-out-soft),
    transform 0.34s var(--ease-out-soft),
    filter 0.34s var(--ease-out-soft);
}
.page-leave-active {
  transition:
    opacity 0.16s ease,
    transform 0.16s ease,
    filter 0.16s ease;
}
.page-enter-from {
  opacity: 0;
  transform: translateY(14px) scale(0.992);
  filter: blur(10px);
}
.page-leave-to {
  opacity: 0;
  transform: scale(0.996);
  filter: blur(6px);
}

/* 全屏播放器:像一层玻璃从底部液化升起 */
.np-enter-active {
  transition:
    opacity 0.42s var(--ease-out-soft),
    transform 0.5s var(--ease-out-soft),
    filter 0.42s var(--ease-out-soft);
}
.np-leave-active {
  transition:
    opacity 0.26s ease,
    transform 0.3s ease,
    filter 0.26s ease;
}
.np-enter-from {
  opacity: 0;
  transform: translateY(48px) scale(1.015);
  filter: blur(14px);
}
.np-leave-to {
  opacity: 0;
  transform: translateY(30px) scale(0.99);
  filter: blur(8px);
}
</style>
