<script setup lang="ts">
import { computed } from "vue";
import { albums, lib, addFolder, removeFolder, rescan, openImportDir, reloadLibrary } from "../stores/library";
import { go, openAlbum } from "../stores/ui";
import AlbumCard from "../components/AlbumCard.vue";

/** 最近添加:严格按入库时间(addedAt)取最新的 12 张专辑。
 *  不用 mtime 参与排序——重扫/在线匹配会 touch 文件,把整个库"变新";也不展示全库,
 *  全库浏览是「专辑」页的职责,这里只回答"最近往库里加了什么" */
const recent = computed(() => {
  const addedAt = new Map(albums.value.map((a) => [a.key, Math.max(...a.tracks.map((t) => t.addedAt), 0)]));
  return [...albums.value]
    .sort(
      (a, b) =>
        (addedAt.get(b.key) ?? 0) - (addedAt.get(a.key) ?? 0) || a.artist.localeCompare(b.artist, "zh"),
    )
    .slice(0, 12);
});

function shortPath(p: string): string {
  const parts = p.split(/[\\/]/).filter(Boolean);
  return parts.length > 2 ? "…/" + parts.slice(-2).join("/") : p;
}
</script>

<template>
  <div class="view">
    <div v-if="!lib.loaded" class="empty">
      <h2>正在加载曲库…</h2>
    </div>

    <div v-else-if="lib.loadError" class="empty">
      <h2>曲库读取失败</h2>
      <p class="load-err">{{ lib.loadError }}</p>
      <button class="primary-pill" style="margin-top: 8px" @click="reloadLibrary">重试</button>
    </div>

    <div v-else-if="lib.folders.length === 0" class="empty">
      <svg class="icon" width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
        <path d="M9.2 17.3V7l9-1.9v10.2" />
        <circle cx="6.9" cy="17.4" r="2.3" />
        <circle cx="15.9" cy="15.4" r="2.3" />
      </svg>
      <h2>欢迎使用 TauriMusic</h2>
      <p>添加一个音乐文件夹,自动识别封面、歌词与专辑信息</p>
      <button class="primary-pill" style="margin-top: 8px" @click="addFolder">添加音乐文件夹</button>
    </div>

    <div v-else-if="lib.tracks.length === 0" class="empty">
      <svg class="icon" width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
        <path d="M9.2 17.3V7l9-1.9v10.2" />
        <circle cx="6.9" cy="17.4" r="2.3" />
        <circle cx="15.9" cy="15.4" r="2.3" />
      </svg>
      <h2>曲库为空</h2>
      <p>把音频文件<b>拖进本窗口</b>即可导入,或放入导入文件夹</p>
      <button class="primary-pill" style="margin-top: 8px" title="MP3 / FLAC / M4A / OGG / OPUS / WAV" @click="openImportDir">
        打开导入文件夹
      </button>
      <p class="dir" :title="lib.importDir">{{ lib.importDir }}</p>
      <p class="formats">支持 MP3 / FLAC / M4A / OGG / OPUS / WAV · 歌词支持内嵌或同名 .lrc · 封面自动识别</p>
      <div class="alt">
        <button class="link" @click="addFolder">添加其他文件夹</button>
        <span class="dot">·</span>
        <button class="link" @click="rescan">重新扫描</button>
      </div>
    </div>

    <template v-else>
      <header class="view-head">
        <h1>最近添加</h1>
        <div class="right">
          <span class="count">最新 {{ recent.length }} 张 · 共 {{ albums.length }} 张专辑</span>
          <button v-if="albums.length > recent.length" class="all-btn" @click="go('albums')">
            全部专辑
          </button>
        </div>
      </header>

      <div class="folders">
        <span v-for="f in lib.folders" :key="f" class="chip" :title="f">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round">
            <path d="M3.5 7c0-1.1.9-2 2-2h3.4l2 2.2h7.6c1.1 0 2 .9 2 2v8.3c0 1.1-.9 2-2 2h-13c-1.1 0-2-.9-2-2z" />
          </svg>
          {{ shortPath(f) }}
          <button v-if="f !== lib.importDir" class="x" title="移除该文件夹" @click="removeFolder(f)">×</button>
        </span>
      </div>

      <div class="grid">
        <AlbumCard v-for="a in recent" :key="a.key" :album="a" @open="openAlbum(a.key)" />
      </div>
    </template>
  </div>
</template>

<style scoped>
.right {
  display: flex;
  align-items: center;
  gap: 12px;
}
.all-btn {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text);
  background: var(--pill-bg);
  border-radius: 999px;
  padding: 6px 16px;
  transition: background 0.2s ease, transform 0.35s var(--ease-spring);
}
.all-btn:hover {
  background: var(--hover);
}
.all-btn:active {
  transform: scale(0.97);
  transition-duration: 0.09s;
}

.folders {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 22px;
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: var(--pill-bg);
  color: var(--text-2);
  border-radius: 999px;
  padding: 4px 8px 4px 10px;
  font-size: 12px;
  max-width: 320px;
  white-space: nowrap;
}
.chip .x {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
  line-height: 1;
  color: var(--text-2);
}
.chip .x:hover {
  background: var(--active);
  color: var(--accent);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(168px, 1fr));
  gap: 24px 20px;
}

.load-err {
  color: var(--text-2);
  font-size: 12px;
  max-width: 480px;
  word-break: break-all;
}

.link {
  color: var(--accent);
  font-size: 13px;
  margin-top: 4px;
}
.link:hover {
  text-decoration: underline;
}

.dir {
  font-size: 11.5px;
  color: var(--text-3);
  max-width: 560px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  direction: rtl;
  text-align: center;
}
.formats {
  font-size: 12px;
  color: var(--text-2);
  margin-top: 2px;
}
.alt {
  display: flex;
  align-items: center;
  gap: 8px;
}
.alt .dot {
  color: var(--text-3);
}
</style>
