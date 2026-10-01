<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";

const emit = defineEmits<{ close: [] }>();

function onKey(e: KeyboardEvent): void {
  if (e.key === "Escape") emit("close");
}
onMounted(() => window.addEventListener("keydown", onKey));
onUnmounted(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="modal">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: absolute;
  inset: 0;
  z-index: 300;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
}
.modal {
  width: 460px;
  max-width: 88%;
  max-height: 82%;
  overflow-y: auto;
  background: var(--bg);
  border: 1px solid var(--hairline);
  border-radius: 16px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.4);
  padding: 20px 22px;
}
/* 玻璃模式下弹窗也是磨砂卡片 */
html.glass:not(.dark) .modal {
  background: rgba(255, 255, 255, 0.78);
  backdrop-filter: blur(30px) saturate(1.6);
}
html.dark.glass .modal {
  background: rgba(28, 28, 34, 0.72);
  backdrop-filter: blur(30px) saturate(1.6);
}
</style>
