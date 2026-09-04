/**
 * Agents grouped by the directory they read. Several agents share `~/.agents/skills` (and
 * `.agents/skills` inside a project), so installing or uninstalling one of them affects
 * every agent of that group.
 */

import type { InstallScope } from "./api/hub";
import type { AgentInfo } from "./api/skills";

export interface AgentGroup {
  key: string;
  /** Skills directory shared by the agents of this group. */
  path: string;
  agents: AgentInfo[];
}

export const agentScopePath = (agent: AgentInfo, scope: InstallScope): string | null =>
  (scope === "user" ? agent.global_path : agent.project_path) || null;

export function groupAgents(agents: AgentInfo[], scope: InstallScope): AgentGroup[] {
  const groups = new Map<string, AgentGroup>();
  for (const agent of agents) {
    const path = agentScopePath(agent, scope);
    if (!path) continue;
    const key = path.toLowerCase();
    const group = groups.get(key);
    if (group) group.agents.push(agent);
    else groups.set(key, { key, path, agents: [agent] });
  }
  return [...groups.values()];
}

export const agentNames = (agents: AgentInfo[]): string =>
  agents.map((agent) => agent.display_name).join(", ");
