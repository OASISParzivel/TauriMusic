import { reactive, watchEffect } from "vue";
import { getCurrentWindow, Effect } from "@tauri-apps/api/window";

export type ViewName = "home" | "albums" | "album" | "artists" | "songs" | "search";

interface UiState {
  view: ViewName;
  /** 当前打开的专辑 key */
  albumKey: string | null;
  /** 当前打开的艺人名 */
  artist: string | null;
  /** 搜索关键词 */
  search: string;
  dark: boolean;
  /** 液态玻璃外观(窗口透明 + 亚克力模糊) */
  glass: boolean;
  /** 全屏播放页(含歌词)是否打开 */
  nowPlayingOpen: boolean;
}

export const ui = reactive<UiState>({
  view: "home",
  albumKey: null,
  artist: null,
  search: "",
  dark: false,
  glass: false,
  nowPlayingOpen: false,
});

export function go(view: ViewName): void {
  ui.view = view;
  if (view !== "search") ui.search = "";
}

export function openAlbum(key: string): void {
  ui.albumKey = key;
  ui.view = "album";
}

export function openArtist(name: string): void {
  ui.artist = name;
  ui.view = "artists";
}

export function initTheme(): void {
  const saved = localStorage.getItem("tm-dark");
  ui.dark = saved ? saved === "1" : window.matchMedia("(prefers-color-scheme: dark)").matches;
  watchEffect(() => {
    document.documentElement.classList.toggle("dark", ui.dark);
  });
}

export function toggleDark(): void {
  ui.dark = !ui.dark;
  localStorage.setItem("tm-dark", ui.dark ? "1" : "0");
  if (ui.glass) void applyGlass();
}

/** 玻璃外观:半透明材质 + 系统级 Acrylic 背景模糊 */
export function initGlass(): void {
  ui.glass = localStorage.getItem("tm-glass") !== "0"; // 默认开启
  void applyGlass();
}

export function toggleGlass(): void {
  ui.glass = !ui.glass;
  localStorage.setItem("tm-glass", ui.glass ? "1" : "0");
  void applyGlass();
}

async function applyGlass(): Promise<void> {
  document.documentElement.classList.toggle("glass", ui.glass);
  try {
    const win = getCurrentWindow();
    if (ui.glass) {
      await win.setEffects({
        effects: [Effect.Acrylic],
        color: ui.dark
          ? { red: 26, green: 26, blue: 30, alpha: 128 }
          : { red: 248, green: 248, blue: 252, alpha: 128 },
      });
    } else {
      await win.clearEffects();
    }
  } catch (err) {
    console.warn("窗口特效不可用", err);
  }
}
