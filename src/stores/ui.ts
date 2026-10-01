import { reactive, watchEffect } from "vue";

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
  /** 全屏播放页(含歌词)是否打开 */
  nowPlayingOpen: boolean;
}

export const ui = reactive<UiState>({
  view: "home",
  albumKey: null,
  artist: null,
  search: "",
  dark: false,
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
}
