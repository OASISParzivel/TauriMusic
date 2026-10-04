<script setup lang="ts">
import { ref } from "vue";
import Modal from "./Modal.vue";
import { lib } from "../stores/library";
import {
  addToPlaylist,
  closePlaylistPicker,
  createAndAdd,
  picker,
} from "../stores/playlists";

const newName = ref("");
/** 只显示条目数量,便于选择 */
const countOf = (id: string): number => lib.playlists.find((p) => p.id === id)?.entries.length ?? 0;

async function pick(id: string): Promise<void> {
  await addToPlaylist(id);
}

async function createNew(): Promise<void> {
  await createAndAdd(newName.value);
  newName.value = "";
}
</script>

<template>
  <Modal @close="closePlaylistPicker">
    <header class="head">
      <h2>添加到播放列表</h2>
      <button class="close" title="关闭" @click="closePlaylistPicker">
        <svg viewBox="0 0 24 24"><path d="m6 6 12 12M18 6 6 18" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
      </button>
    </header>
    <p class="sub">共 {{ picker.trackIds?.length ?? 0 }} 首</p>

    <div v-if="lib.playlists.length" class="pl-list">
      <button v-for="p in lib.playlists" :key="p.id" class="pl-row" @click="pick(p.id)">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round">
          <path d="M4 6.5h11M4 12h11M4 17.5h7" />
          <circle cx="18.5" cy="15.5" r="2.2" />
          <path d="M20.7 15.5V9l-3.4.9" />
        </svg>
        <span class="pl-name">{{ p.name }}</span>
        <span class="pl-count">{{ countOf(p.id) }} 首</span>
      </button>
    </div>
    <p v-else class="empty">还没有播放列表,先创建一个吧</p>

    <div class="new-row">
      <input
        v-model="newName"
        class="new-input"
        type="text"
        maxlength="60"
        placeholder="新建播放列表并添加"
        spellcheck="false"
        @keydown.enter.prevent="createNew"
      />
      <button class="mini primary" :disabled="!newName.trim()" @click="createNew">新建</button>
    </div>
  </Modal>
</template>

<style scoped>
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
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
.sub {
  font-size: 12px;
  color: var(--text-2);
  margin: 4px 0 12px;
}

.pl-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 300px;
  overflow-y: auto;
}
.pl-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  border-radius: 9px;
  font-size: 13.5px;
  color: var(--text);
  text-align: left;
  transition: background 0.2s ease;
}
.pl-row:hover {
  background: var(--hover);
}
.pl-row .ico {
  width: 17px;
  height: 17px;
  flex: none;
  color: var(--text-2);
}
.pl-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.pl-count {
  flex: none;
  font-size: 11.5px;
  color: var(--text-3);
}
.empty {
  font-size: 13px;
  color: var(--text-2);
  padding: 8px 2px 12px;
}

.new-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid var(--hairline);
}
.new-input {
  flex: 1;
  min-width: 0;
  border: 1px solid var(--hairline);
  background: var(--pill-bg);
  border-radius: 10px;
  padding: 8px 12px;
  font-size: 13px;
  color: var(--text);
  outline: none;
}
.new-input:focus {
  border-color: var(--accent);
}
.mini {
  flex: none;
  font-size: 13px;
  font-weight: 600;
  color: #fff;
  background: var(--accent);
  border-radius: 999px;
  padding: 8px 18px;
  transition: background 0.2s ease;
}
.mini.primary:hover:not(:disabled) {
  background: var(--accent-hover);
}
.mini.primary:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
