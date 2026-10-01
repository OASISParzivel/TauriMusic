import { ref, reactive, watchEffect } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type ViewName = "home" | "albums" | "album" | "artists" | "songs" | "search";
/** 皮肤模式:跟随系统 / 浅色 / 黑色 */
export type ThemeMode = "system" | "light" | "dark";

interface UiState {
  view: ViewName;
  /** 当前打开的专辑 key */
  albumKey: string | null;
  /** 当前打开的艺人名 */
  artist: string | null;
  /** 搜索关键词 */
  search: string;
  /** 解析后的深色状态(供玻璃令牌等使用) */
  dark: boolean;
  /** 皮肤模式选择 */
  themeMode: ThemeMode;
  /** 液态玻璃外观(应用内氛围背景 + 玻璃卡片) */
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
  themeMode: "system",
  glass: false,
  nowPlayingOpen: false,
});

const systemDark = ref(false);

function prefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

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

export function setThemeMode(mode: ThemeMode): void {
  ui.themeMode = mode;
  localStorage.setItem("tm-theme", mode);
}

export function initTheme(): void {
  const saved = localStorage.getItem("tm-theme");
  if (saved === "light" || saved === "dark" || saved === "system") {
    ui.themeMode = saved;
  } else {
    // 迁移旧版 tm-dark 记忆
    const legacy = localStorage.getItem("tm-dark");
    ui.themeMode = legacy == null ? "system" : legacy === "1" ? "dark" : "light";
    localStorage.removeItem("tm-dark");
  }

  systemDark.value = prefersDark();
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    systemDark.value = prefersDark();
  });

  watchEffect(() => {
    ui.dark = ui.themeMode === "dark" || (ui.themeMode === "system" && systemDark.value);
    document.documentElement.classList.toggle("dark", ui.dark);
    // 原生窗口边框/标题区颜色同步;"跟随系统"交还给系统
    try {
      void getCurrentWindow().setTheme(ui.themeMode === "system" ? null : ui.themeMode);
    } catch {
      /* 浏览器调试环境无窗口 */
    }
  });
}

/** 液态玻璃:封面驱动的氛围背景 + 悬浮玻璃卡片(折射应用自身内容) */
export function initGlass(): void {
  ui.glass = localStorage.getItem("tm-glass") !== "0"; // 默认开启
  watchEffect(() => {
    document.documentElement.classList.toggle("glass", ui.glass);
  });
}

export function toggleGlass(): void {
  ui.glass = !ui.glass;
  localStorage.setItem("tm-glass", ui.glass ? "1" : "0");
}
