import { writable, get } from "svelte/store";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type UpdaterState = {
  checked: boolean;
  checking: boolean;
  installing: boolean;
  hasUpdate: boolean;
  latestVersion: string;
  update: Update | null;
  error: string;
  /** Bytes received by the running install. */
  downloaded: number;
  /** Size of the update package; 0 when the server does not send it. */
  contentLength: number;
};

const initialState: UpdaterState = {
  checked: false,
  checking: false,
  installing: false,
  hasUpdate: false,
  latestVersion: "",
  update: null,
  error: "",
  downloaded: 0,
  contentLength: 0,
};

export const updaterState = writable<UpdaterState>({ ...initialState });

/** Visibility of the update dialog, shared by every entry point. */
export const updateDialogOpen = writable(false);

let checkPromise: Promise<void> | null = null;

const getErrorMessage = (error: unknown, fallback: string): string => {
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  try {
    const text = JSON.stringify(error);
    return text && text !== "{}" ? text : fallback;
  } catch {
    return fallback;
  }
};

export async function ensureUpdateChecked(force = false): Promise<void> {
  if (checkPromise) {
    await checkPromise;
    return;
  }

  const current = get(updaterState);
  if (!force && (current.checked || current.checking)) {
    return;
  }

  const task = (async () => {
    updaterState.update((state) => ({ ...state, checking: true, error: "" }));
    try {
      const update = await check();
      updaterState.update((state) => ({
        ...state,
        checked: true,
        checking: false,
        hasUpdate: Boolean(update),
        latestVersion: update?.version ?? "",
        update: update ?? null,
      }));
    } catch (error) {
      updaterState.update((state) => ({
        ...state,
        checked: true,
        checking: false,
        error: getErrorMessage(error, "Failed to check for update"),
      }));
      throw error;
    }
  })();

  checkPromise = task;
  try {
    await task;
  } finally {
    checkPromise = null;
  }
}

export async function installAvailableUpdate(): Promise<void> {
  const current = get(updaterState);
  if (current.installing) return;

  if (!current.update) {
    await ensureUpdateChecked(true);
  }

  const refreshed = get(updaterState);
  if (!refreshed.update) return;

  updaterState.update((state) => ({
    ...state,
    installing: true,
    error: "",
    downloaded: 0,
    contentLength: 0,
  }));
  try {
    await refreshed.update.downloadAndInstall((event: DownloadEvent) => {
      if (event.event === "Started") {
        const contentLength = event.data.contentLength ?? 0;
        updaterState.update((state) => ({ ...state, contentLength }));
      } else if (event.event === "Progress") {
        const { chunkLength } = event.data;
        updaterState.update((state) => ({ ...state, downloaded: state.downloaded + chunkLength }));
      }
    });
    await relaunch();
  } catch (error) {
    updaterState.update((state) => ({
      ...state,
      installing: false,
      error: getErrorMessage(error, "Failed to install update"),
    }));
    throw error;
  }
}

/** Opens the update dialog and starts a fresh check, like Sparkle's "Check for Updates…". */
export function openUpdateDialog(): void {
  updateDialogOpen.set(true);
  if (get(updaterState).installing) return;
  ensureUpdateChecked(true).catch(console.error);
}
