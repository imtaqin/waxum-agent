import { LazyStore } from "@tauri-apps/plugin-store";
import { DEFAULT_SETTINGS, type Settings } from "./types";

const store = new LazyStore("settings.json");

/** Local-only prefill, never committed (see src/lib/devDefaults.local.ts,
 * gitignored) — absent on a fresh clone, in which case this is a no-op. */
async function loadDevDefaults(): Promise<Partial<Settings>> {
  try {
    const mod = await import("./devDefaults.local");
    return mod.devDefaults;
  } catch {
    return {};
  }
}

export async function loadSettings(): Promise<Settings> {
  const [devDefaults, saved] = await Promise.all([
    loadDevDefaults(),
    store.get<Settings>("settings"),
  ]);
  return { ...DEFAULT_SETTINGS, ...devDefaults, ...(saved ?? {}) };
}

export async function saveSettings(settings: Settings): Promise<void> {
  await store.set("settings", settings);
  await store.save();
}
