import type { ScopeRef } from "$lib/api/hub";

export type SidebarActiveKey = "library" | "instructions" | "projects" | "market" | "settings";

export type LibraryFilter = "all" | "changed" | "uninstalled";

export type AppLocation = {
  activeKey: SidebarActiveKey;
  /** Selected skill in the library (`?skill=`). */
  skill: string | null;
  /** Selected template on the instructions page (`?id=`). */
  instruction: string | null;
  /** Selected agent file on the instructions page (`?file=`). */
  instructionFile: string | null;
  /** Selected install scope on the projects page (`?scope=user` or `?project=<path>`). */
  scope: ScopeRef | null;
  filter: LibraryFilter;
};

const LIBRARY_FILTERS: LibraryFilter[] = ["all", "changed", "uninstalled"];

const isSettingsPath = (pathname: string) =>
  pathname === "/settings" || pathname.startsWith("/agent-apps");

const isMarketPath = (pathname: string) =>
  pathname === "/market" || pathname.startsWith("/skills/remote/");

const isProjectsPath = (pathname: string) => pathname.startsWith("/projects");

const isInstructionsPath = (pathname: string) => pathname.startsWith("/instructions");

export const parseLibraryFilter = (value: string | null): LibraryFilter =>
  LIBRARY_FILTERS.includes(value as LibraryFilter) ? (value as LibraryFilter) : "all";

const parseScope = (url: URL): ScopeRef | null => {
  const projectPath = url.searchParams.get("project");
  if (projectPath) return { scope: "project", projectPath };
  if (url.searchParams.get("scope") === "user") return { scope: "user", projectPath: null };
  return null;
};

export const getAppLocation = (url: URL): AppLocation => {
  const scope = parseScope(url);

  let activeKey: SidebarActiveKey = "library";
  if (isSettingsPath(url.pathname)) {
    activeKey = "settings";
  } else if (isMarketPath(url.pathname)) {
    activeKey = "market";
  } else if (isProjectsPath(url.pathname)) {
    activeKey = "projects";
  } else if (isInstructionsPath(url.pathname)) {
    activeKey = "instructions";
  }

  return {
    activeKey,
    skill: url.searchParams.get("skill") || null,
    instruction: url.searchParams.get("id") || null,
    instructionFile: url.searchParams.get("file") || null,
    scope,
    filter: parseLibraryFilter(url.searchParams.get("filter")),
  };
};

export const buildLibraryHref = (
  options: { skill?: string | null; filter?: LibraryFilter } = {}
): string => {
  const params = new URLSearchParams();
  if (options.filter && options.filter !== "all") params.set("filter", options.filter);
  if (options.skill) params.set("skill", options.skill);
  const query = params.toString();
  return query ? `/?${query}` : "/";
};

/** The instructions page with a template (`id`) or an agent file (`file`) selected. */
export const buildInstructionsHref = (
  options: { id?: string | null; file?: string | null; filter?: LibraryFilter } = {}
): string => {
  const params = new URLSearchParams();
  if (options.filter && options.filter !== "all") params.set("filter", options.filter);
  if (options.id) params.set("id", options.id);
  else if (options.file) params.set("file", options.file);
  const query = params.toString();
  return query ? `/instructions?${query}` : "/instructions";
};

/** The projects page, optionally with one scope selected. */
export const buildScopeHref = (scope?: ScopeRef | null): string => {
  if (!scope) return "/projects";
  if (scope.scope === "user") return "/projects?scope=user";
  return `/projects?project=${encodeURIComponent(scope.projectPath ?? "")}`;
};

export const buildMarketHref = (): string => "/market";
