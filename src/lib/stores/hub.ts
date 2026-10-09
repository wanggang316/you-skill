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

/** Automatic checks run at most this often; the GitHub API allows 60 anonymous requests an hour. */
const AUTO_SOURCE_CHECK_INTERVAL_MS = 5 * 60 * 1000;

let sourceCheckPromise: Promise<SourceUpdate[]> | null = null;
let lastFullSourceCheckAt = 0;

export function checkSourceUpdates(names?: string[]): Promise<SourceUpdate[]> {
  if (sourceCheckPromise) return sourceCheckPromise;
  sourceChecking.set(true);
  sourceCheckPromise = (async () => {
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
      sourceCheckPromise = null;
    }
  })();
  return sourceCheckPromise;
}

/**
 * Check every GitHub-sourced skill in one batched call. Automatic triggers (startup, opening
 * the library) are throttled; `force` is for an explicit refresh.
 */
export async function checkAllSourceUpdates(
  options: { force?: boolean } = {}
): Promise<SourceUpdate[]> {
  if (!get(hubSkills).some((skill) => skill.source.type === "github")) return [];
  const now = Date.now();
  if (!options.force && now - lastFullSourceCheckAt < AUTO_SOURCE_CHECK_INTERVAL_MS) {
    return get(lastSourceCheck);
  }
  lastFullSourceCheckAt = now;
  return checkSourceUpdates();
}

export async function loadMigrationReport(): Promise<void> {
  try {
    migrationReport.set(await migrationStatus());
  } catch (error) {
    console.error("Failed to load migration status:", error);
  }
}
