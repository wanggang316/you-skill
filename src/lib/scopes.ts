/**
 * Install scopes: the user level and every project. A scope groups the skills installed
 * into it, which is what the projects page lists and details.
 */

import { scopedInstalls, type HubSkillView, type ScopeRef } from "./api/hub";
import type { UserProject } from "./api/user-projects";

export interface ScopeEntry {
  key: string;
  kind: "user" | "project";
  name: string;
  /** Absolute folder: the home directory for the user scope, the project folder otherwise. */
  path: string;
  ref: ScopeRef;
  skills: HubSkillView[];
  /** Skills whose install in this scope is out of sync with the hub. */
  driftCount: number;
  /** The project folder no longer exists. */
  missing: boolean;
  /** Has installs but is not in the project list. */
  unregistered: boolean;
  /** Workspace the project was discovered in. */
  workspacePath: string | null;
}

export const scopeKey = (ref: ScopeRef): string =>
  ref.scope === "user" ? "user" : `project:${ref.projectPath ?? ""}`;

export const baseName = (path: string): string => path.split(/[/\\]/).filter(Boolean).pop() || path;

const collect = (skills: HubSkillView[], ref: ScopeRef) => {
  const installed = skills.filter((skill) => scopedInstalls(skill, ref).length > 0);
  const driftCount = installed.filter((skill) =>
    scopedInstalls(skill, ref).some((install) => install.state !== "in_sync")
  ).length;
  const missing = installed.some((skill) =>
    scopedInstalls(skill, ref).some((install) => install.projectMissing)
  );
  return { installed, driftCount, missing };
};

/**
 * The user scope first, then every registered project, then projects that only appear in
 * install records (folders that were scanned or migrated but never registered).
 */
export function buildScopeEntries(
  skills: HubSkillView[],
  projects: UserProject[],
  userLabel: string,
  homePath = ""
): ScopeEntry[] {
  const userRef: ScopeRef = { scope: "user", projectPath: null };
  const user = collect(skills, userRef);
  const entries: ScopeEntry[] = [
    {
      key: "user",
      kind: "user",
      name: userLabel,
      path: homePath,
      ref: userRef,
      skills: user.installed,
      driftCount: user.driftCount,
      missing: false,
      unregistered: false,
      workspacePath: null,
    },
  ];

  const seen = new Set<string>();
  const push = (
    path: string,
    name: string,
    unregistered: boolean,
    workspacePath: string | null
  ) => {
    if (seen.has(path)) return;
    seen.add(path);
    const ref: ScopeRef = { scope: "project", projectPath: path };
    const { installed, driftCount, missing } = collect(skills, ref);
    entries.push({
      key: scopeKey(ref),
      kind: "project",
      name,
      path,
      ref,
      skills: installed,
      driftCount,
      missing,
      unregistered,
      workspacePath,
    });
  };

  for (const project of projects) {
    push(project.path, project.name, false, project.workspacePath ?? null);
  }
  for (const skill of skills) {
    for (const install of skill.installs) {
      if (install.scope !== "project" || !install.projectPath) continue;
      push(install.projectPath, baseName(install.projectPath), true, null);
    }
  }
  return entries;
}
