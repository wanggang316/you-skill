import { writable } from "svelte/store";
import type { ActionResult, DiffAgainst, InstallScope, ScopeRef } from "../api/hub";
import type { InstructionActionResult } from "../api/instructions";
import { applySkillView } from "./hub";
import { applyInstructionView } from "./instructions";

/** Which library a dialog acts on: hub skills or instruction files. */
export type LibraryKind = "skill" | "instruction";

export type ImportTab = "github" | "zip" | "folder";

export interface ImportModalState {
  open: boolean;
  initialTab: ImportTab;
  /** Pre-selected folder for the scan tab. */
  initialFolder: string | null;
}

export interface InstallModalState {
  open: boolean;
  kind: LibraryKind;
  skillNames: string[];
  initialScope: InstallScope;
  initialProjectPath: string | null;
  /** Several places at once; only used together with `lockScope`. */
  targets: ScopeRef[];
  /** The target is fixed by where the action started; the dialog offers no scope switch. */
  lockScope: boolean;
}

export interface LocationPickerModalState {
  open: boolean;
  kind: LibraryKind;
  skillName: string;
}

export interface ImportInstructionModalState {
  open: boolean;
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
  kind: LibraryKind;
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
  kind: "skill",
  skillNames: [],
  initialScope: "user",
  initialProjectPath: null,
  targets: [],
  lockScope: false,
});

export const locationPickerModal = writable<LocationPickerModalState>({
  open: false,
  kind: "skill",
  skillName: "",
});

export const importInstructionModal = writable<ImportInstructionModalState>({ open: false });

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

export const diffModal = writable<DiffModalState>({
  open: false,
  kind: "skill",
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
  options?: {
    scope?: InstallScope;
    projectPath?: string | null;
    targets?: ScopeRef[];
    lockScope?: boolean;
    kind?: LibraryKind;
  }
): void {
  if (skillNames.length === 0) return;
  installModal.set({
    open: true,
    kind: options?.kind ?? "skill",
    skillNames,
    initialScope: options?.scope ?? "user",
    initialProjectPath: options?.projectPath ?? null,
    targets: options?.targets ?? [],
    lockScope: options?.lockScope ?? false,
  });
}

export function openLocationPickerModal(skillName: string, kind: LibraryKind = "skill"): void {
  locationPickerModal.set({ open: true, kind, skillName });
}

export function openImportInstructionModal(): void {
  importInstructionModal.set({ open: true });
}

export function closeImportInstructionModal(): void {
  importInstructionModal.set({ open: false });
}

export function closeLocationPickerModal(): void {
  locationPickerModal.update((state) => ({ ...state, open: false }));
}

export function closeInstallModal(): void {
  installModal.update((state) => ({ ...state, open: false }));
}

export interface ActionOutcome {
  applied: boolean;
  blockers: string[];
}

/**
 * Run an action that may be refused because local changes would be lost. When the backend
 * reports blockers, the user is asked whether to force the action; the returned promise
 * resolves with the final (possibly still blocked) result. `apply` pushes each result into
 * the matching store.
 */
export async function confirmForce<T extends ActionOutcome>(
  run: (force: boolean) => Promise<T>,
  apply: (result: T) => void
): Promise<T> {
  const first = await run(false);
  apply(first);
  if (first.applied || first.blockers.length === 0) {
    return first;
  }
  return new Promise<T>((resolve) => {
    forceModal.set({
      open: true,
      blockers: first.blockers,
      onConfirm: async () => {
        try {
          const forced = await run(true);
          apply(forced);
          resolve(forced);
        } catch (error) {
          resolve({ ...first, applied: false, blockers: [String(error)] });
        }
      },
      onCancel: () => resolve(first),
    });
  });
}

/** `confirmForce` for hub skills. */
export function performAction(
  run: (force: boolean) => Promise<ActionResult>
): Promise<ActionResult> {
  return confirmForce(run, (result) => applySkillView(result.skill));
}

/** `confirmForce` for library instructions. */
export function performInstructionAction(
  run: (force: boolean) => Promise<InstructionActionResult>
): Promise<InstructionActionResult> {
  return confirmForce(run, (result) => applyInstructionView(result.instruction));
}

export function closeForceModal(): void {
  forceModal.set({ open: false, blockers: [], onConfirm: null, onCancel: null });
}

export function openDiffModal(
  name: string,
  against: DiffAgainst,
  kind: LibraryKind = "skill"
): void {
  diffModal.set({ open: true, kind, name, against });
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
