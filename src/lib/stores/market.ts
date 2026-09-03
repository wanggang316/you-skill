import { writable } from "svelte/store";
import { fetchRemoteSkills, type RemoteSkill } from "../api/skills";

export const remoteSkills = writable<RemoteSkill[]>([]);
export const remoteLoading = writable(false);
export const remoteError = writable("");
export const remoteHasMore = writable(false);
export const remoteTotal = writable(0);
export const remoteLoaded = writable(false);

export async function loadRemote(options: {
  reset?: boolean;
  skip: number;
  limit: number;
  search: string;
  sortBy: string;
  sortOrder: string;
}): Promise<void> {
  const { reset = false, skip, limit, search, sortBy, sortOrder } = options;
  remoteLoading.set(true);
  remoteError.set("");
  try {
    if (reset) {
      remoteSkills.set([]);
    }
    const response = await fetchRemoteSkills({ skip, limit, search, sortBy, sortOrder });
    remoteHasMore.set(response.has_more);
    remoteTotal.set(response.total);
    if (reset) {
      remoteSkills.set(response.skills);
    } else {
      remoteSkills.update((current) => [...current, ...response.skills]);
    }
    remoteLoaded.set(true);
  } catch (error) {
    remoteError.set(String(error));
  } finally {
    remoteLoading.set(false);
  }
}
