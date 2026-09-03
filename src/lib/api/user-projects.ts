import { apiCall } from "./index";

export interface UserProject {
  name: string;
  path: string;
  /** Workspace the project was discovered in; absent when it was added by hand. */
  workspacePath?: string | null;
}

/** A folder that holds projects. */
export interface UserWorkspace {
  name: string;
  path: string;
}

/** A folder inside a workspace that looks like a project. */
export interface ProjectCandidate {
  name: string;
  path: string;
  /** Agents whose project skills directory exists there. */
  agentIds: string[];
  /** Instruction files found there (AGENTS.md, CLAUDE.md). */
  profiles: string[];
  skillCount: number;
  registered: boolean;
}

export interface ProjectRegistration {
  name: string;
  path: string;
  workspacePath?: string | null;
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

export async function listWorkspaces(): Promise<UserWorkspace[]> {
  return apiCall<UserWorkspace[]>("list_workspaces");
}

export async function addWorkspace(name: string, path: string): Promise<UserWorkspace> {
  return apiCall<UserWorkspace>("add_workspace", { name, path });
}

export async function removeWorkspace(path: string, removeProjects: boolean): Promise<void> {
  return apiCall<void>("remove_workspace", { path, removeProjects });
}

/** Project folders inside a workspace: an agent skills directory or an instruction file. */
export async function scanWorkspace(path: string, maxDepth?: number): Promise<ProjectCandidate[]> {
  return apiCall<ProjectCandidate[]>("scan_workspace", { path, maxDepth: maxDepth ?? null });
}

export async function registerProjects(projects: ProjectRegistration[]): Promise<UserProject[]> {
  return apiCall<UserProject[]>("register_projects", { projects });
}
