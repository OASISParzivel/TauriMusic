<script setup lang="ts">
import { MODE_LABEL, player, type PlayMode } from "../stores/player";

function cycle(): void {
  const order: PlayMode[] = ["seq", "loop", "one", "shuffle"];
  const i = order.indexOf(player.mode);
  player.mode = order[(i + 1) % order.length];
}

type Mode = PlayMode;
const icons: Record<Mode, string[]> = {
  seq: ["M4 12h11", "M12 8l4 4-4 4", "M20 5v14"],
  loop: [
    "m17 2.5 3.5 3.5-3.5 3.5",
    "M3.5 11.5v-1a4.5 4.5 0 0 1 4.5-4.5h12",
    "m7 21.5-3.5-3.5L7 14.5",
    "M20.5 12.5v1a4.5 4.5 0 0 1-4.5 4.5H4",
  ],
  one: [
    "m17 2.5 3.5 3.5-3.5 3.5",
    "M3.5 11.5v-1a4.5 4.5 0 0 1 4.5-4.5h12",
    "m7 21.5-3.5-3.5L7 14.5",
    "M20.5 12.5v1a4.5 4.5 0 0 1-4.5 4.5H4",
  ],
  shuffle: [
    "M2.5 18h1.6c1.3 0 2.6-.6 3.4-1.7l6-8.6c.8-1.1 2.1-1.7 3.4-1.7h3.1",
    "m18.5 2.5 3 3.5-3 3.5",
    "M2.5 6h1.6c1.5 0 2.9.9 3.7 2.2",
    "M20 18h-4.1c-1.3 0-2.5-.7-3.3-1.8l-.6-.8",
    "m18.5 14.5 3 3.5-3 3.5",
  ],
};
</script>

<template>
  <!-- 播放模式单键:顺序 → 列表循环 → 单曲循环 → 随机,四态互斥循环切换 -->
  <button
    class="mode-btn"
    :class="{ on: player.mode !== 'seq' }"
    :title="MODE_LABEL[player.mode]"
    @click="cycle()"
  >
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <path v-for="d in icons[player.mode]" :key="d" :d="d" />
      <path v-if="player.mode === 'one'" d="M10.5 9.5 12 8.6V15" stroke-width="2" />
    </svg>
    <span v-if="player.mode === 'one'" class="badge">1</span>
  </button>
</template>

<style scoped>
.mode-btn {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  color: inherit;
}
.mode-btn.on {
  color: var(--accent);
}
.mode-btn.on svg {
  color: inherit;
}
.badge {
  position: absolute;
  top: 0;
  right: -1px;
  font-size: 8px;
  font-weight: 700;
  color: var(--accent);
}
</style>
