<script setup lang="ts">
import { computed } from "vue";
import { ui, go, toggleDark, toggleGlass } from "../stores/ui";
import { lib, addFolder, rescan } from "../stores/library";

const navItems = [
  { name: "home", label: "最近添加", icon: "clock" },
  { name: "albums", label: "专辑", icon: "disc" },
  { name: "artists", label: "艺人", icon: "artist" },
  { name: "songs", label: "歌曲", icon: "note" },
] as const;

function isActive(name: string): boolean {
  if (name === "albums") return ui.view === "albums" || ui.view === "album";
  if (name === "artists") return ui.view === "artists";
  return ui.view === name;
}

function onSearchInput(): void {
  if (ui.search.trim()) ui.view = "search";
  else if (ui.view === "search") ui.view = "home";
}

const progressText = computed(() =>
  lib.scanTotal > 0 ? `正在扫描 ${lib.scanCurrent}/${lib.scanTotal}` : "正在扫描…",
);
</script>

<template>
  <aside class="sidebar">
    <div class="search">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
        <circle cx="11" cy="11" r="6.5" />
        <path d="m16.2 16.2 4.3 4.3" />
      </svg>
      <input v-model="ui.search" type="text" placeholder="搜索" spellcheck="false" @input="onSearchInput" />
    </div>

    <div class="section-label">资料库</div>
    <nav>
      <button
        v-for="item in navItems"
        :key="item.name"
        class="nav-item"
        :class="{ active: isActive(item.name) }"
        @click="go(item.name)"
      >
        <svg v-if="item.icon === 'clock'" class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
          <circle cx="12" cy="12" r="8.2" />
          <path d="M12 7.8v4.4l2.8 1.6" />
        </svg>
        <svg v-else-if="item.icon === 'disc'" class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
          <circle cx="12" cy="12" r="8.2" />
          <circle cx="12" cy="12" r="2.1" fill="currentColor" stroke="none" />
        </svg>
        <svg v-else-if="item.icon === 'artist'" class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
          <circle cx="12" cy="8.4" r="3.4" />
          <path d="M5.6 19c.8-3.3 3.3-4.9 6.4-4.9s5.6 1.6 6.4 4.9" />
        </svg>
        <svg v-else class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9.2 17.3V7l9-1.9v10.2" />
          <circle cx="6.9" cy="17.4" r="2.3" />
          <circle cx="15.9" cy="15.4" r="2.3" />
        </svg>
        <span>{{ item.label }}</span>
      </button>
    </nav>

    <div class="spacer" />

    <div v-if="lib.scanning" class="status">{{ progressText }}</div>

    <div class="actions">
      <button class="action" title="添加音乐文件夹" @click="addFolder">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3.5 7c0-1.1.9-2 2-2h3.4l2 2.2h7.6c1.1 0 2 .9 2 2v8.3c0 1.1-.9 2-2 2h-13c-1.1 0-2-.9-2-2z" />
          <path d="M12 11.4v4.4M9.8 13.6h4.4" />
        </svg>
        <span>添加音乐文件夹</span>
      </button>
      <button class="action" title="重新扫描" @click="rescan">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M20 12a8 8 0 1 1-2.4-5.7" />
          <path d="M20 4.5V9h-4.5" />
        </svg>
        <span>重新扫描</span>
      </button>
      <button class="action" :title="ui.glass ? '关闭液态玻璃' : '开启液态玻璃'" @click="toggleGlass">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round">
          <path d="M12 3.5c3.2 3.7 5.5 6.7 5.5 9.5a5.5 5.5 0 1 1-11 0c0-2.8 2.3-5.8 5.5-9.5z" />
          <path d="M9.5 13.5a2.6 2.6 0 0 0 2 2.6" stroke-linecap="round" />
        </svg>
        <span>{{ ui.glass ? "原生外观" : "液态玻璃" }}</span>
      </button>
      <button v-if="!ui.glass" class="action" :title="ui.dark ? '切换为浅色' : '切换为深色'" @click="toggleDark">
        <svg v-if="!ui.dark" class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round">
          <path d="M20 13.6A8 8 0 0 1 10.4 4 8 8 0 1 0 20 13.6z" />
        </svg>
        <svg v-else class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
          <circle cx="12" cy="12" r="4" />
          <path d="M12 3.5v2M12 18.5v2M3.5 12h2M18.5 12h2M6 6l1.4 1.4M16.6 16.6 18 18M18 6l-1.4 1.4M7.4 16.6 6 18" />
        </svg>
        <span>{{ ui.dark ? "浅色外观" : "深色外观" }}</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  grid-row: 1 / 3;
  display: flex;
  flex-direction: column;
  background: var(--bg-2);
  border-right: 1px solid var(--hairline);
  padding: 14px 12px 14px;
  gap: 4px;
  min-height: 0;
}

.search {
  display: flex;
  align-items: center;
  gap: 7px;
  background: var(--pill-bg);
  border-radius: 8px;
  padding: 6px 10px;
  color: var(--text-2);
  margin-bottom: 14px;
  flex: none;
}
.search svg {
  width: 15px;
  height: 15px;
  flex: none;
}
.search input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  color: var(--text);
  font-size: 13px;
}
.search input::placeholder {
  color: var(--text-3);
}

.section-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-3);
  padding: 0 10px 6px;
  letter-spacing: 0.4px;
  flex: none;
}

nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: none;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  border-radius: 7px;
  font-size: 13.5px;
  font-weight: 500;
  color: var(--text);
  text-align: left;
  transition: background 0.25s var(--ease-out-soft), transform 0.35s var(--ease-spring);
}
.nav-item:hover {
  background: var(--hover);
}
.nav-item:active {
  transform: scale(0.98);
  transition-duration: 0.1s;
}
.nav-item.active {
  background: var(--active);
  color: var(--accent);
}
.nav-item .ico {
  width: 18px;
  height: 18px;
  flex: none;
  color: var(--accent);
}

.spacer {
  flex: 1;
}

.status {
  font-size: 12px;
  color: var(--text-2);
  padding: 0 10px 8px;
  flex: none;
}

.actions {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: none;
}
.action {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  border-radius: 7px;
  font-size: 13px;
  color: var(--text-2);
  text-align: left;
  transition: background 0.25s var(--ease-out-soft), color 0.2s ease, transform 0.35s var(--ease-spring);
}
.action:hover {
  background: var(--hover);
  color: var(--text);
}
.action:active {
  transform: scale(0.97);
  transition-duration: 0.1s;
}
.action .ico {
  width: 17px;
  height: 17px;
  flex: none;
}
</style>
