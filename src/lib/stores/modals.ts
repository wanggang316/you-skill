import { writable } from "svelte/store";
import type { ActionResult, DiffAgainst, InstallScope, ScopeRef } from "../api/hub";
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

export interface SkillPickerModalState {
  open: boolean;
  scope: ScopeRef | null;
}

export interface ScopeAgentModalState {
  open: boolean;
  scope: ScopeRef | null;
  scopeName: string;
  /** Skills of the scope; each is installed into the agents that get added. */
  skillNames: string[];
  existingIds: string[];
}

export interface WorkspaceModalState {
  open: boolean;
  /** "add" registers a new workspace; "rescan" looks again at an existing one. */
  mode: "add" | "rescan";
  name: string;
  path: string;
}

export interface DiffModalState {
  open: boolean;
  name: string;
  against: DiffAgainst;
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

export const skillPickerModal = writable<SkillPickerModalState>({ open: false, scope: null });

export const scopeAgentModal = writable<ScopeAgentModalState>({
  open: false,
  scope: null,
  scopeName: "",
  skillNames: [],
  existingIds: [],
});

export const workspaceModal = writable<WorkspaceModalState>({
  open: false,
  mode: "add",
  name: "",
  path: "",
});

/** The project list dialog, opened from the projects page and the tray. */
export const projectFormModal = writable<{ open: boolean }>({ open: false });

export const diffModal = writable<DiffModalState>({
  open: false,
  name: "",
  against: { kind: "source" },
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

export function openDiffModal(name: string, against: DiffAgainst): void {
  diffModal.set({ open: true, name, against });
}

export function closeDiffModal(): void {
  diffModal.update((state) => ({ ...state, open: false }));
}

/** Pick library skills that are not yet installed in `scope`, then open the install dialog. */
export function openSkillPickerModal(scope: ScopeRef): void {
  skillPickerModal.set({ open: true, scope });
}

export function closeSkillPickerModal(): void {
  skillPickerModal.update((state) => ({ ...state, open: false }));
}

export function openScopeAgentModal(options: {
  scope: ScopeRef;
  scopeName: string;
  skillNames: string[];
  existingIds: string[];
}): void {
  scopeAgentModal.set({ open: true, ...options });
}

export function closeScopeAgentModal(): void {
  scopeAgentModal.update((state) => ({ ...state, open: false }));
}

export function openProjectFormModal(): void {
  projectFormModal.set({ open: true });
}

export function closeProjectFormModal(): void {
  projectFormModal.set({ open: false });
}

export function openWorkspaceModal(options?: {
  mode?: "add" | "rescan";
  name?: string;
  path?: string;
}): void {
  workspaceModal.set({
    open: true,
    mode: options?.mode ?? "add",
    name: options?.name ?? "",
    path: options?.path ?? "",
  });
}

export function closeWorkspaceModal(): void {
  workspaceModal.update((state) => ({ ...state, open: false }));
}
