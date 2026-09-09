import { derived, writable } from "svelte/store";
import { listInstructions, type InstructionView } from "../api/instructions";

export const instructions = writable<InstructionView[]>([]);
export const instructionsLoading = writable(false);
export const instructionsError = writable("");
export const instructionsLoaded = writable(false);

export const instructionsByName = derived(
  instructions,
  ($items) => new Map($items.map((item) => [item.name, item]))
);

let refreshPromise: Promise<void> | null = null;

export async function refreshInstructions(): Promise<void> {
  if (refreshPromise) return refreshPromise;
  instructionsLoading.set(true);
  instructionsError.set("");
  refreshPromise = (async () => {
    try {
      const items = await listInstructions();
      items.sort((a, b) => a.name.localeCompare(b.name));
      instructions.set(items);
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
    const index = current.findIndex((item) => item.name === view.name);
    if (index === -1) {
      return [...current, view].sort((a, b) => a.name.localeCompare(b.name));
    }
    const next = [...current];
    next[index] = view;
    return next;
  });
}
