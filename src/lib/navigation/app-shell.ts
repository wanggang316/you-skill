export type SidebarActiveKey = "library" | "market" | "settings" | `project:${string}`;

export type LibraryFilter = "all" | "changed" | "uninstalled" | "user";

export type AppLocation = {
  activeKey: SidebarActiveKey;
  /** Selected skill in the library (`?skill=`). */
  skill: string | null;
  /** Project filter for the library (`?project=`). */
  projectPath: string | null;
  filter: LibraryFilter;
};

const LIBRARY_FILTERS: LibraryFilter[] = ["all", "changed", "uninstalled", "user"];

const isSettingsPath = (pathname: string) =>
  pathname === "/settings" || pathname.startsWith("/agent-apps");

const isMarketPath = (pathname: string) =>
  pathname === "/market" || pathname.startsWith("/skills/remote/");

export const projectScopeKey = (projectPath: string): `project:${string}` =>
  `project:${encodeURIComponent(projectPath)}`;

export const parseLibraryFilter = (value: string | null): LibraryFilter =>
  LIBRARY_FILTERS.includes(value as LibraryFilter) ? (value as LibraryFilter) : "all";

export const getAppLocation = (url: URL): AppLocation => {
  const projectPath = url.searchParams.get("project");
  const skill = url.searchParams.get("skill");
  const filter = parseLibraryFilter(url.searchParams.get("filter"));

  let activeKey: SidebarActiveKey = "library";
  if (isSettingsPath(url.pathname)) {
    activeKey = "settings";
  } else if (isMarketPath(url.pathname)) {
    activeKey = "market";
  } else if (projectPath) {
    activeKey = projectScopeKey(projectPath);
  }

  return {
    activeKey,
    skill: skill || null,
    projectPath: projectPath || null,
    filter,
  };
};

export const buildLibraryHref = (
  options: { skill?: string | null; projectPath?: string | null; filter?: LibraryFilter } = {}
): string => {
  const params = new URLSearchParams();
  if (options.projectPath) params.set("project", options.projectPath);
  if (options.filter && options.filter !== "all") params.set("filter", options.filter);
  if (options.skill) params.set("skill", options.skill);
  const query = params.toString();
  return query ? `/?${query}` : "/";
};

export const buildMarketHref = (): string => "/market";

export const buildSkillFilesHref = (name: string, returnTo: string): string =>
  `/skills/hub/${encodeURIComponent(name)}?returnTo=${encodeURIComponent(returnTo)}`;
