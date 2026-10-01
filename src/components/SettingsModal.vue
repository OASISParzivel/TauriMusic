<script setup lang="ts">
import Modal from "./Modal.vue";
import { ui, setThemeMode, toggleGlass } from "../stores/ui";
import { lib, openImportDir, rescan } from "../stores/library";

const themeOptions = [
  { value: "system", label: "跟随系统" },
  { value: "light", label: "浅色" },
  { value: "dark", label: "黑色" },
] as const;

const FORMAT_HINT = "支持 MP3 / FLAC / M4A / OGG / OPUS / WAV · 歌词支持内嵌或同名 .lrc · 封面自动识别";
</script>

<template>
  <Modal @close="ui.settingsOpen = false">
    <header class="head">
      <h2>设置</h2>
      <button class="close" title="关闭" @click="ui.settingsOpen = false">
        <svg viewBox="0 0 24 24"><path d="m6 6 12 12M18 6 6 18" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
      </button>
    </header>

    <div class="sec">皮肤</div>
    <div class="group">
      <button
        v-for="opt in themeOptions"
        :key="opt.value"
        class="opt"
        :class="{ on: ui.themeMode === opt.value }"
        @click="setThemeMode(opt.value)"
      >
        <span>{{ opt.label }}</span>
        <svg v-if="ui.themeMode === opt.value" class="check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
          <path d="m5 12.5 4.5 4.5L19 8" />
        </svg>
      </button>
      <button class="opt" :class="{ on: ui.glass }" @click="toggleGlass">
        <span>液态玻璃</span>
        <svg v-if="ui.glass" class="check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
          <path d="m5 12.5 4.5 4.5L19 8" />
        </svg>
      </button>
    </div>

    <div class="sec">导入</div>
    <div class="group">
      <div class="row">
        <span class="row-label">导入文件夹</span>
        <button class="mini" @click="openImportDir">打开</button>
      </div>
      <p class="dir" :title="lib.importDir">{{ lib.importDir }}</p>
      <p class="hint">{{ FORMAT_HINT }}</p>
    </div>

    <div class="sec">资料库</div>
    <div class="group">
      <div class="row">
        <span class="row-label">{{ lib.tracks.length }} 首歌曲 · {{ lib.folders.length }} 个文件夹</span>
        <button class="mini" :disabled="lib.scanning" @click="rescan">
          {{ lib.scanning ? `扫描中 ${lib.scanCurrent}/${lib.scanTotal}` : "重新扫描" }}
        </button>
      </div>
    </div>
  </Modal>
</template>

<style scoped>
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 14px;
}
.head h2 {
  font-size: 18px;
  font-weight: 700;
}
.close {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-2);
  transition: background 0.2s ease, color 0.2s ease;
}
.close:hover {
  background: var(--hover);
  color: var(--text);
}
.close svg {
  width: 15px;
  height: 15px;
}

.sec {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-3);
  letter-spacing: 0.4px;
  padding: 14px 2px 6px;
}
.group {
  border: 1px solid var(--hairline);
  border-radius: 12px;
  padding: 4px;
}
.opt {
  display: flex;
  align-items: center;
  width: 100%;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 13px;
  color: var(--text-2);
  text-align: left;
  transition: background 0.2s var(--ease-out-soft), color 0.2s ease;
}
.opt:hover {
  background: var(--hover);
  color: var(--text);
}
.opt.on {
  color: var(--text);
}
.opt .check {
  width: 14px;
  height: 14px;
  margin-left: auto;
  flex: none;
  color: var(--text-2);
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 12px;
}
.row-label {
  font-size: 13px;
  color: var(--text);
  min-width: 0;
}
.mini {
  flex: none;
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
  background: var(--pill-bg);
  border-radius: 999px;
  padding: 5px 14px;
  transition: background 0.2s ease, transform 0.35s var(--ease-spring);
}
.mini:hover {
  background: var(--hover);
}
.mini:disabled {
  opacity: 0.5;
  cursor: default;
}

.dir {
  font-size: 11.5px;
  color: var(--text-3);
  padding: 0 12px 2px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  direction: rtl;
  text-align: left;
}
.hint {
  font-size: 12px;
  color: var(--text-2);
  padding: 2px 12px 10px;
  line-height: 1.6;
}
</style>
