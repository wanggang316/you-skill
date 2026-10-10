/**
 * Library list groups: GitHub skills by repository owner, local skills by the folder they
 * were imported from, then the skills without a source.
 */

import type { HubSkillView } from "./api/hub";
import { baseName, parentDir } from "./scopes";

export type LibraryGroupKind = "github" | "folder" | "none";

export interface LibraryGroup {
  key: string;
  kind: LibraryGroupKind;
  /** Owner for GitHub, folder name for local groups; empty for skills without a source. */
  label: string;
  /** Full folder path of a local group, shown as a tooltip. */
  path: string | null;
  skills: HubSkillView[];
}

const KIND_ORDER: Record<LibraryGroupKind, number> = { github: 0, folder: 1, none: 2 };

function groupOf(skill: HubSkillView): Omit<LibraryGroup, "skills"> {
  const source = skill.source;
  switch (source.type) {
    case "github": {
      const owner = source.repo.split("/")[0] || source.repo;
      return { key: `github:${owner.toLowerCase()}`, kind: "github", label: owner, path: null };
    }
    case "folder":
    case "zip": {
      // A folder source is the skill directory and a zip source the archive; both are
      // grouped by the folder that holds them.
      const folder = parentDir(source.path);
      return { key: `folder:${folder}`, kind: "folder", label: baseName(folder), path: folder };
    }
    default:
      return { key: "none", kind: "none", label: "", path: null };
  }
}

/** Group skills, keeping their order inside each group. */
export function groupLibrarySkills(skills: HubSkillView[]): LibraryGroup[] {
  const groups = new Map<string, LibraryGroup>();
  for (const skill of skills) {
    const head = groupOf(skill);
    const group = groups.get(head.key);
    if (group) {
      group.skills.push(skill);
    } else {
      groups.set(head.key, { ...head, skills: [skill] });
    }
  }
  return [...groups.values()].sort(
    (a, b) =>
      KIND_ORDER[a.kind] - KIND_ORDER[b.kind] ||
      a.label.localeCompare(b.label, undefined, { sensitivity: "base" }) ||
      (a.path ?? "").localeCompare(b.path ?? "")
  );
}

export function githubAvatarUrl(owner: string): string {
  return `https://github.com/${encodeURIComponent(owner)}.png?size=40`;
}
