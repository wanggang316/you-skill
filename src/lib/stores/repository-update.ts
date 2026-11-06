import { get } from "svelte/store";
import { checkSourceUpdates, pullGithubSources, type SourcePullResult } from "../api/hub";
import { t } from "../i18n";
import {
  localChangeIds,
  needsPull,
  summarizeGroupUpdate,
  type GroupUpdateReport,
} from "../repository-actions";
import { hubSkills, refreshHub, skillLabel } from "./hub";
import { askForce } from "./modals";

/**
 * Update every skill of a repository group: a forced source check, one question about local
 * changes, one batch pull (one download for each repository branch), then one report.
 * Install targets are not pushed.
 */
export async function updateRepositoryGroup(ids: string[]): Promise<GroupUpdateReport> {
  let check;
  try {
    check = await checkSourceUpdates(ids);
  } catch (error) {
    return failAll(ids, error);
  }
  await refreshHub().catch(console.error);

  const groupSkills = get(hubSkills).filter((skill) => ids.includes(skill.id));
  const local = localChangeIds(check, groupSkills);
  let overwrite = false;
  if (local.length > 0) {
    const translate = get(t);
    const label = get(skillLabel);
    overwrite = await askForce(
      local.map((id) => label(id)),
      {
        title: translate("library.repoUpdate.forceTitle"),
        description: translate("library.repoUpdate.forceDescription"),
        confirm: translate("library.repoUpdate.overwriteAll"),
        cancel: translate("library.repoUpdate.skipAll"),
      }
    );
  }

  const pullIds = check
    .filter(needsPull)
    .map((item) => item.id)
    .filter((id) => overwrite || !local.includes(id));
  // The confirmed hub content: a skill edited again before the write is skipped.
  const forced = overwrite
    ? groupSkills
        .filter((skill) => local.includes(skill.id) && skill.hubHash)
        .map((skill) => ({ id: skill.id, hubHash: skill.hubHash as string }))
    : [];
  let pulled: SourcePullResult[] = [];
  if (pullIds.length > 0) {
    try {
      pulled = await pullGithubSources(pullIds, forced);
    } catch (error) {
      pulled = pullIds.map((id) => failed(id, error));
    }
    await refreshHub().catch(console.error);
  }
  return summarizeGroupUpdate(
    check,
    overwrite ? [] : local,
    pulled,
    forced.map((item) => item.id)
  );
}

function failed(id: string, error: unknown): SourcePullResult {
  return { id, name: get(skillLabel)(id), status: "failed", message: String(error) };
}

function failAll(ids: string[], error: unknown): GroupUpdateReport {
  return summarizeGroupUpdate(
    [],
    [],
    ids.map((id) => failed(id, error))
  );
}
