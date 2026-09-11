import { writable } from "svelte/store";
import { listScanRoots, type ScanRoot } from "$lib/api/scan-roots";

export const scanRoots = writable<ScanRoot[]>([]);

export const refreshScanRoots = async (): Promise<ScanRoot[]> => {
  const roots = await listScanRoots();
  scanRoots.set(roots);
  return roots;
};
