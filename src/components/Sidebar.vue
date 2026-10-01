<script setup lang="ts">
import { ui } from "../stores/ui";
</script>

<template>
  <aside class="sidebar">
    <div class="spacer" />
    <button class="rail-btn" title="设置" @click="ui.settingsOpen = true">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round">
        <path d="M5 7.5h7M16.5 7.5H19M5 12h3M11.5 12H19M5 16.5h7M16.5 16.5H19" />
        <circle cx="14" cy="7.5" r="2" />
        <circle cx="9.5" cy="12" r="2" />
        <circle cx="14" cy="16.5" r="2" />
      </svg>
    </button>
    <button class="rail-btn" title="关于 TauriMusic" @click="ui.aboutOpen = true">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round">
        <circle cx="12" cy="12" r="8.2" />
        <path d="M12 11.2v4.6" />
        <circle cx="12" cy="8" r="1" fill="currentColor" stroke="none" />
      </svg>
    </button>
  </aside>
</template>

<style scoped>
.sidebar {
  grid-row: 2;
  grid-column: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  background: var(--bg-2);
  border-right: 1px solid var(--hairline);
  padding: 12px 0;
  gap: 6px;
  min-height: 0;
}
.spacer {
  flex: 1;
}
.rail-btn {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-2);
  transition: background 0.25s var(--ease-out-soft), color 0.2s ease, transform 0.35s var(--ease-spring);
}
.rail-btn:hover {
  background: var(--hover);
  color: var(--text);
}
.rail-btn:active {
  transform: scale(0.9);
  transition-duration: 0.08s;
}
.rail-btn svg {
  width: 20px;
  height: 20px;
}
</style>
