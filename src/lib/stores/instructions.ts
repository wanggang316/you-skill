import { derived, writable } from "svelte/store";
import {
  listInstructionFiles,
  listInstructions,
  type AgentFileView,
  type InstructionView,
} from "../api/instructions";

/** Templates in the library. */
export const instructions = writable<InstructionView[]>([]);
/** Instruction files that exist at agent locations. */
export const instructionFiles = writable<AgentFileView[]>([]);
export const instructionsLoading = writable(false);
export const instructionsError = writable("");
export const instructionsLoaded = writable(false);

export const instructionsById = derived(
  instructions,
  ($items) => new Map($items.map((item) => [item.id, item]))
);

/** Display name of a template, for dialogs that only hold the id. */
export const instructionName = derived(
  instructionsById,
  ($byId) => (id: string) => $byId.get(id)?.name ?? id
);

let refreshPromise: Promise<void> | null = null;

/** Reload templates and agent files together; both change with almost every action. */
export async function refreshInstructions(): Promise<void> {
  if (refreshPromise) return refreshPromise;
  instructionsLoading.set(true);
  instructionsError.set("");
  refreshPromise = (async () => {
    try {
      const [items, files] = await Promise.all([listInstructions(), listInstructionFiles()]);
      items.sort((a, b) => a.name.localeCompare(b.name) || a.id.localeCompare(b.id));
      instructions.set(items);
      instructionFiles.set(files);
      instructionsLoaded.set(true);
    } catch (error) {
      instructionsError.set(String(error));
    } finally {
      instructionsLoading.set(false);
      refreshPromise = null;
    }
  })();
  return refreshPromise;
}

export function applyInstructionView(view: InstructionView | null | undefined): void {
  if (!view) return;
  instructions.update((current) => {
    const index = current.findIndex((item) => item.id === view.id);
    if (index === -1) {
      return [...current, view].sort((a, b) => a.name.localeCompare(b.name));
    }
    const next = [...current];
    next[index] = view;
    return next;
  });
}
