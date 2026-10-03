import { apiCall } from "./index";

/** A folder registered for repeated skill scans. */
export interface ScanRoot {
  name: string;
  path: string;
  lastScannedAt?: string | null;
  /** How many skill folders the last scan of this path reported. */
  lastFound?: number | null;
  /** The folder was deleted or renamed since it was registered. */
  missing?: boolean;
}

/** An agent skills directory that exists but is not registered yet. */
export interface ScanRootSuggestion {
  path: string;
  agentIds: string[];
}

export async function listScanRoots(): Promise<ScanRoot[]> {
  return apiCall<ScanRoot[]>("list_scan_roots");
}

export async function addScanRoot(name: string, path: string): Promise<ScanRoot> {
  return apiCall<ScanRoot>("add_scan_root", { name, path });
}

export async function removeScanRoot(path: string): Promise<void> {
  return apiCall<void>("remove_scan_root", { path });
}

export async function suggestScanRoots(): Promise<ScanRootSuggestion[]> {
  return apiCall<ScanRootSuggestion[]>("suggest_scan_roots");
}
