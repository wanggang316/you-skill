/**
 * Agent Apps API
 *
 * 处理 Agent 应用管理相关的 IPC 调用
 */

import { apiCall } from "./index";
import type { InstallScope } from "./hub";
import type { AgentInfo } from "./skills";

// ============ Types ============

export type AgentApp = AgentInfo;

/** An agent instruction file of one scope; agents reading the same file share an entry. */
export interface MemoryFile {
  path: string;
  name: string;
  agentIds: string[];
  exists: boolean;
}

// ============ Agent Apps ============

/**
 * 获取本地已安装的 Agent 应用列表
 */
export async function listLocalAgentApps(): Promise<AgentApp[]> {
  return apiCall<AgentApp[]>("list_local_agent_apps");
}

/**
 * 刷新 Agent 应用 - 清除缓存并重新扫描文件系统
 */
export async function refreshAgentApps(): Promise<AgentApp[]> {
  return apiCall<AgentApp[]>("refresh_agent_apps");
}

/**
 * 添加用户自定义 Agent 应用
 */
export async function addAgentApp(
  displayName: string,
  globalPath: string,
  projectPath?: string,
  profilePath?: string,
  globalProfilePath?: string
): Promise<AgentApp> {
  return apiCall<AgentApp>("add_user_agent_app", {
    displayName,
    globalPath,
    projectPath,
    profilePath,
    globalProfilePath,
  });
}

/**
 * 移除用户自定义 Agent 应用
 */
export async function removeAgentApp(id: string): Promise<void> {
  return apiCall<void>("remove_user_agent_app", { id });
}

/**
 * 更新用户自定义 Agent 应用
 */
export async function updateAgentApp(
  id: string,
  displayName: string,
  globalPath: string,
  projectPath?: string,
  profilePath?: string,
  globalProfilePath?: string
): Promise<AgentApp> {
  return apiCall<AgentApp>("update_user_agent_app", {
    id,
    displayName,
    globalPath,
    projectPath,
    profilePath,
    globalProfilePath,
  });
}

/** Agent instruction files of one scope, with the agents that read each of them. */
export async function listMemoryFiles(
  scope: InstallScope,
  projectPath?: string | null
): Promise<MemoryFile[]> {
  return apiCall<MemoryFile[]>("list_memory_files", { scope, projectPath: projectPath ?? null });
}
