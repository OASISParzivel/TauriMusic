<script setup lang="ts">
import { computed, onMounted } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import Sidebar from "./components/Sidebar.vue";
import TopBar from "./components/TopBar.vue";
import NowPlaying from "./components/NowPlaying.vue";
import SettingsModal from "./components/SettingsModal.vue";
import AboutModal from "./components/AboutModal.vue";
import WelcomeModal from "./components/WelcomeModal.vue";
import PlaylistsView from "./views/PlaylistsView.vue";
import PlaylistDetailView from "./views/PlaylistDetailView.vue";
import NameModal from "./components/NameModal.vue";
import PlaylistPickerModal from "./components/PlaylistPickerModal.vue";
import ExportModal from "./components/ExportModal.vue";
import ContextMenu from "./components/ContextMenu.vue";
import HomeView from "./views/HomeView.vue";
import AlbumsView from "./views/AlbumsView.vue";
import AlbumDetailView from "./views/AlbumDetailView.vue";
import ArtistsView from "./views/ArtistsView.vue";
import SongsView from "./views/SongsView.vue";
import SearchView from "./views/SearchView.vue";
import TrashView from "./views/TrashView.vue";
import { ui, initTheme, initGlass, initWelcome } from "./stores/ui";
import { exportState, initLibrary, initDragImport } from "./stores/library";
import { initShortcuts } from "./stores/shortcuts";
import { current } from "./stores/player";
import { nameDialog, picker } from "./stores/playlists";

const views = {
  home: HomeView,
  albums: AlbumsView,
  album: AlbumDetailView,
  artists: ArtistsView,
  songs: SongsView,
  search: SearchView,
  playlists: PlaylistsView,
  playlist: PlaylistDetailView,
  trash: TrashView,
} as const;

const view = computed(() => views[ui.view]);
/* 视图或其参数变化都重新触发页面过渡 */
const pageKey = computed(
  () => `${ui.view}|${ui.albumKey ?? ""}|${ui.artist ?? ""}|${ui.playlistId ?? ""}`,
);

const ambientImg = computed(() => {
  // 氛围背景全屏铺开,用大图档;模糊之下小图也够,但大图切歌时的淡入更细腻
  const src = current.value?.coverLarge ?? current.value?.cover;
  return src ? `url("${convertFileSrc(src)}")` : "";
});
const ambientKey = computed(() => current.value?.id ?? "none");

onMounted(() => {
  initTheme();
  initGlass();
  initWelcome();
  void initLibrary();
  void initDragImport();
  initShortcuts();
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
      <TopBar />
      <Sidebar />
      <main class="content">
        <Transition name="page" mode="out-in">
          <component :is="view" :key="pageKey" />
        </Transition>
      </main>
    </div>

    <Transition name="np">
      <NowPlaying v-if="ui.nowPlayingOpen && current" />
    </Transition>

    <Transition name="modal">
      <SettingsModal v-if="ui.settingsOpen" />
    </Transition>
    <Transition name="modal">
      <AboutModal v-if="ui.aboutOpen" />
    </Transition>
    <Transition name="modal">
      <WelcomeModal v-if="ui.welcomeOpen" />
    </Transition>

    <Transition name="modal">
      <NameModal v-if="nameDialog.open" />
    </Transition>
    <Transition name="modal">
      <PlaylistPickerModal v-if="picker.trackIds" />
    </Transition>
    <Transition name="modal">
      <ExportModal v-if="exportState.open" />
    </Transition>

    <ContextMenu />
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
  grid-template-rows: 48px 1fr;
  height: 100%;
  /* 顶栏与侧栏共用的连续铬底,内容区以圆角纸面浮于其上(macOS 式) */
  background: var(--bg-2);
}
.content {
  position: relative;
  overflow: hidden;
  background: var(--bg);
  min-width: 0;
  border-top-left-radius: 14px;
  box-shadow:
    -1px -1px 0 var(--hairline),
    -8px -4px 20px rgba(0, 0, 0, 0.05);
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

/* 弹窗过渡:卡片弹簧上浮;遮罩用 background-color 淡入。
   遮罩不能动 opacity——overlay 挂着 backdrop-filter,opacity<1 会把元素隔离成
   backdrop root,模糊瞬间采不到背景,弹窗打开的第一帧窗口整个透底 */
.modal-enter-active {
  transition: background-color 0.28s var(--ease-out-soft);
}
.modal-leave-active {
  transition: background-color 0.2s ease;
}
.modal-leave-active :deep(.modal) {
  transition: opacity 0.2s ease;
}
.modal-leave-to :deep(.modal) {
  opacity: 0;
}
.modal-enter-active :deep(.modal) {
  transition: transform 0.38s var(--ease-spring);
}
.modal-enter-from {
  background-color: rgba(0, 0, 0, 0);
}
.modal-leave-to {
  background-color: rgba(0, 0, 0, 0);
}
.modal-enter-from :deep(.modal) {
  transform: translateY(16px) scale(0.965);
}
</style>
