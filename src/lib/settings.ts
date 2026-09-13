import { LazyStore } from "@tauri-apps/plugin-store";
import { DEFAULT_SETTINGS, type Settings } from "./types";

const store = new LazyStore("settings.json");

export async function loadSettings(): Promise<Settings> {
  const saved = (await store.get<Settings>("settings")) ?? {};
  return { ...DEFAULT_SETTINGS, ...saved };
}

export async function saveSettings(settings: Settings): Promise<void> {
  await store.set("settings", settings);
  await store.save();
}
