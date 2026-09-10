import { homeDir } from "@tauri-apps/api/path";
import { writable } from "svelte/store";

/** The user's home directory without a trailing separator; empty until loaded. */
export const homePath = writable("");

export async function loadHomePath(): Promise<void> {
  const path = await homeDir();
  homePath.set(path.replace(/[/\\]+$/, ""));
}
