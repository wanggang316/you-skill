import { derived, get, writable } from 'svelte/store'
import { getSettings as fetchSettings, updateSettings as persistSettings } from '../api'

export type ThemeMode = 'system' | 'light' | 'dark'
export type LanguageMode = 'en' | 'zh'
export type SyncMode = 'symlink' | 'copy'

export interface AppSettings {
  language: LanguageMode
  theme: ThemeMode
  syncMode: SyncMode
}

const defaultSettings: AppSettings = {
  language: 'en',
  theme: 'system',
  syncMode: 'symlink'
}

export const settings = writable<AppSettings>({ ...defaultSettings })

let systemMedia: MediaQueryList | null = null
let systemListener: ((event: MediaQueryListEvent) => void) | null = null

const applyTheme = (theme: ThemeMode) => {
  if (typeof document === 'undefined') return
  const root = document.documentElement
  if (theme === 'dark') {
    root.dataset.theme = 'dark'
    return
  }
  if (theme === 'light') {
    delete root.dataset.theme
    return
  }
  const prefersDark =
    typeof window !== 'undefined' &&
    window.matchMedia &&
    window.matchMedia('(prefers-color-scheme: dark)').matches
  if (prefersDark) {
    root.dataset.theme = 'dark'
  } else {
    delete root.dataset.theme
  }
}

const syncSystemTheme = (theme: ThemeMode) => {
  if (typeof window === 'undefined' || !window.matchMedia) return
  if (systemMedia && systemListener) {
    systemMedia.removeEventListener('change', systemListener)
    systemMedia = null
    systemListener = null
  }
  if (theme !== 'system') return
  systemMedia = window.matchMedia('(prefers-color-scheme: dark)')
  systemListener = (event) => {
    applyTheme(event.matches ? 'dark' : 'light')
  }
  systemMedia.addEventListener('change', systemListener)
}

const fromApi = (remote: {
  language: LanguageMode
  theme: ThemeMode
  sync_mode: SyncMode
}): AppSettings => ({
  language: remote.language,
  theme: remote.theme,
  syncMode: remote.sync_mode
})

const toApi = (value: AppSettings) => ({
  language: value.language,
  theme: value.theme,
  sync_mode: value.syncMode
})

export const loadSettings = async () => {
  try {
    const remote = await fetchSettings()
    const merged = { ...defaultSettings, ...fromApi(remote) }
    settings.set(merged)
    applyTheme(merged.theme)
    syncSystemTheme(merged.theme)
  } catch (error) {
    console.error(error)
    settings.set({ ...defaultSettings })
    applyTheme(defaultSettings.theme)
    syncSystemTheme(defaultSettings.theme)
  }
}

export const updateSettings = async (patch: Partial<AppSettings>) => {
  const current = get(settings)
  const next = { ...current, ...patch }
  settings.set(next)
  applyTheme(next.theme)
  syncSystemTheme(next.theme)
  try {
    await persistSettings(toApi(next))
  } catch (error) {
    console.error(error)
  }
}

export const language = derived(settings, ($settings) => $settings.language)
