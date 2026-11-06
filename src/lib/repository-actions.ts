/**
 * Pure logic of the "Repository actions" menu of a repository group in the Library
 * (see docs/decisions/0005-library-repository-group-actions.md).
 */

import type { HubSkillView, SourcePullResult, SourceUpdate } from "./api/hub";
import type { DetectedSkill } from "./api/skills";

type SkillLike = Pick<HubSkillView, "id" | "source" | "hubState">;

/** The branch that every skill of the group has, or `null` for the default branch. */
export function repositoryImportBranch(skills: Pick<HubSkillView, "source">[]): string | null {
  let branch: string | null = null;
  for (const { source } of skills) {
    const own = source.type === "github" ? source.branch?.trim() || null : null;
    if (!own || (branch !== null && own !== branch)) return null;
    branch = own;
  }
  return branch;
}

/** `skills/foo/SKILL.md` and `./skills/foo/SKILL.md` -> `skills/foo`; `SKILL.md` -> ``. */
function skillDir(skillPath: string): string {
  return skillPath
    .trim()
    .replace(/^\.\//, "")
    .replace(/\/?SKILL\.md$/, "")
    .replace(/\/+$/, "");
}

/**
 * Whether the library has a skill from the same repository folder. Same rule as the
 * backend `same_location`: the branch counts only when both sides have one.
 */
export function isImportedGithubSkill(
  library: Pick<HubSkillView, "source">[],
  repo: string,
  detected: Pick<DetectedSkill, "skill_path" | "branch">
): boolean {
  const wantedRepo = repo.trim().toLowerCase();
  const wantedDir = skillDir(detected.skill_path);
  const wantedBranch = detected.branch?.trim() || null;
  return library.some(({ source }) => {
    if (source.type !== "github") return false;
    if (source.repo.trim().toLowerCase() !== wantedRepo) return false;
    if (skillDir(source.skillPath) !== wantedDir) return false;
    const branch = source.branch?.trim() || null;
    return !branch || !wantedBranch || branch === wantedBranch;
  });
}

/** Skills with an update whose hub copy has local changes; an update would overwrite them. */
export function localChangeIds(
  check: Pick<SourceUpdate, "id" | "updateAvailable">[],
  skills: SkillLike[]
): string[] {
  const states = new Map(skills.map((skill) => [skill.id, skill.hubState]));
  return check
    .filter((item) => item.updateAvailable)
    .filter((item) => {
      const state = states.get(item.id);
      return state === "modified" || state === "name_mismatch";
    })
    .map((item) => item.id);
}

export interface GroupUpdateReport {
  updated: number;
  upToDate: number;
  /** Not updated because the hub copy has local changes. */
  skipped: { id: string; name: string }[];
  failed: { id: string; name: string; error: string }[];
}

/**
 * One report for a group update: the forced check, the skills the user chose to skip,
 * and the batch pull results.
 */
export function summarizeGroupUpdate(
  check: Pick<SourceUpdate, "id" | "name" | "updateAvailable" | "error">[],
  skippedIds: string[],
  pulled: SourcePullResult[]
): GroupUpdateReport {
  const report: GroupUpdateReport = { updated: 0, upToDate: 0, skipped: [], failed: [] };
  for (const item of check) {
    if (item.error) {
      report.failed.push({ id: item.id, name: item.name, error: item.error });
    } else if (!item.updateAvailable) {
      report.upToDate += 1;
    } else if (skippedIds.includes(item.id)) {
      report.skipped.push({ id: item.id, name: item.name });
    }
  }
  for (const result of pulled) {
    switch (result.status) {
      case "updated":
        report.updated += 1;
        break;
      case "skipped":
        report.skipped.push({ id: result.id, name: result.name });
        break;
      default:
        report.failed.push({ id: result.id, name: result.name, error: result.message ?? "" });
    }
  }
  return report;
}

/** A clean update gets a toast; a skipped or failed skill needs the result dialog. */
export function reportNeedsDialog(report: GroupUpdateReport): boolean {
  return report.skipped.length > 0 || report.failed.length > 0;
}
