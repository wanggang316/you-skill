import { derived, writable } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import { AUTO_SYNC_EVENT, getAutoSyncStatus, type AutoSyncStatus } from "../api/library-sync";
import { loadAgents, refreshHub } from "./hub";
import { refreshInstructions } from "./instructions";
import { settings } from "./settings";

/** The last library auto sync of this app run; `null` before the first cycle. */
export const autoSyncStatus = writable<AutoSyncStatus | null>(null);

/** Auto sync is on and its last cycle needs the user or failed: the Settings badge. */
export const autoSyncNeedsAttention = derived(
  [settings, autoSyncStatus],
  ([$settings, $status]) =>
    $settings.auto_sync.enabled &&
    Boolean($status?.error || $status?.outcome?.result === "needs_attention")
);

/** Follows the auto sync results; an auto restore reloads the library. Returns the remover. */
export function startAutoSyncStatus(): () => void {
  let stopped = false;
  let unlisten: (() => void) | null = null;
  listen<AutoSyncStatus | null>(AUTO_SYNC_EVENT, ({ payload }) => {
    autoSyncStatus.set(payload);
    if (!payload?.outcome?.restored) return;
    if (payload.outcome.agentsChanged) loadAgents().catch(console.error);
    refreshHub().catch(console.error);
    refreshInstructions().catch(console.error);
  })
    .then((remove) => {
      if (stopped) remove();
      else unlisten = remove;
    })
    .catch(console.error);
  // A cycle at app start can end before the page listens.
  getAutoSyncStatus()
    .then((status) => {
      if (!stopped && status) autoSyncStatus.set(status);
    })
    .catch(console.error);
  return () => {
    stopped = true;
    unlisten?.();
  };
}
