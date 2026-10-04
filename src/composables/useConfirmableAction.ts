import { type Ref, ref } from "vue";

/**
 * 两段式删除确认:第一次调用进入确认态,`timeout` 毫秒内再次调用才执行。
 * 超时自动复位,防误删。TrackList 单曲/批量与专辑删除共用此逻辑。
 */
export function useConfirmableAction(timeout = 3000): {
  confirmingId: Ref<string | null>;
  confirm: (key: string | null) => boolean;
  reset: () => void;
} {
  const confirmingId = ref<string | null>(null);
  let timer = 0;

  /** 返回 true 表示本次调用应执行删除;否则已进入确认态 */
  function confirm(key: string | null): boolean {
    if (confirmingId.value !== key) {
      confirmingId.value = key;
      window.clearTimeout(timer);
      timer = window.setTimeout(() => (confirmingId.value = null), timeout);
      return false;
    }
    confirmingId.value = null;
    return true;
  }

  function reset(): void {
    confirmingId.value = null;
    window.clearTimeout(timer);
  }

  return { confirmingId, confirm, reset };
}
