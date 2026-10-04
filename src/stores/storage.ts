/** localStorage 统一管理:键名集中、防散落 */
const PREFIX = "tm-";

export const StorageKeys = {
  volume: `${PREFIX}vol`,
  welcome: `${PREFIX}welcome`,
  theme: `${PREFIX}theme`,
  glass: `${PREFIX}glass`,
  mode: `${PREFIX}mode`,
} as const;

export function storageGet(key: string): string | null {
  return localStorage.getItem(key);
}

export function storageGetString(key: string, fallback: string): string {
  return localStorage.getItem(key) ?? fallback;
}

export function storageSet(key: string, value: string): void {
  localStorage.setItem(key, value);
}

export function storageRemove(key: string): void {
  localStorage.removeItem(key);
}
