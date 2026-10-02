<script setup lang="ts">
import { computed } from "vue";
import { ui, go } from "../stores/ui";
import { lib } from "../stores/library";

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

const statusText = computed(() =>
  lib.scanning
    ? `正在扫描 ${lib.scanTotal > 0 ? `${lib.scanCurrent}/${lib.scanTotal}` : "…"}`
    : lib.importStatus,
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

    <div v-if="statusText" class="status">{{ statusText }}</div>

    <div class="actions">
      <button class="action" title="设置" @click="ui.settingsOpen = true">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round">
          <path d="M5 7.5h7M16.5 7.5H19M5 12h3M11.5 12H19M5 16.5h7M16.5 16.5H19" />
          <circle cx="14" cy="7.5" r="2" />
          <circle cx="9.5" cy="12" r="2" />
          <circle cx="14" cy="16.5" r="2" />
        </svg>
        <span>设置</span>
      </button>
      <button class="action" title="关于 TauriMusic" @click="ui.aboutOpen = true">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round">
          <circle cx="12" cy="12" r="8.2" />
          <path d="M12 11.2v4.6" />
          <circle cx="12" cy="8" r="1" fill="currentColor" stroke="none" />
        </svg>
        <span>关于</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  grid-row: 2;
  grid-column: 1;
  display: flex;
  flex-direction: column;
  background: transparent; /* 与顶栏共用 shell 的连续铬底 */
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
  color: var(--text);
}
.nav-item .ico {
  width: 18px;
  height: 18px;
  flex: none;
  color: var(--text-2);
}
.nav-item.active .ico {
  color: var(--text);
}
/* 玻璃皮肤下恢复红色图标点缀 */
html.glass .nav-item .ico {
  color: var(--accent);
}
html.glass .nav-item.active {
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
