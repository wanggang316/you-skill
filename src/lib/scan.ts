/**
 * Scan results, read as "one skill name = one library copy + the folders holding it".
 *
 * The scan returns one item per folder, so the same skill name can come back several times.
 * The hub is keyed by name and holds a single copy, so at most one folder per name may be
 * written into it; the others are install locations, or nothing at all.
 */

import type { ScanItem } from "./api/hub";

export type ScanAction =
  /** Write this folder into the library, as a new skill or over the existing copy. */
  | "to_hub"
  /** Overwrite this folder with the library copy. */
  | "push"
  /** Leave both alone, just record the folder as a place the skill is installed. */
  | "register"
  | "ignore";

export interface ScanGroup {
  name: string;
  /** The library already holds a skill under this name. */
  inHub: boolean;
  items: ScanItem[];
}

export interface ScanSections {
  /** Names the library does not have yet. */
  fresh: ScanGroup[];
  /** Names the library already holds. */
  known: ScanGroup[];
  /** Folders with nothing to decide, listed for completeness. */
  inert: ScanItem[];
}

/** A folder inside an agent's skills directory can be recorded as an install location. */
export const canRegister = (item: ScanItem): boolean => Boolean(item.inAgentRoot);

function isActionable(item: ScanItem): boolean {
  if (item.status === "new" || item.status === "different") return true;
  // An identical copy is only worth listing when it can become an install location.
  return item.status === "identical" && canRegister(item);
}

export function groupScanItems(items: ScanItem[]): ScanSections {
  const groups = new Map<string, ScanGroup>();
  const inert: ScanItem[] = [];

  for (const item of items) {
    if (!isActionable(item)) {
      inert.push(item);
      continue;
    }
    const group = groups.get(item.name);
    if (group) group.items.push(item);
    else groups.set(item.name, { name: item.name, inHub: item.status !== "new", items: [item] });
  }

  const all = [...groups.values()];
  return {
    fresh: all.filter((group) => !group.inHub),
    known: all.filter((group) => group.inHub),
    inert,
  };
}

/** Copies an agent already reads are the likeliest to be the live version of a skill. */
function hubCopyRank(item: ScanItem): number {
  if (item.inAgentRoot?.scope === "user") return 0;
  if (item.inAgentRoot?.scope === "project") return 1;
  return 2;
}

/** The folder a new skill is taken from unless the user picks another one. */
export function preferredHubCopy(group: ScanGroup): ScanItem {
  return group.items.reduce((best, item) => (hubCopyRank(item) < hubCopyRank(best) ? item : best));
}

/**
 * New skills come in; everything already in the library is left as it is. Overwriting in
 * either direction is never the default — it needs a deliberate choice.
 */
export function defaultActions(sections: ScanSections): Record<string, ScanAction> {
  const actions: Record<string, ScanAction> = {};
  const fallback = (item: ScanItem) => (canRegister(item) ? "register" : "ignore");

  for (const group of sections.fresh) {
    const chosen = preferredHubCopy(group);
    for (const item of group.items) {
      actions[item.path] = item === chosen ? "to_hub" : fallback(item);
    }
  }
  for (const group of sections.known) {
    for (const item of group.items) actions[item.path] = fallback(item);
  }
  return actions;
}

/** The folder of this group that is set to become the library copy, if any. */
export function hubBound(group: ScanGroup, actions: Record<string, ScanAction>): ScanItem | null {
  return group.items.find((item) => actions[item.path] === "to_hub") ?? null;
}

export function actionsFor(
  group: ScanGroup,
  item: ScanItem,
  actions: Record<string, ScanAction>
): ScanAction[] {
  const out: ScanAction[] = [];
  // An identical copy has nothing to write in either direction.
  if (item.status !== "identical") out.push("to_hub");
  if (group.inHub && item.status === "different") out.push("push");
  // An install location needs a library copy to point at: for a skill the library does not
  // have yet, that means one of the other folders has to go in first.
  const bound = hubBound(group, actions);
  if (canRegister(item) && (group.inHub || (bound !== null && bound.path !== item.path))) {
    out.push("register");
  }
  out.push("ignore");
  return out;
}

/**
 * Set one folder's action, keeping the group coherent: at most one folder per skill name
 * goes into the library, and a new skill nobody takes in has no install locations either.
 */
export function withAction(
  actions: Record<string, ScanAction>,
  group: ScanGroup,
  item: ScanItem,
  action: ScanAction
): Record<string, ScanAction> {
  const next = { ...actions, [item.path]: action };
  if (action === "to_hub") {
    for (const other of group.items) {
      if (other.path !== item.path && next[other.path] === "to_hub") {
        next[other.path] = canRegister(other) ? "register" : "ignore";
      }
    }
    return next;
  }
  if (!group.inHub && !hubBound(group, next)) {
    for (const other of group.items) {
      if (next[other.path] === "register") next[other.path] = "ignore";
    }
  }
  return next;
}

/**
 * How a folder compares with the one going into the library. Only meaningful for a new
 * skill found in several places; once a name is in the library, each folder's own status
 * already says how it compares.
 */
export function copyRelation(
  group: ScanGroup,
  item: ScanItem,
  actions: Record<string, ScanAction>
): "same" | "differs" | null {
  if (group.inHub || group.items.length < 2) return null;
  const chosen = group.items.find((other) => actions[other.path] === "to_hub");
  if (!chosen || chosen.path === item.path || !item.hash || !chosen.hash) return null;
  return item.hash === chosen.hash ? "same" : "differs";
}
