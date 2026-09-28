import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";

export interface LaunchInfo {
  base_url: string;
  token: string;
  binary_path: string;
  port: number;
}

/** Child processes can't be launched on Android/iOS, so bundled mode is
 * desktop-only. */
export const isMobile = /Android|iPhone|iPad|iPod/i.test(navigator.userAgent);

/** Starts the in-app waxum (downloading it on first use) or returns the
 * one already running, and copies its URL and auto-generated token into
 * `s`. The token lives in the app data dir, so the user never types it. */
export async function launchBundled(s: Settings): Promise<LaunchInfo> {
  const info = await invoke<LaunchInfo>("bundled_launch", {
    binaryPath: s.bundledBinaryPath.trim() || null,
  });
  s.baseUrl = info.base_url;
  s.token = info.token;
  return info;
}
