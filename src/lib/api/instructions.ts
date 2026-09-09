/**
 * Instruction library API
 *
 * Agent instruction files (AGENTS.md, CLAUDE.md, ...) kept once under
 * ~/.youskill/instructions and installed to the file each agent reads. Install records
 * share the hub types; only the unit differs (a file, not a directory).
 */

import { apiCall } from "./index";
import type {
  HubState,
  InstallRequest,
  InstallView,
  SkillDiff,
  SyncAction,
  UninstallRequest,
} from "./hub";

// ============ Types ============

export interface InstructionView {
  name: string;
  hubPath: string;
  /** First line of the file. */
  description?: string | null;
  hash: string;
  hubHash?: string | null;
  hubState: HubState;
  importedAt: string;
  updatedAt: string;
  installs: InstallView[];
  hasDrift: boolean;
}

export interface InstructionImportItem {
  name: string;
  /** Markdown file to copy into the library. */
  path: string;
}

export interface InstructionImportOutcome {
  name: string;
  hubPath: string;
  hash: string;
  replaced: boolean;
}

export interface InstructionActionResult {
  applied: boolean;
  blockers: string[];
  instruction?: InstructionView | null;
}

/** A Markdown file found in a folder, among picked files, or in a downloaded repository. */
export interface DetectedInstruction {
  /** Suggested library name, editable before importing. */
  name: string;
  /** Absolute path to copy from (a temp directory for GitHub sources). */
  path: string;
  /** Path relative to the folder or repository it was found in. */
  relPath: string;
  fileName: string;
}

// ============ Commands ============

export async function listInstructions(): Promise<InstructionView[]> {
  return apiCall<InstructionView[]>("list_instructions");
}

export async function getInstruction(name: string): Promise<InstructionView | null> {
  return apiCall<InstructionView | null>("get_instruction", { name });
}

export async function readInstruction(name: string): Promise<string> {
  return apiCall<string>("read_instruction", { name });
}

export async function importInstructions(
  items: InstructionImportItem[],
  overwrite = false
): Promise<InstructionImportOutcome[]> {
  return apiCall<InstructionImportOutcome[]>("import_instructions", { items, overwrite });
}

export async function createInstruction(
  name: string,
  content = ""
): Promise<InstructionImportOutcome> {
  return apiCall<InstructionImportOutcome>("create_instruction", { name, content });
}

export async function installInstruction(
  request: InstallRequest
): Promise<InstructionActionResult> {
  return apiCall<InstructionActionResult>("install_instruction", { request });
}

export async function uninstallInstruction(
  request: UninstallRequest
): Promise<InstructionActionResult> {
  return apiCall<InstructionActionResult>("uninstall_instruction", { request });
}

export async function removeInstruction(name: string, removeInstalls: boolean): Promise<void> {
  return apiCall<void>("remove_instruction", { name, removeInstalls });
}

export async function syncInstruction(
  name: string,
  action: SyncAction
): Promise<InstructionActionResult> {
  return apiCall<InstructionActionResult>("sync_instruction", { name, action });
}

export async function diffInstruction(name: string, path: string): Promise<SkillDiff> {
  return apiCall<SkillDiff>("diff_instruction", { name, path });
}

/** Save edited content; unmodified copies are pushed, edited ones come back as blockers. */
export async function writeInstruction(
  name: string,
  content: string
): Promise<InstructionActionResult> {
  return apiCall<InstructionActionResult>("write_instruction", { name, content });
}

/** Markdown files in the given files and folders. */
export async function detectInstructionFiles(paths: string[]): Promise<DetectedInstruction[]> {
  return apiCall<DetectedInstruction[]>("detect_instruction_files", { paths });
}

/** Markdown files in a GitHub repository, folder or file URL. */
export async function detectInstructionGithub(githubPath: string): Promise<DetectedInstruction[]> {
  return apiCall<DetectedInstruction[]>("detect_instruction_github", { githubPath });
}
