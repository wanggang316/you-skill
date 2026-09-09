/**
 * Instruction library API
 *
 * Agent instruction files (AGENTS.md, CLAUDE.md, ...) kept once under
 * ~/.youskill/instructions and installed to the file each agent reads. Install records
 * share the hub types; only the unit differs (a file, not a directory).
 */

import { apiCall } from "./index";
import type {
  AgentRootMatch,
  HubState,
  InstallRequest,
  InstallView,
  ScanResolution,
  ScanStatus,
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

/** An instruction file found at an agent location (user level or a registered project). */
export interface InstructionScanItem {
  /** Suggested library name, editable before importing. */
  name: string;
  path: string;
  fileName: string;
  hash?: string | null;
  status: ScanStatus;
  /** Library entry the file belongs to, by install record or by identical content. */
  hubName?: string | null;
  /** The path is already an install record of `hubName`. */
  registered: boolean;
  location: AgentRootMatch;
  error?: string | null;
}

export interface InstructionScanDecision {
  name: string;
  path: string;
  resolution: ScanResolution;
  registerInstall: boolean;
  hubName?: string | null;
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

export async function scanInstructionFiles(): Promise<InstructionScanItem[]> {
  return apiCall<InstructionScanItem[]>("scan_instruction_files");
}

export async function importScannedInstructions(
  decisions: InstructionScanDecision[]
): Promise<InstructionImportOutcome[]> {
  return apiCall<InstructionImportOutcome[]>("import_scanned_instructions", { decisions });
}
