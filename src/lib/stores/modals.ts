import { writable } from "svelte/store";
import type { ActionResult, InstallScope } from "../api/hub";
import { applySkillView } from "./hub";

export type ImportTab = "github" | "zip" | "folder";

export interface ImportModalState {
  open: boolean;
  initialTab: ImportTab;
  /** Pre-selected folder for the scan tab. */
  initialFolder: string | null;
}

export interface InstallModalState {
  open: boolean;
  skillNames: string[];
  initialScope: InstallScope;
  initialProjectPath: string | null;
}

export interface ForceModalState {
  open: boolean;
  blockers: string[];
  onConfirm: (() => Promise<void>) | null;
  onCancel: (() => void) | null;
}

export const importModal = writable<ImportModalState>({
  open: false,
  initialTab: "github",
  initialFolder: null,
});

export const installModal = writable<InstallModalState>({
  open: false,
  skillNames: [],
  initialScope: "user",
  initialProjectPath: null,
});

export const forceModal = writable<ForceModalState>({
  open: false,
  blockers: [],
  onConfirm: null,
  onCancel: null,
});

export function openImportModal(options?: { tab?: ImportTab; folder?: string | null }): void {
  importModal.set({
    open: true,
    initialTab: options?.tab ?? "github",
    initialFolder: options?.folder ?? null,
  });
}

export function closeImportModal(): void {
  importModal.update((state) => ({ ...state, open: false }));
}

export function openInstallModal(
  skillNames: string[],
  options?: { scope?: InstallScope; projectPath?: string | null }
): void {
  if (skillNames.length === 0) return;
  installModal.set({
    open: true,
    skillNames,
    initialScope: options?.scope ?? "user",
    initialProjectPath: options?.projectPath ?? null,
  });
}

export function closeInstallModal(): void {
  installModal.update((state) => ({ ...state, open: false }));
}

/**
 * Run a hub action that may be refused because local changes would be lost. When the
 * backend reports blockers, the user is asked whether to force the action; the returned
 * promise resolves with the final (possibly still blocked) result.
 */
export async function performAction(
  run: (force: boolean) => Promise<ActionResult>
): Promise<ActionResult> {
  const first = await run(false);
  applySkillView(first.skill);
  if (first.applied || first.blockers.length === 0) {
    return first;
  }
  return new Promise<ActionResult>((resolve) => {
    forceModal.set({
      open: true,
      blockers: first.blockers,
      onConfirm: async () => {
        try {
          const forced = await run(true);
          applySkillView(forced.skill);
          resolve(forced);
        } catch (error) {
          resolve({ applied: false, blockers: [String(error)], skill: first.skill });
        }
      },
      onCancel: () => resolve(first),
    });
  });
}

export function closeForceModal(): void {
  forceModal.set({ open: false, blockers: [], onConfirm: null, onCancel: null });
}
