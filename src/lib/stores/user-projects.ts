import { writable } from "svelte/store";
import {
  listUserProjects,
  listWorkspaces,
  type UserProject,
  type UserWorkspace,
} from "$lib/api/user-projects";

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
