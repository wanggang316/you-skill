import { apiCall } from "./index";

export interface UserProject {
  name: string;
  path: string;
}

export async function listUserProjects(): Promise<UserProject[]> {
  return apiCall<UserProject[]>("list_user_projects");
}

export async function addUserProject(name: string, path: string): Promise<UserProject> {
  return apiCall<UserProject>("add_user_project", { name, path });
}

export async function updateUserProject(
  originalName: string,
  name: string,
  path: string
): Promise<UserProject> {
  return apiCall<UserProject>("update_user_project", { originalName, name, path });
}

export async function removeUserProject(name: string): Promise<void> {
  return apiCall<void>("remove_user_project", { name });
}
