import { derived, get, writable } from "svelte/store";
import { listLocalAgentApps } from "../api/agent-apps";
import {
  checkSourceUpdates as checkSourceUpdatesApi,
  listHubSkills,
  migrationStatus,
  sourceLabel,
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

export const hubSkillsById = derived(
  hubSkills,
  ($skills) => new Map($skills.map((skill) => [skill.id, skill]))
);

/** Names that more than one skill has; their rows also show the source. */
export const sharedSkillNames = derived(hubSkills, ($skills) => {
  const seen = new Set<string>();
  const shared = new Set<string>();
  for (const skill of $skills) {
    const name = skill.name.toLowerCase();
    if (seen.has(name)) shared.add(name);
    seen.add(name);
  }
  return shared;
});

/** Display name of a skill id: the name, plus the source when another skill has the name. */
export const skillLabel = derived([hubSkillsById, sharedSkillNames], ([$byId, $shared]) => {
  return (id: string): string => {
    const skill = $byId.get(id);
    if (!skill) return id;
    const source = sourceLabel(skill.source) || skill.sourceKey;
    return $shared.has(skill.name.toLowerCase()) ? `${skill.name} (${source})` : skill.name;
  };
});

function compareSkills(a: HubSkillView, b: HubSkillView): number {
  return a.name.localeCompare(b.name) || a.sourceKey.localeCompare(b.sourceKey);
}

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
      skills.sort(compareSkills);
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
    const index = current.findIndex((skill) => skill.id === view.id);
    if (index === -1) {
      return [...current, view].sort(compareSkills);
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

export function checkSourceUpdates(ids?: string[]): Promise<SourceUpdate[]> {
  if (sourceCheckPromise) return sourceCheckPromise;
  sourceChecking.set(true);
  sourceCheckPromise = (async () => {
    try {
      const result = await checkSourceUpdatesApi(ids ?? null);
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
