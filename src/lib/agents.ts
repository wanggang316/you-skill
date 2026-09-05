/**
 * Agents grouped by the directory they read. Several agents share `~/.agents/skills` (and
 * `.agents/skills` inside a project), so installing or uninstalling one of them affects
 * every agent of that group. A group is presented as a single agent: its primary member,
 * the first one in registry order ("Agents (shared)" for the shared directory), and the
 * other members are only revealed on hover.
 */

import type { InstallScope } from "./api/hub";
import type { AgentInfo } from "./api/skills";

export interface AgentGroup {
  key: string;
  /** Skills directory shared by the agents of this group. */
  path: string;
  /** Members in registry order; the first one represents the group. */
  agents: AgentInfo[];
}

export interface AgentLabel {
  id: string;
  name: string;
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

/**
 * Labels for a set of agent ids in registry order, so the first label is the agent that
 * represents the directory. Ids without a registered app keep their id as the name.
 */
export function resolveAgents(ids: string[], agents: Map<string, AgentInfo>): AgentLabel[] {
  const known: AgentLabel[] = [];
  for (const agent of agents.values()) {
    if (ids.includes(agent.id)) known.push({ id: agent.id, name: agent.display_name });
  }
  const unknown = ids.filter((id) => !agents.has(id)).map((id) => ({ id, name: id }));
  return [...known, ...unknown];
}

export const agentNames = (agents: { name: string }[] | AgentInfo[]): string =>
  agents.map((agent) => ("display_name" in agent ? agent.display_name : agent.name)).join(", ");
