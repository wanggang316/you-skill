/**
 * Agent Apps API
 *
 * 处理 Agent 应用管理相关的 IPC 调用
 */

import { apiCall } from "./index";
import type { AgentInfo } from "./skills";

// ============ Types ============

export type AgentApp = AgentInfo;

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
 * 获取内置 Agent 应用列表
 */
export async function listInternalAgentApps(): Promise<AgentApp[]> {
  return apiCall<AgentApp[]>("list_internal_agent_apps");
}

/**
 * 获取用户自定义 Agent 应用列表
 */
export async function listUserAgentApps(): Promise<AgentApp[]> {
  return apiCall<AgentApp[]>("list_user_agent_apps");
}

/**
 * 添加用户自定义 Agent 应用
 */
export async function addAgentApp(
  displayName: string,
  globalPath: string,
  projectPath?: string
): Promise<AgentApp> {
  return apiCall<AgentApp>("add_user_agent_app", {
    displayName,
    globalPath,
    projectPath,
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
  projectPath?: string
): Promise<AgentApp> {
  return apiCall<AgentApp>("update_user_agent_app", {
    id,
    displayName,
    globalPath,
    projectPath,
  });
}
