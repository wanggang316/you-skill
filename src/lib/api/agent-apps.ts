/**
 * Agent Apps API
 *
 * 处理 Agent 应用管理相关的 IPC 调用
 */

import { apiCall } from "./index";
import type { AgentInfo } from "./skills";

// ============ Types ============

export interface AgentAppDetail {
  id: string;
  display_name: string;
  project_path: string | null;
  global_path: string | null;
  is_internal: boolean;
  is_installed: boolean;
}

export interface ValidateResult {
  errors: string[];
  warnings: string[];
}

// ============ Agent Apps ============

/**
 * 获取本地已安装的 Agent 应用列表
 */
export async function listLocalAgentApps(): Promise<AgentInfo[]> {
  return apiCall<AgentInfo[]>("list_local_agent_apps");
}

/**
 * 刷新 Agent 应用 - 清除缓存并重新扫描文件系统
 */
export async function refreshAgentApps(): Promise<AgentInfo[]> {
  return apiCall<AgentInfo[]>("refresh_agent_apps");
}

/**
 * 获取所有 Agent 应用（内置 + 用户自定义）
 */
export async function listAllAgentApps(): Promise<AgentAppDetail[]> {
  return apiCall<AgentAppDetail[]>("list_all_agent_apps");
}

/**
 * 获取内置 Agent 应用列表
 */
export async function listInternalAgentApps(): Promise<AgentAppDetail[]> {
  return apiCall<AgentAppDetail[]>("list_internal_agent_apps");
}

/**
 * 获取用户自定义 Agent 应用列表
 */
export async function listUserAgentApps(): Promise<AgentAppDetail[]> {
  return apiCall<AgentAppDetail[]>("list_user_agent_apps");
}

/**
 * 添加用户自定义 Agent 应用
 */
export async function addAgentApp(
  displayName: string,
  globalPath: string,
  projectPath?: string
): Promise<AgentAppDetail> {
  return apiCall<AgentAppDetail>("add_agent_app", {
    displayName,
    globalPath,
    projectPath,
  });
}

/**
 * 移除用户自定义 Agent 应用
 */
export async function removeAgentApp(id: string): Promise<void> {
  return apiCall<void>("remove_agent_app", { id });
}

/**
 * 更新用户自定义 Agent 应用
 */
export async function updateAgentApp(
  id: string,
  displayName: string,
  globalPath: string,
  projectPath?: string
): Promise<AgentAppDetail> {
  return apiCall<AgentAppDetail>("update_agent_app", {
    id,
    displayName,
    globalPath,
    projectPath,
  });
}

/**
 * 验证 Agent 应用配置
 */
export async function validateAgentApp(
  displayName: string,
  globalPath: string
): Promise<ValidateResult> {
  return apiCall<ValidateResult>("validate_agent_app", { displayName, globalPath });
}
