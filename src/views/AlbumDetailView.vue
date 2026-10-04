<script setup lang="ts">
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { albumByKey, enrichAlbumNetease, deleteTracks, exportTracksTmc } from "../stores/library";
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
  player.mode = "shuffle";
  playTracks(album.value.tracks, Math.floor(Math.random() * album.value.tracks.length));
}

const enriching = ref(false);
function onEnrich(): void {
  if (!album.value || enriching.value) return;
  enriching.value = true;
  void enrichAlbumNetease(album.value.key).finally(() => (enriching.value = false));
}

/** 整张专辑导出为 TMC 音乐包(每首一个,存到所选文件夹) */
function onExportAlbum(): void {
  if (!album.value) return;
  void exportTracksTmc(
    album.value.tracks.map((t) => t.id),
    album.value.tracks,
    "整张专辑",
  );
}

/** 删除整张专辑(两段确认,确认后回到专辑列表) */
const confirmingDelete = ref(false);
let confirmTimer = 0;
function onDeleteAlbum(): void {
  if (!album.value) return;
  if (!confirmingDelete.value) {
    confirmingDelete.value = true;
    window.clearTimeout(confirmTimer);
    confirmTimer = window.setTimeout(() => (confirmingDelete.value = false), 3000);
    return;
  }
  confirmingDelete.value = false;
  const ids = album.value.tracks.map((t) => t.id);
  // 删除失败(文件被占用)时留在原页,状态条已提示
  void deleteTracks(ids).then((ok) => {
    if (ok) go("albums");
  });
}
</script>

<template>
  <div v-if="album" class="view">
    <button class="back" @click="go('albums')">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M14.5 5.5 8 12l6.5 6.5" />
      </svg>
      专辑
    </button>
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
          <button
            class="ghost-pill"
            title="每首歌打包成一个 TMC 音乐包,存到所选文件夹"
            @click="onExportAlbum"
          >
            导出专辑
          </button>
          <button
            class="ghost-pill"
            title="通过网易云公开接口补全封面与歌词(仅元数据,不影响本地文件播放)"
            :disabled="enriching"
            @click="onEnrich"
          >
            {{ enriching ? "匹配中…" : "在线匹配" }}
          </button>
          <button
            class="ghost-pill danger"
            :class="{ confirm: confirmingDelete }"
            :title="confirmingDelete ? '再次点击确认删除整张专辑(音频移入回收站)' : '删除这张专辑(音频移入回收站)'"
            @click="onDeleteAlbum"
          >
            {{ confirmingDelete ? "确认删除?" : "删除专辑" }}
          </button>
        </div>
      </div>
    </header>

    <TrackList :tracks="album.tracks" show-index virtual />
  </div>
  <div v-else class="view">
    <div class="empty">
      <h2>未找到该专辑</h2>
      <button class="primary-pill" @click="go('albums')">返回专辑列表</button>
    </div>
  </div>
</template>

<style scoped>
/* 虚拟滚动:视图变为有界容器,专辑头部固定,曲目列表自身滚动 */
.view {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.view :deep(.detail-head) {
  flex: none;
  margin-bottom: 14px;
}
.view :deep(.tl.virtual) {
  flex: 1;
  min-height: 0;
}

.back {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  width: fit-content;
  color: var(--accent);
  font-size: 13.5px;
  font-weight: 600;
  padding: 4px 10px 4px 4px;
  margin-bottom: 14px;
  border-radius: 7px;
  transition: background 0.2s ease, transform 0.35s var(--ease-spring);
}
.back svg {
  width: 17px;
  height: 17px;
}
.back:hover {
  background: var(--hover);
}
.back:active {
  transform: scale(0.97);
  transition-duration: 0.1s;
}

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
  transition: filter 0.2s ease, transform 0.35s var(--ease-spring);
}
.ghost-pill:hover {
  filter: brightness(1.06);
}
.ghost-pill.danger.confirm {
  color: #e81123;
  border: 1px solid #e81123;
}
</style>
