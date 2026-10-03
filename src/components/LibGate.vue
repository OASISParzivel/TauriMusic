<script setup lang="ts">
import { lib, reloadLibrary } from "../stores/library";
</script>

<template>
  <!-- 曲库加载守卫:未加载完/读取失败时占位,成功才渲染插槽内容 -->
  <div v-if="!lib.loaded" class="empty">
    <h2>正在加载曲库…</h2>
  </div>
  <div v-else-if="lib.loadError" class="empty">
    <h2>曲库读取失败</h2>
    <p class="err">{{ lib.loadError }}</p>
    <button class="primary-pill" @click="reloadLibrary">重试</button>
  </div>
  <slot v-else />
</template>

<style scoped>
.err {
  color: var(--text-2);
  font-size: 12px;
  max-width: 480px;
  word-break: break-all;
}
</style>
