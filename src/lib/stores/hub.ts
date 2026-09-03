import { derived, get, writable } from "svelte/store";
import { listLocalAgentApps } from "../api/agent-apps";
import {
  checkSourceUpdates as checkSourceUpdatesApi,
  listHubSkills,
  migrationStatus,
  type HubSkillView,
  type MigrationReport,
  type SourceUpdate,
} from "../api/hub";
import type { AgentInfo } from "../api/skills";

export const agents = writable<AgentInfo[]>([]);

export const hubSkills = writable<HubSkillView[]>([]);
export const hubLoading = writable(false);
export const hubError = writable("");
export const hubLoaded = writable(false);

export const sourceChecking = writable(false);
export const lastSourceCheck = writable<SourceUpdate[]>([]);

export const migrationReport = writable<MigrationReport | null>(null);

export const hubSkillsByName = derived(
  hubSkills,
  ($skills) => new Map($skills.map((skill) => [skill.name, skill]))
);

export const agentsById = derived(
  agents,
  ($agents) => new Map($agents.map((agent) => [agent.id, agent]))
);

export async function loadAgents(): Promise<void> {
  try {
    agents.set(await listLocalAgentApps());
  } catch (error) {
    console.error(error);
  }
}

let refreshPromise: Promise<void> | null = null;

export async function refreshHub(): Promise<void> {
  if (refreshPromise) return refreshPromise;
  hubLoading.set(true);
  hubError.set("");
  refreshPromise = (async () => {
    try {
      const skills = await listHubSkills();
      skills.sort((a, b) => a.name.localeCompare(b.name));
      hubSkills.set(skills);
      hubLoaded.set(true);
    } catch (error) {
      hubError.set(String(error));
    } finally {
      hubLoading.set(false);
      refreshPromise = null;
    }
  })();
  return refreshPromise;
}

export function applySkillView(view: HubSkillView | null | undefined): void {
  if (!view) return;
  hubSkills.update((current) => {
    const index = current.findIndex((skill) => skill.name === view.name);
    if (index === -1) {
      return [...current, view].sort((a, b) => a.name.localeCompare(b.name));
    }
    const next = [...current];
    next[index] = view;
    return next;
  });
}

export async function checkSourceUpdates(names?: string[]): Promise<SourceUpdate[]> {
  if (get(sourceChecking)) return get(lastSourceCheck);
  sourceChecking.set(true);
  try {
    const result = await checkSourceUpdatesApi(names ?? null);
    lastSourceCheck.set(result);
    await refreshHub();
    return result;
  } catch (error) {
    console.error("Failed to check source updates:", error);
    return [];
  } finally {
    sourceChecking.set(false);
  }
}

export async function loadMigrationReport(): Promise<void> {
  try {
    migrationReport.set(await migrationStatus());
  } catch (error) {
    console.error("Failed to load migration status:", error);
  }
}
