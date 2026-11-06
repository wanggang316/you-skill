import { derived, writable, type Writable } from "svelte/store";
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

type Unlisten = () => void;

export type AutoSyncStatusDeps = {
  status: Writable<AutoSyncStatus | null>;
  /** Registers the event listener and resolves to its remover. */
  listen: (handler: (status: AutoSyncStatus | null) => void) => Promise<Unlisten>;
  getStatus: () => Promise<AutoSyncStatus | null>;
  /** Called after an auto restore that wrote. */
  onRestored: (agentsChanged: boolean) => void;
};

const defaultDeps: AutoSyncStatusDeps = {
  status: autoSyncStatus,
  listen: (handler) =>
    listen<AutoSyncStatus | null>(AUTO_SYNC_EVENT, ({ payload }) => handler(payload)),
  getStatus: getAutoSyncStatus,
  onRestored: (agentsChanged) => {
    if (agentsChanged) loadAgents().catch(console.error);
    refreshHub().catch(console.error);
    refreshInstructions().catch(console.error);
  },
};

/**
 * Follows the auto sync results; an auto restore reloads the library. The listener is
 * registered before the last status is read, and that read is used only when no event came
 * in meanwhile, so a cycle that ends at app start is never lost or replaced by an older
 * value. Returns the remover.
 */
export function startAutoSyncStatus(deps: AutoSyncStatusDeps = defaultDeps): Unlisten {
  let stopped = false;
  let eventSeen = false;
  let unlisten: Unlisten | null = null;
  deps
    .listen((status) => {
      if (stopped) return;
      eventSeen = true;
      deps.status.set(status);
      if (status?.outcome?.restored) deps.onRestored(status.outcome.agentsChanged);
    })
    .then((remove) => {
      if (stopped) {
        remove();
        return;
      }
      unlisten = remove;
      return deps.getStatus().then((status) => {
        if (!stopped && !eventSeen) deps.status.set(status);
      });
    })
    .catch(console.error);
  return () => {
    stopped = true;
    unlisten?.();
  };
}
