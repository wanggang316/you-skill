import { get } from "svelte/store";
import {
  checkSourceUpdates,
  pullGithubSources,
  type SourcePull,
  type SourcePullResult,
} from "../api/hub";
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

  // Each pull carries the hub state the update was decided on: a hub copy that changes
  // before the write (an edit, a removed SKILL.md) is skipped, forced or not. A hub copy
  // without a hash (missing folder or SKILL.md) has no state to guard the write with, so
  // it is not pulled.
  const hubHashes = new Map(groupSkills.map((skill) => [skill.id, skill.hubHash ?? null]));
  const pullIds = check
    .filter(needsPull)
    .map((item) => item.id)
    .filter((id) => overwrite || !local.includes(id));
  const unavailable: SourcePullResult[] = pullIds
    .filter((id) => !hubHashes.get(id))
    .map((id) => ({
      id,
      name: get(skillLabel)(id),
      status: "skipped",
      reason: "hub_unavailable",
    }));
  const pulls: SourcePull[] = pullIds
    .filter((id) => hubHashes.get(id))
    .map((id) => ({ id, hubHash: hubHashes.get(id), force: local.includes(id) }));
  let pulled: SourcePullResult[] = [...unavailable];
  if (pulls.length > 0) {
    try {
      pulled.push(...(await pullGithubSources(pulls)));
    } catch (error) {
      pulled.push(...pulls.map((pull) => failed(pull.id, error)));
    }
    await refreshHub().catch(console.error);
  }
  return summarizeGroupUpdate(
    check,
    overwrite ? [] : local,
    pulled,
    pulls.filter((pull) => pull.force).map((pull) => pull.id)
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
