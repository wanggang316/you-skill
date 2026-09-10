/** Sidebar width in px: user-resizable, remembered per machine, icons only when narrow. */
export const SIDEBAR = {
  initial: 180,
  min: 176,
  max: 360,
  /** Width of the icons-only sidebar: wide enough to keep the window controls inside it. */
  collapsed: 80,
  /** Dragging narrower than this snaps to icons only. */
  collapseBelow: 140,
} as const;

const STORAGE_KEY = "youskill.sidebarWidth";

export function readSidebarWidth(): number {
  try {
    const value = Number(localStorage.getItem(STORAGE_KEY));
    if (!Number.isFinite(value) || value <= 0) return SIDEBAR.initial;
    if (value <= SIDEBAR.collapsed) return SIDEBAR.collapsed;
    return Math.min(SIDEBAR.max, Math.max(SIDEBAR.min, value));
  } catch {
    return SIDEBAR.initial;
  }
}

export function writeSidebarWidth(width: number): void {
  try {
    localStorage.setItem(STORAGE_KEY, String(Math.round(width)));
  } catch {
    // Remembering the width is best effort.
  }
}
