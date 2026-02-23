import { get, writable } from "svelte/store";
import { listLocalAgentApps } from "../api/agent-apps";
import {
  checkSkillsUpdates,
  fetchRemoteSkills,
  fetchSkillsByNames,
  listSkills,
  type AgentInfo,
  type LocalSkill,
  type RemoteSkill,
} from "../api/skills";

export const agents = writable<AgentInfo[]>([]);

export const localSkills = writable<LocalSkill[]>([]);
export const localLoading = writable(false);
export const localError = writable("");

export const remoteSkills = writable<RemoteSkill[]>([]);
export const remoteLoading = writable(false);
export const remoteError = writable("");
export const remoteHasMore = writable(false);
export const remoteTotal = writable(0);
export const remoteLoaded = writable(false);

export const skillsWithUpdate = writable<RemoteSkill[]>([]);
export const updatingSkills = writable<string[]>([]);

let isCheckingUpdates = false;
let updateCheckPromise: Promise<void> | null = null;

export async function loadAgents(): Promise<void> {
  try {
    agents.set(await listLocalAgentApps());
  } catch (error) {
    console.error(error);
  }
}

export async function refreshLocal(): Promise<void> {
  localLoading.set(true);
  localError.set("");
  try {
    localSkills.set(await listSkills());
  } catch (error) {
    localError.set(String(error));
  } finally {
    localLoading.set(false);
  }
}

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
    const response = await fetchRemoteSkills({
      skip,
      limit,
      search,
      sortBy,
      sortOrder,
    });
    remoteHasMore.set(response.has_more);
    remoteTotal.set(response.total);
    if (reset) {
      remoteSkills.set(response.skills);
    } else {
      remoteSkills.update((current) => [...current, ...response.skills]);
    }
    await checkUpdatesFromRemoteList();
  } catch (error) {
    remoteError.set(String(error));
  } finally {
    remoteLoading.set(false);
  }
}

export async function checkForSkillUpdates(): Promise<void> {
  if (isCheckingUpdates) {
    if (updateCheckPromise) {
      await updateCheckPromise;
    }
    return;
  }

  const local = get(localSkills);
  if (local.length === 0) {
    skillsWithUpdate.set([]);
    return;
  }

  isCheckingUpdates = true;

  const checkPromise = (async () => {
    try {
      const skillNames = local.map((s) => s.name);
      const remoteSkillsByName = await fetchSkillsByNames(skillNames);
        const remoteByName = new Map(remoteSkillsByName.map((skill) => [skill.name, skill]));
        const checks = remoteSkillsByName
          .filter((skill) => Boolean(skill.skill_path_sha))
          .map((skill) => ({
            name: skill.name,
            source: skill.source,
            remote_sha: skill.skill_path_sha!,
          }));
      const updatedNames = new Set(await checkSkillsUpdates(checks));
      const updates = skillNames
        .filter((name) => updatedNames.has(name))
        .map((name) => remoteByName.get(name))
        .filter((skill): skill is RemoteSkill => Boolean(skill));
      skillsWithUpdate.set(updates);
    } catch (error) {
      console.error("Failed to check for skill updates:", error);
    } finally {
      isCheckingUpdates = false;
      updateCheckPromise = null;
    }
  })();

  updateCheckPromise = checkPromise;
  await checkPromise;
}

export async function checkUpdatesFromRemoteList(): Promise<void> {
  const local = get(localSkills);
  const remote = get(remoteSkills);
  if (local.length === 0 || remote.length === 0) return;

  const remoteMap = new Map(remote.map((item) => [item.name, item]));
  const checks = local
      .map((localSkill) => {
        const remoteSkill = remoteMap.get(localSkill.name);
        if (!remoteSkill?.skill_path_sha) return null;
        return {
          name: localSkill.name,
          source: remoteSkill.source,
          remote_sha: remoteSkill.skill_path_sha,
        };
      })
      .filter((item): item is { name: string; source: string; remote_sha: string } => item !== null);
  if (checks.length === 0) return;

  const updatedNames = new Set(await checkSkillsUpdates(checks));
  const next = checks
    .filter((check) => updatedNames.has(check.name))
    .map((check) => remoteMap.get(check.name))
    .filter((skill): skill is RemoteSkill => Boolean(skill));
  skillsWithUpdate.set(next);
}
