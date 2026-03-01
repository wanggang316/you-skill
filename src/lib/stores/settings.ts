import { derived, get, writable } from "svelte/store";
import { getSettings as fetchSettings, updateSettings as persistSettings } from "../api";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AppSettings } from "../api/settings";

const defaultSettings: AppSettings = {
  language: "en",
  theme: "system",
  sync_mode: "symlink",
  unknown_skill_install_permission: false,
  openrouter_api_key: null,
  translate_target_language: "",
  translate_model: "",
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

const applyTheme = (theme: AppSettings["theme"]) => {
  applyNativeTheme(theme);
  applyDocumentTheme(theme);
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

export const loadSettings = async () => {
  try {
    const remote = await fetchSettings();
    const merged = {
      ...defaultSettings,
      language: remote.language,
      theme: remote.theme,
      sync_mode: remote.sync_mode,
      unknown_skill_install_permission: remote.unknown_skill_install_permission,
      openrouter_api_key: remote.openrouter_api_key ?? null,
      translate_target_language: remote.translate_target_language ?? "",
      translate_model: remote.translate_model ?? "",
    };
    settings.set(merged);
    applyTheme(merged.theme);
    syncSystemTheme(merged.theme);
  } catch (error) {
    console.error(error);
    settings.set({ ...defaultSettings });
    applyTheme(defaultSettings.theme);
    syncSystemTheme(defaultSettings.theme);
  }
};

export const updateSettings = async (
  patch: Partial<
    Pick<
      AppSettings,
      | "language"
      | "theme"
      | "sync_mode"
      | "unknown_skill_install_permission"
      | "openrouter_api_key"
      | "translate_target_language"
      | "translate_model"
    >
  >
) => {
  const current = get(settings);
  const next = { ...current, ...patch };
  settings.set(next);
  applyTheme(next.theme);
  syncSystemTheme(next.theme);
  try {
    await persistSettings(next);
  } catch (error) {
    console.error(error);
  }
};

export const language = derived(settings, ($settings) => $settings.language);
