export type SidebarActiveKey = "library" | "user" | "market" | "settings" | `project:${string}`;

export type LibraryFilter = "all" | "changed" | "uninstalled";

/** Which slice of the hub the library shows: everything, or one install scope. */
export type LibraryScope =
  | { kind: "library" }
  | { kind: "user" }
  | { kind: "project"; projectPath: string };

export type AppLocation = {
  activeKey: SidebarActiveKey;
  /** Selected skill in the library (`?skill=`). */
  skill: string | null;
  scope: LibraryScope;
  filter: LibraryFilter;
};

const LIBRARY_FILTERS: LibraryFilter[] = ["all", "changed", "uninstalled"];

const isSettingsPath = (pathname: string) =>
  pathname === "/settings" || pathname.startsWith("/agent-apps");

const isMarketPath = (pathname: string) =>
  pathname === "/market" || pathname.startsWith("/skills/remote/");

export const projectScopeKey = (projectPath: string): `project:${string}` =>
  `project:${encodeURIComponent(projectPath)}`;

export const parseLibraryFilter = (value: string | null): LibraryFilter =>
  LIBRARY_FILTERS.includes(value as LibraryFilter) ? (value as LibraryFilter) : "all";

const parseScope = (url: URL): LibraryScope => {
  const projectPath = url.searchParams.get("project");
  if (projectPath) return { kind: "project", projectPath };
  if (url.searchParams.get("scope") === "user") return { kind: "user" };
  return { kind: "library" };
};

export const getAppLocation = (url: URL): AppLocation => {
  const skill = url.searchParams.get("skill");
  const filter = parseLibraryFilter(url.searchParams.get("filter"));
  const scope = parseScope(url);

  let activeKey: SidebarActiveKey = "library";
  if (isSettingsPath(url.pathname)) {
    activeKey = "settings";
  } else if (isMarketPath(url.pathname)) {
    activeKey = "market";
  } else if (scope.kind === "project") {
    activeKey = projectScopeKey(scope.projectPath);
  } else if (scope.kind === "user") {
    activeKey = "user";
  }

  return { activeKey, skill: skill || null, scope, filter };
};

export const buildLibraryHref = (
  options: { skill?: string | null; scope?: LibraryScope; filter?: LibraryFilter } = {}
): string => {
  const params = new URLSearchParams();
  const scope = options.scope ?? { kind: "library" };
  if (scope.kind === "project") params.set("project", scope.projectPath);
  if (scope.kind === "user") params.set("scope", "user");
  if (options.filter && options.filter !== "all") params.set("filter", options.filter);
  if (options.skill) params.set("skill", options.skill);
  const query = params.toString();
  return query ? `/?${query}` : "/";
};

export const buildMarketHref = (): string => "/market";
