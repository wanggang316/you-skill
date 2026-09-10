/**
 * Back / forward state for the sidebar buttons.
 *
 * SvelteKit numbers history entries in `history.state`; reading that index tells how far
 * back we are, and the highest index seen tells how far forward we can go. Should the
 * key ever be missing, the buttons stay enabled and the browser decides.
 */

import { writable } from "svelte/store";
import type { AfterNavigate } from "@sveltejs/kit";

const HISTORY_INDEX = "sveltekit:history";

export const canGoBack = writable(false);
export const canGoForward = writable(false);

let index: number | null = null;
let maxIndex = 0;

function readIndex(): number | null {
  if (typeof history === "undefined") return null;
  const value = (history.state as Record<string, unknown> | null)?.[HISTORY_INDEX];
  return typeof value === "number" ? value : null;
}

/** Call from `afterNavigate`; keeps the two stores in step with the history position. */
export function trackNavigation(navigation: AfterNavigate): void {
  const current = readIndex();
  if (current === null) {
    canGoBack.set(history.length > 1);
    canGoForward.set(true);
    return;
  }
  if (index === null) {
    maxIndex = current;
  } else if (navigation.type !== "popstate" && current > index) {
    // A new entry cuts off whatever was ahead.
    maxIndex = current;
  } else if (current > maxIndex) {
    maxIndex = current;
  }
  index = current;
  canGoBack.set(current > 0);
  canGoForward.set(current < maxIndex);
}

export function goBack(): void {
  history.back();
}

export function goForward(): void {
  history.forward();
}
