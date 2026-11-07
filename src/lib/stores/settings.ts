import { derived, get, writable } from "svelte/store";
import { getSettings as fetchSettings, updateSettings as persistSettings } from "../api";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { DEFAULT_AUTO_SYNC, type AppSettings, type EditableSettings } from "../api/settings";
import { createLatestWriter } from "../utils/latest-writer";

const defaultSettings: AppSettings = {
  language: "en",
  theme: "system",
  sync_mode: "copy",
  openrouter_api_key: null,
  translate_target_language: "",
  translate_model: "",
  translate_display_mode: "bilingual",
  translate_text_style: "none",
  shortcuts: {},
  auto_sync: DEFAULT_AUTO_SYNC,
};

export const settings = writable<AppSettings>({ ...defaultSettings });

let systemMedia: MediaQueryList | null = null;
let systemListener: ((event: MediaQueryListEvent) => void) | null = null;
let systemThemeUnlisten: (() => void) | null = null;
let systemSyncToken = 0;

const applyDocumentTheme = (theme: AppSettings["theme"]) => {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  if (theme === "dark") {
    root.dataset.theme = "dark";
    return;
  }
  if (theme === "light") {
    delete root.dataset.theme;
    return;
  }
  const prefersDark =
    typeof window !== "undefined" &&
    window.matchMedia &&
    window.matchMedia("(prefers-color-scheme: dark)").matches;
  if (prefersDark) {
    root.dataset.theme = "dark";
  } else {
    delete root.dataset.theme;
  }
};

const applyNativeTheme = (theme: AppSettings["theme"]) => {
  if (typeof window === "undefined") return;
  const nativeTheme = theme === "system" ? null : theme;
  getCurrentWindow()
    .setTheme(nativeTheme)
    .catch((error) => {
      // Non-tauri environment (e.g. web preview) can fail here, safe to ignore.
      console.debug("Failed to apply native app theme:", error);
    });
};

/** Read by the inline script in app.html to theme the first paint before settings load. */
const THEME_STORAGE_KEY = "youskill.theme";

const cacheTheme = (theme: AppSettings["theme"]) => {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // Caching the theme is best effort.
  }
};

const applyTheme = (theme: AppSettings["theme"]) => {
  applyNativeTheme(theme);
  applyDocumentTheme(theme);
  cacheTheme(theme);
};

const syncSystemTheme = (theme: AppSettings["theme"]) => {
  const syncToken = ++systemSyncToken;

  if (systemMedia && systemListener) {
    systemMedia.removeEventListener("change", systemListener);
    systemMedia = null;
    systemListener = null;
  }
  if (systemThemeUnlisten) {
    systemThemeUnlisten();
    systemThemeUnlisten = null;
  }

  if (typeof window === "undefined" || !window.matchMedia) return;
  if (theme !== "system") return;

  systemMedia = window.matchMedia("(prefers-color-scheme: dark)");
  systemListener = (event) => {
    applyDocumentTheme(event.matches ? "dark" : "light");
  };
  systemMedia.addEventListener("change", systemListener);

  getCurrentWindow()
    .onThemeChanged(({ payload }) => {
      applyDocumentTheme(payload);
    })
    .then((unlisten) => {
      if (syncToken !== systemSyncToken) {
        unlisten();
        return;
      }
      systemThemeUnlisten = unlisten;
    })
    .catch((error) => {
      console.debug("Failed to listen for native theme change:", error);
    });
};

const fromStored = (remote: AppSettings): AppSettings => ({
  ...defaultSettings,
  language: remote.language,
  theme: remote.theme,
  sync_mode: remote.sync_mode,
  openrouter_api_key: remote.openrouter_api_key ?? null,
  translate_target_language: remote.translate_target_language ?? "",
  translate_model: remote.translate_model ?? "",
  translate_display_mode: remote.translate_display_mode ?? defaultSettings.translate_display_mode,
  translate_text_style: remote.translate_text_style ?? defaultSettings.translate_text_style,
  shortcuts: remote.shortcuts ?? {},
  auto_sync: remote.auto_sync ?? DEFAULT_AUTO_SYNC,
});

const showSettings = (next: AppSettings) => {
  settings.set(next);
  applyTheme(next.theme);
  syncSystemTheme(next.theme);
};

export const loadSettings = async () => {
  try {
    showSettings(fromStored(await fetchSettings()));
  } catch (error) {
    console.error(error);
    settings.set({ ...defaultSettings });
    applyTheme(defaultSettings.theme);
    syncSystemTheme(defaultSettings.theme);
  }
};

/** Each save sends the whole settings; one at a time, so the last change is the one stored. */
const saveSettings = createLatestWriter<AppSettings>(persistSettings);

/**
 * Shows the change at once and saves it. A failed save rejects; when no later change came
 * before or during the read of the stored settings, the settings go back to what is stored,
 * so the UI does not show a value that was not saved.
 */
export const updateSettings = async (patch: Partial<EditableSettings>) => {
  const next = { ...get(settings), ...patch };
  showSettings(next);
  try {
    await saveSettings(next);
  } catch (error) {
    if (get(settings) === next) {
      try {
        const stored = fromStored(await fetchSettings());
        // A change made during the read is newer than what it read; keep that change.
        if (get(settings) === next) showSettings(stored);
      } catch (reloadError) {
        console.error(reloadError);
      }
    }
    throw error;
  }
};

export const language = derived(settings, ($settings) => $settings.language);
