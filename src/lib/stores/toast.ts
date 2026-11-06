import { writable } from "svelte/store";

export interface Toast {
  id: number;
  message: string;
}

const TOAST_DURATION_MS = 4000;

/** Short notices that close by themselves. */
export const toasts = writable<Toast[]>([]);

let nextId = 1;

export function showToast(message: string): void {
  const id = nextId++;
  toasts.update((current) => [...current, { id, message }]);
  setTimeout(() => dismissToast(id), TOAST_DURATION_MS);
}

export function dismissToast(id: number): void {
  toasts.update((current) => current.filter((toast) => toast.id !== id));
}
