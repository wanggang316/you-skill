export type SidebarActiveKey = "library" | "global" | "settings" | `project:${string}`;

export type SkillsLocation = {
  activeKey: SidebarActiveKey;
  tab: "local" | "remote";
  scope: "global" | "project";
  projectPath: string | null;
};

const isSettingsPath = (pathname: string) =>
  pathname === "/settings" || pathname.startsWith("/agent-apps");

const isRemoteSkillPath = (pathname: string) => pathname.startsWith("/skills/remote/");

export const projectScopeKey = (projectPath: string): `project:${string}` =>
  `project:${encodeURIComponent(projectPath)}`;

export const getSkillsLocation = (url: URL): SkillsLocation => {
  const projectPath = url.searchParams.get("projectPath");
  const isProjectScope = url.searchParams.get("scope") === "project" && Boolean(projectPath);
  const tab =
    isRemoteSkillPath(url.pathname) ||
    (url.pathname === "/" && url.searchParams.get("tab") === "remote")
      ? "remote"
      : "local";
  const scope = isProjectScope ? "project" : "global";

  let activeKey: SidebarActiveKey = "global";
  if (isSettingsPath(url.pathname)) {
    activeKey = "settings";
  } else if (tab === "remote") {
    activeKey = "library";
  } else if (scope === "project" && projectPath) {
    activeKey = projectScopeKey(projectPath);
  }

  return {
    activeKey,
    tab,
    scope,
    projectPath: scope === "project" ? projectPath : null,
  };
};

export const buildSkillsHref = (
  tab: "local" | "remote",
  projectPath: string | null = null
): string => {
  const params = new URLSearchParams({ tab });
  if (tab === "local" && projectPath) {
    params.set("scope", "project");
    params.set("projectPath", projectPath);
  } else {
    params.set("scope", "global");
  }
  return `/?${params.toString()}`;
};
