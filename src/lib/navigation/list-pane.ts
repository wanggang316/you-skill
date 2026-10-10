import { writable } from "svelte/store";

/** Width in px of the list column on split pages; resizable and shared by all of them. */
export const LIST_PANE = {
  initial: 288,
  min: 220,
  max: 480,
} as const;

const STORAGE_KEY = "youskill.listPaneWidth";

export const clampListPaneWidth = (width: number): number =>
  Math.round(Math.min(LIST_PANE.max, Math.max(LIST_PANE.min, width)));

function readWidth(): number {
  try {
    const value = Number(localStorage.getItem(STORAGE_KEY));
    return Number.isFinite(value) && value > 0 ? clampListPaneWidth(value) : LIST_PANE.initial;
  } catch {
    return LIST_PANE.initial;
  }
}

export const listPaneWidth = writable(readWidth());

export function saveListPaneWidth(width: number): void {
  listPaneWidth.set(width);
  try {
    localStorage.setItem(STORAGE_KEY, String(width));
  } catch {
    // Remembering the width is best effort.
  }
}
