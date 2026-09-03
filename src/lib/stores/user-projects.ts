import { writable } from "svelte/store";
import { listUserProjects, type UserProject } from "$lib/api/user-projects";

export const userProjects = writable<UserProject[]>([]);

export const refreshUserProjects = async (): Promise<UserProject[]> => {
  const projects = await listUserProjects();
  userProjects.set(projects);
  return projects;
};
