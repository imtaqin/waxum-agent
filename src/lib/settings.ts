import { LazyStore } from "@tauri-apps/plugin-store";
import { DEFAULT_SETTINGS, type Settings } from "./types";

const store = new LazyStore("settings.json");

/** Local-only prefill, never committed (see src/lib/devDefaults.local.ts,
 * gitignored) — absent on a fresh clone/CI, in which case this is a
 * no-op. `import.meta.glob` (rather than a literal dynamic `import()`)
 * is what makes that safe: it's resolved at build time to whatever
 * matching files actually exist, so a fresh clone with zero matches
 * type-checks and builds cleanly instead of failing to resolve a module
 * that isn't there. */
const devDefaultsModules = import.meta.glob<{ devDefaults: Partial<Settings> }>(
  "./devDefaults.local.ts",
);

async function loadDevDefaults(): Promise<Partial<Settings>> {
  const loader = devDefaultsModules["./devDefaults.local.ts"];
  if (!loader) return {};
  try {
    const mod = await loader();
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
