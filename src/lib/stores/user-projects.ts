import { get, writable } from "svelte/store";
import { scopedInstalls, uninstallSkill, type ScopeRef } from "$lib/api/hub";
import { uninstallInstruction } from "$lib/api/instructions";
import {
  listUserProjects,
  listWorkspaces,
  removeUserProject,
  type UserProject,
  type UserWorkspace,
} from "$lib/api/user-projects";
import { hubSkills, refreshHub } from "./hub";
import { instructions, refreshInstructions } from "./instructions";

export const userProjects = writable<UserProject[]>([]);
export const workspaces = writable<UserWorkspace[]>([]);

export const refreshUserProjects = async (): Promise<UserProject[]> => {
  const projects = await listUserProjects();
  userProjects.set(projects);
  return projects;
};

export const refreshWorkspaces = async (): Promise<UserWorkspace[]> => {
  const list = await listWorkspaces();
  workspaces.set(list);
  return list;
};

const projectRef = (path: string): ScopeRef => ({ scope: "project", projectPath: path });

/** Skills and instructions with an install record in the project folder. */
export function projectInstallCount(path: string): number {
  const ref = projectRef(path);
  const skills = get(hubSkills).filter((skill) => scopedInstalls(skill, ref).length > 0);
  const items = get(instructions).filter((item) => scopedInstalls(item, ref).length > 0);
  return skills.length + items.length;
}

/**
 * Forget a project whose folder is gone: drop the install records that point into it (no
 * file is touched) and, when it is registered, remove it from the project list. Returns
 * the records that could not be dropped.
 */
export async function forgetProject(
  path: string,
  registeredName: string | null
): Promise<string[]> {
  const ref = projectRef(path);
  const failures: string[] = [];
  for (const skill of get(hubSkills)) {
    const paths = scopedInstalls(skill, ref).map((install) => install.path);
    if (paths.length === 0) continue;
    const result = await uninstallSkill({ name: skill.name, targets: [], paths, force: true });
    if (!result.applied) failures.push(`${skill.name}: ${result.blockers.join("; ")}`);
  }
  for (const item of get(instructions)) {
    const paths = scopedInstalls(item, ref).map((install) => install.path);
    if (paths.length === 0) continue;
    const result = await uninstallInstruction({ name: item.name, targets: [], paths, force: true });
    if (!result.applied) failures.push(`${item.name}: ${result.blockers.join("; ")}`);
  }
  if (registeredName) await removeUserProject(registeredName);
  await Promise.all([refreshUserProjects(), refreshHub(), refreshInstructions()]);
  return failures;
}
