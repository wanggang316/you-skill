/**
 * Pure logic of the "Repository actions" menu of a repository group in the Library
 * (see docs/decisions/0005-library-repository-group-actions.md).
 */

import type { HubSkillView, SourcePullResult, SourceUpdate } from "./api/hub";
import type { DetectedSkill } from "./api/skills";

type SkillLike = Pick<HubSkillView, "id" | "source" | "hubState">;

/** What Import of a repository group detects: `owner/repo` and the shared branch. */
export interface RepositoryImportTarget {
  repo: string;
  branch: string | null;
  /** The address shown in the import dialog. */
  url: string;
}

export function repositoryImportTarget(
  repo: string,
  branch: string | null
): RepositoryImportTarget {
  return { repo, branch, url: branch ? `https://github.com/${repo}/tree/${branch}` : repo };
}

/**
 * Arguments for GitHub detection. While the input is the address that Import filled in,
 * the branch goes separately: a branch with `/` cannot be read back from a `/tree/` URL.
 */
export function githubDetectArgs(
  input: string,
  target: RepositoryImportTarget | null
): { path: string; branch: string | null } {
  const path = input.trim();
  if (target && path === target.url) return { path: target.repo, branch: target.branch };
  return { path, branch: null };
}

/** The items of the "Repository actions" menu; each acts on the whole group. */
export type RepositoryAction = "import" | "install" | "update" | "remove";

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

type CheckItem = Pick<
  SourceUpdate,
  "id" | "updateAvailable" | "remoteSha" | "latestRemoteSha" | "error"
>;

/**
 * Whether a group update pulls the skill: it has an update, or its installed version is
 * unknown (no `remoteSha`, e.g. when the lookup failed at import), so the check cannot
 * tell. A pull records the version.
 */
export function needsPull(item: CheckItem): boolean {
  if (item.error) return false;
  return item.updateAvailable || (!item.remoteSha && !!item.latestRemoteSha);
}

/** Skills to pull whose hub copy has local changes; a pull would overwrite them. */
export function localChangeIds(check: CheckItem[], skills: SkillLike[]): string[] {
  const states = new Map(skills.map((skill) => [skill.id, skill.hubState]));
  return check
    .filter(needsPull)
    .filter((item) => {
      const state = states.get(item.id);
      return state === "modified" || state === "name_mismatch";
    })
    .map((item) => item.id);
}

/**
 * Why a skill was not updated: the user chose to skip its local changes, or its hub copy
 * changed after the user confirmed the overwrite.
 */
export type SkipReason = "local_changes" | "changed_after_confirmation";

export interface GroupUpdateReport {
  updated: number;
  upToDate: number;
  skipped: { id: string; name: string; reason: SkipReason }[];
  failed: { id: string; name: string; error: string }[];
}

/**
 * One report for a group update: the forced check, the skills the user chose to skip, the
 * batch pull results, and the skills pulled with a confirmed overwrite.
 */
export function summarizeGroupUpdate(
  check: (CheckItem & Pick<SourceUpdate, "name">)[],
  skippedIds: string[],
  pulled: SourcePullResult[],
  forcedIds: string[] = []
): GroupUpdateReport {
  const report: GroupUpdateReport = { updated: 0, upToDate: 0, skipped: [], failed: [] };
  for (const item of check) {
    if (item.error) {
      report.failed.push({ id: item.id, name: item.name, error: item.error });
    } else if (!needsPull(item)) {
      report.upToDate += 1;
    } else if (skippedIds.includes(item.id)) {
      report.skipped.push({ id: item.id, name: item.name, reason: "local_changes" });
    }
  }
  for (const result of pulled) {
    switch (result.status) {
      case "updated":
        report.updated += 1;
        break;
      case "skipped":
        // The pull skips a forced skill only when it changed after the confirmation.
        report.skipped.push({
          id: result.id,
          name: result.name,
          reason: forcedIds.includes(result.id) ? "changed_after_confirmation" : "local_changes",
        });
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
