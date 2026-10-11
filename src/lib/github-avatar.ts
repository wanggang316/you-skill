/**
 * GitHub owner avatars, cached for the app session as object URLs on top of the backend
 * disk cache, so remounted lists show them without a network round trip.
 */

import { getGithubAvatar } from "./api/github";

const resolved = new Map<string, string | null>();
const pending = new Map<string, Promise<string | null>>();

const keyOf = (owner: string) => owner.toLowerCase();

/** The loaded avatar URL, `null` when it failed, `undefined` when not loaded yet. */
export function cachedGithubAvatar(owner: string): string | null | undefined {
  return resolved.get(keyOf(owner));
}

export function loadGithubAvatar(owner: string): Promise<string | null> {
  const key = keyOf(owner);
  if (resolved.has(key)) return Promise.resolve(resolved.get(key) ?? null);
  let request = pending.get(key);
  if (!request) {
    request = getGithubAvatar(owner)
      .then((bytes) => URL.createObjectURL(new Blob([bytes])))
      .catch(() => null)
      .then((url) => {
        resolved.set(key, url);
        pending.delete(key);
        return url;
      });
    pending.set(key, request);
  }
  return request;
}
