<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { get } from "svelte/store";
  import { open } from "@tauri-apps/plugin-shell";
  import { Loader2 } from "@lucide/svelte";
  import PageHeader from "../../../../lib/components/PageHeader.svelte";
  import { parseMarkdown, renderMarkdownBody } from "../../../../lib/utils/markdown";
  import { t } from "../../../../lib/i18n";
  import {
    fetchSkillsByNames,
    listSkillDirectory,
    listSkills,
    openInFileManager,
    readSkillRelativeFile,
    readSkillFile,
    type LocalSkill,
    type RemoteSkill,
    type SkillDirectoryEntry,
  } from "../../../../lib/api/skills";

  type SkillType = "local" | "remote";

  let skillLoading = $state(true);
  let error = $state("");
  let skill = $state<LocalSkill | RemoteSkill | null>(null);
  let currentType = $state<SkillType>("local");
  let currentName = $state("");
  let contentLoading = $state(true);
  let contentError = $state("");
  let content = $state("");
  let hasFrontmatter = $state(false);
  let parsedFrontmatter = $state<Record<string, string>>({});
  let directoryOpen = $state(false);
  let directoryLoading = $state(false);
  let directoryError = $state("");
  let directoryEntries = $state<SkillDirectoryEntry[]>([]);
  let activeFilePath = $state("SKILL.md");
  let activeFileIsMarkdown = $state(true);

  const params = $derived($page.params);
  const query = $derived($page.url.searchParams);

  const parseType = (value: string): SkillType | null => {
    if (value === "local" || value === "remote") {
      return value;
    }
    return null;
  };

  const decodeName = (value: string): string => {
    try {
      return decodeURIComponent(value);
    } catch {
      return value;
    }
  };

  const parseLocalScope = (value: string | null): "global" | "project" =>
    value === "project" ? "project" : "global";

  const escapeHtml = (value: string) =>
    value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");

  const isMarkdownFile = (filePath: string) => /\.md$/i.test(filePath);

  const normalizeRelativePath = (baseFile: string, target: string): string | null => {
    const withoutHash = target.split("#")[0]?.split("?")[0] ?? "";
    if (!withoutHash) return null;
    if (withoutHash.startsWith("/")) {
      return withoutHash.replace(/^\/+/, "");
    }

    const baseSegments = baseFile.split("/").slice(0, -1);
    const segments = withoutHash.split("/");
    const resolved = [...baseSegments];

    for (const segment of segments) {
      if (!segment || segment === ".") continue;
      if (segment === "..") {
        if (resolved.length === 0) return null;
        resolved.pop();
        continue;
      }
      resolved.push(segment);
    }

    return resolved.join("/");
  };

  const formatDirectoryEntries = (entries: SkillDirectoryEntry[]): SkillDirectoryEntry[] =>
    [...entries].sort((a, b) => {
      if (a.path === b.path) {
        if (a.is_directory === b.is_directory) return 0;
        return a.is_directory ? -1 : 1;
      }
      return a.path.localeCompare(b.path, undefined, { sensitivity: "base" });
    });

  const getEntryDepth = (entryPath: string) => entryPath.split("/").length - 1;
  const getEntryName = (entryPath: string) => entryPath.split("/").at(-1) || entryPath;

  const getSkillRootPath = () => {
    if (!skill || !("installed_agent_apps" in skill)) return null;
    return skill.root_folder || skill.installed_agent_apps[0]?.skill_folder || null;
  };

  const buildGitHubUrl = (
    url: string,
    path: string | null | undefined,
    relativePath?: string,
    branch = "main"
  ) => {
    if (!url || !url.includes("github.com")) return null;
    const match = url.match(/github\.com\/([^\/]+)\/([^\/]+)/);
    if (!match) return null;
    const [, owner, repo] = match;
    const cleanRepo = repo.replace(/\.git$/, "");
    const basePath = (path || "").replace(/^\/+|\/+$/g, "");
    if (!relativePath) {
      return basePath
        ? `https://github.com/${owner}/${cleanRepo}/tree/${branch}/${basePath}`
        : `https://github.com/${owner}/${cleanRepo}`;
    }
    const normalizedRelativePath = relativePath.replace(/^\/+/, "");
    const fullPath = basePath ? `${basePath}/${normalizedRelativePath}` : normalizedRelativePath;
    return `https://github.com/${owner}/${cleanRepo}/tree/${branch}/${fullPath}`;
  };

  const loadSkill = async () => {
    skillLoading = true;
    error = "";
    skill = null;
    directoryEntries = [];
    directoryError = "";
    directoryOpen = false;

    const type = parseType(params.type);
    const name = decodeName(params.name);

    if (!type) {
      error = "Invalid skill type";
      skillLoading = false;
      return;
    }

    currentType = type;
    currentName = name;

    try {
      if (type === "local") {
        const scope = parseLocalScope(query.get("scope"));
        const projectPath = scope === "project" ? query.get("projectPath") : null;
        const localSkills = await listSkills(scope, projectPath);
        skill = localSkills.find((item) => item.name === name) ?? null;
      } else {
        const remoteSkills = await fetchSkillsByNames([name]);
        skill = remoteSkills.find((item) => item.name === name) ?? remoteSkills[0] ?? null;
      }

      if (!skill) {
        error = "Skill not found";
      }
    } catch (err) {
      error = String(err);
    } finally {
      skillLoading = false;
    }
  };

  const resolveLocalPath = (relativePath: string) => {
    if (!skill || !("installed_agent_apps" in skill)) return null;
    const dirPath = skill.root_folder || skill.installed_agent_apps[0]?.skill_folder;
    if (!dirPath) return null;
    if (relativePath.startsWith("/")) {
      return dirPath + relativePath;
    }
    const parts = dirPath.split("/");
    const relParts = relativePath.split("/");
    for (const part of relParts) {
      if (part === "..") {
        parts.pop();
      } else if (part !== ".") {
        parts.push(part);
      }
    }
    return parts.join("/");
  };

  const loadDirectory = async () => {
    if (!skill) return [];
    directoryLoading = true;
    directoryError = "";

    try {
      if (currentType === "local") {
        const localPath = getSkillRootPath();
        if (!localPath) throw new Error("Skill path is missing");
        const entries = await listSkillDirectory(localPath);
        const formatted = formatDirectoryEntries(entries);
        directoryEntries = formatted;
        return formatted;
      } else {
        if (!("url" in skill) || !skill.url) {
          throw new Error("Skill source URL is missing");
        }
        const repoMatch = skill.url.match(/github\.com\/([^\/]+)\/([^\/]+)/);
        if (!repoMatch) {
          throw new Error("Unable to resolve GitHub repository");
        }
        const [, owner, repo] = repoMatch;
        const cleanRepo = repo.replace(/\.git$/, "");
        const branch = skill.branch || "main";
        const response = await fetch(
          `https://api.github.com/repos/${owner}/${cleanRepo}/git/trees/${encodeURIComponent(branch)}?recursive=1`
        );
        if (!response.ok) {
          throw new Error(`Failed to fetch tree: ${response.status} ${response.statusText}`);
        }
        const payload = await response.json();
        const basePath = (skill.path || "").replace(/^\/+|\/+$/g, "");
        const prefix = basePath ? `${basePath}/` : "";

        const files: SkillDirectoryEntry[] = Array.isArray(payload.tree)
          ? payload.tree
              .filter(
                (item: { path?: string; type?: string }) =>
                  item.type === "blob" &&
                  typeof item.path === "string" &&
                  (basePath ? item.path.startsWith(prefix) : true)
              )
              .map((item: { path: string }) => ({
                path: basePath ? item.path.slice(prefix.length) : item.path,
                is_directory: false,
              }))
          : [];

        const dirs = new Set<string>();
        for (const file of files) {
          const parts = file.path.split("/");
          for (let i = 1; i < parts.length; i += 1) {
            dirs.add(parts.slice(0, i).join("/"));
          }
        }

        const formatted = formatDirectoryEntries([
          ...[...dirs].map((path) => ({ path, is_directory: true })),
          ...files,
        ]);
        directoryEntries = formatted;
        return formatted;
      }
    } catch (err) {
      directoryError = String(err);
      directoryEntries = [];
      return [];
    } finally {
      directoryLoading = false;
    }
  };

  const loadContent = async (filePath = "SKILL.md") => {
    if (!skill) return;
    activeFilePath = filePath;
    activeFileIsMarkdown = isMarkdownFile(filePath);
    contentLoading = true;
    contentError = "";
    try {
      let rawContent = "";
      if (currentType === "local") {
        const localPath = getSkillRootPath();
        if (!localPath) {
          throw new Error("Skill path is missing");
        }
        if (filePath === "SKILL.md") {
          rawContent = await readSkillFile(localPath);
        } else {
          rawContent = await readSkillRelativeFile(localPath, filePath);
        }
      } else {
        if (!("url" in skill)) {
          throw new Error("Skill source URL is missing");
        }
        const repoMatch = skill.url?.match(/github\.com\/([^\/]+)\/([^\/]+)/);
        if (!repoMatch) {
          throw new Error("Unable to construct raw URL for this skill");
        }
        const [, owner, repo] = repoMatch;
        const cleanRepo = repo.replace(/\.git$/, "");
        const branch = skill.branch || "main";
        const basePath = (skill.path || "").replace(/^\/+|\/+$/g, "");
        const fullPath = basePath ? `${basePath}/${filePath}` : filePath;
        const rawUrl = `https://raw.githubusercontent.com/${owner}/${cleanRepo}/${encodeURIComponent(branch)}/${fullPath}`;
        const response = await fetch(rawUrl);
        if (!response.ok) {
          throw new Error(`Failed to fetch: ${response.status} ${response.statusText}`);
        }
        rawContent = await response.text();
      }
      if (isMarkdownFile(filePath)) {
        const parsed = parseMarkdown(rawContent);
        parsedFrontmatter = parsed.frontmatter as Record<string, string>;
        hasFrontmatter = parsed.hasFrontmatter;
        content = parsed.content;
      } else {
        parsedFrontmatter = {};
        hasFrontmatter = false;
        content = rawContent;
      }
    } catch (err) {
      contentError = String(err);
      parsedFrontmatter = {};
      hasFrontmatter = false;
      content = "";
    } finally {
      contentLoading = false;
    }
  };

  const handleBack = () => {
    const returnTo = get(page).url.searchParams.get("returnTo");
    if (returnTo) {
      goto(decodeURIComponent(returnTo));
      return;
    }
    window.history.back();
  };

  const handleDetailAction = async () => {
    if (!skill) return;

    if (currentType === "remote" && "url" in skill && skill.url) {
      const fullUrl = buildGitHubUrl(skill.url, skill.path || "", undefined, skill.branch || "main") || skill.url;
      await open(fullUrl);
      return;
    }

    if (currentType === "local" && "installed_agent_apps" in skill) {
      const localPath = skill.root_folder || skill.installed_agent_apps[0]?.skill_folder;
      if (localPath) {
        await openInFileManager(localPath);
      }
    }
  };

  const retryContent = () => {
    loadContent(activeFilePath).catch(console.error);
  };

  function markdownInteractions(node: HTMLElement) {
    const handleClick = async (event: MouseEvent) => {
      const target = event.target as HTMLElement;
      const button = target.closest(".markdown-code-block__copy") as HTMLElement | null;
      if (button) {
        event.preventDefault();
        event.stopPropagation();
        const block = button.closest(".markdown-code-block");
        const codeElement = block?.querySelector("code");
        const codeContent = codeElement?.textContent ?? "";
        if (!codeContent) return;
        try {
          await navigator.clipboard.writeText(codeContent);
        } catch {
          const textarea = document.createElement("textarea");
          textarea.value = codeContent;
          textarea.setAttribute("readonly", "");
          textarea.style.position = "absolute";
          textarea.style.left = "-9999px";
          document.body.appendChild(textarea);
          textarea.select();
          try {
            document.execCommand("copy");
          } catch {
            // noop
          }
          document.body.removeChild(textarea);
        }
        button.classList.add("copied");
        const timerId = button.dataset.copyTimeout;
        if (timerId) window.clearTimeout(Number(timerId));
        const timeoutHandle = window.setTimeout(() => {
          button.classList.remove("copied");
          delete button.dataset.copyTimeout;
        }, 1500);
        button.dataset.copyTimeout = String(timeoutHandle);
        return;
      }

      const link = target.closest("a[href]");
      if (!link || !skill) return;
      const href = link.getAttribute("href");
      if (href?.startsWith("http://") || href?.startsWith("https://")) {
        event.preventDefault();
        event.stopPropagation();
        await open(href);
        return;
      }
      if (!href || href.startsWith("#")) return;
      event.preventDefault();
      event.stopPropagation();

      const targetPath = normalizeRelativePath(activeFilePath, href);
      if (targetPath && directoryEntries.some((entry) => !entry.is_directory && entry.path === targetPath)) {
        await loadContent(targetPath);
        return;
      }

      if (currentType === "local") {
        const localPath = resolveLocalPath(href);
        if (localPath) await openInFileManager(localPath);
      } else if ("url" in skill && skill.url) {
        const fullUrl = buildGitHubUrl(skill.url, skill.path || "", href, skill.branch || "main");
        if (fullUrl) await open(fullUrl);
      }
    };

    node.addEventListener("click", handleClick);
    return {
      destroy() {
        node.removeEventListener("click", handleClick);
      },
    };
  }

  $effect(() => {
    params.type;
    params.name;
    query.get("scope");
    query.get("projectPath");
    loadSkill().catch(console.error);
  });

  $effect(() => {
    if (skill) {
      loadDirectory()
        .then((entries) => {
          if (entries.some((entry) => !entry.is_directory && entry.path === "SKILL.md")) {
            return loadContent("SKILL.md");
          }
          const firstFile = entries.find((entry) => !entry.is_directory)?.path;
          return loadContent(firstFile || "SKILL.md");
        })
        .catch(console.error);
    }
  });
</script>

<div class="bg-base-100 text-base-content flex h-screen flex-col overflow-hidden">
  <PageHeader
    currentView="detail"
    activeTab="local"
    skillName={currentName}
    hasUpdate={false}
    onChangeTab={() => {}}
    onAddSkill={() => {}}
    onOpenUpdate={() => {}}
    onOpenSettings={() => {}}
    onBack={handleBack}
    onDetailAction={handleDetailAction}
    onOpenCatalog={() => {
      if (!directoryLoading && !skillLoading) directoryOpen = true;
    }}
    onRefreshAgentApps={() => {}}
  />

  {#if directoryOpen}
    <button
      class="bg-overlay fixed inset-0 z-40 border-0 p-0"
      onclick={() => {
        directoryOpen = false;
      }}
      type="button"
      aria-label={$t("detail.closeCatalog")}
    ></button>
    <aside class="border-base-300 bg-base-100 fixed right-0 top-0 z-50 h-full w-80 border-l shadow-2xl">
      <div class="border-base-300 flex items-center justify-between border-b px-4 py-3">
        <h2 class="text-base-content text-sm font-medium">{$t("detail.catalog")}</h2>
        <button
          class="border-base-300 text-base-content hover:bg-base-200 rounded-lg border px-2 py-1 text-xs transition"
          onclick={() => {
            directoryOpen = false;
          }}
          type="button"
        >
          {$t("detail.closeCatalog")}
        </button>
      </div>
      <div class="h-[calc(100%-57px)] overflow-y-auto p-2">
        {#if directoryLoading}
          <div class="text-base-content-muted flex items-center justify-center py-8 text-sm">
            <Loader2 size={18} class="animate-spin" />
          </div>
        {:else if directoryError}
          <p class="text-error px-2 py-3 text-sm">{directoryError}</p>
        {:else}
          {#each directoryEntries as entry (entry.path)}
            <button
              class="w-full rounded-lg px-2 py-1.5 text-left text-sm transition {entry.is_directory
                ? 'text-base-content-muted cursor-default'
                : entry.path === activeFilePath
                  ? 'bg-base-200 text-base-content'
                  : 'text-base-content hover:bg-base-200'}"
              style={`padding-left:${getEntryDepth(entry.path) * 16 + 12}px;`}
              onclick={() => {
                if (entry.is_directory) return;
                directoryOpen = false;
                loadContent(entry.path).catch(console.error);
              }}
              disabled={entry.is_directory}
              type="button"
            >
              {getEntryName(entry.path)}
            </button>
          {/each}
        {/if}
      </div>
    </aside>
  {/if}

  <main class="flex-1 overflow-y-auto">
    <div class="mx-auto max-w-6xl px-6 py-6">
      {#if skillLoading}
        <div class="flex items-center justify-center py-12">
          <Loader2 size={32} class="text-base-content-muted animate-spin" />
        </div>
      {:else if error}
        <div class="border-error-border bg-base-100 rounded-2xl border p-6 text-center">
          <p class="text-error">{error}</p>
        </div>
      {:else if skill}
        <div class="mx-auto max-w-4xl">
          {#if contentLoading}
            <div class="flex items-center justify-center py-12">
              <Loader2 size={32} class="text-base-content-muted animate-spin" />
            </div>
          {:else if contentError}
            <div class="border-error-border bg-base-100 rounded-2xl border p-6 text-center">
              <p class="text-error">{contentError}</p>
              <button
                class="bg-primary text-primary-content hover:bg-primary-hover mt-4 rounded-lg px-4 py-2 text-sm transition"
                onclick={retryContent}
              >
                {$t("detail.retry")}
              </button>
            </div>
          {:else}
            {#if hasFrontmatter}
              <div class="border-base-300 bg-base-200 mb-6 overflow-hidden rounded-xl border">
                <div class="flex flex-col gap-3 p-4">
                  {#if parsedFrontmatter.description}
                    <div class="flex flex-col gap-1">
                      <span class="text-base-content text-sm leading-6"
                        >{parsedFrontmatter.description}</span
                      >
                    </div>
                  {/if}
                </div>
              </div>
            {/if}
            {#if activeFileIsMarkdown}
              <div class="markdown-content" use:markdownInteractions>
                {@html renderMarkdownBody(content)}
              </div>
            {:else}
              <pre class="code-block"><code>{@html escapeHtml(content)}</code></pre>
            {/if}
          {/if}
        </div>
      {/if}
    </div>
  </main>
</div>
