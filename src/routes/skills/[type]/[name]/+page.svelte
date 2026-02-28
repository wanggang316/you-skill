<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { get } from "svelte/store";
  import { open } from "@tauri-apps/plugin-shell";
  import hljs from "highlight.js/lib/common";
  import { Loader2 } from "@lucide/svelte";
  import PageHeader from "$lib/components/PageHeader.svelte";
  import SkillDirectoryDrawer from "$lib/components/SkillDirectoryDrawer.svelte";
  import MarkdownPreview from "$lib/components/MarkdownPreview.svelte";
  import CodePreview from "$lib/components/CodePreview.svelte";
  import ImagePreview from "$lib/components/ImagePreview.svelte";
  import { parseMarkdown, renderMarkdownBody } from "$lib/utils/markdown";
  import { t } from "$lib/i18n";
  import {
    fetchSkillsByNames,
    listSkillDirectory,
    listSkills,
    openInFileManager,
    readSkillRelativeFileBytes,
    readSkillRelativeFile,
    readSkillFile,
    type LocalSkill,
    type RemoteSkill,
    type SkillDirectoryEntry,
  } from "$lib/api/skills";

  type SkillType = "local" | "remote";
  type FileViewMode = "markdown" | "code" | "image" | "unsupported";

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
  let directoryClosing = $state(false);
  let directoryLoading = $state(false);
  let directoryError = $state("");
  let directoryEntries = $state<SkillDirectoryEntry[]>([]);
  let activeFilePath = $state("SKILL.md");
  let fileViewMode = $state<FileViewMode>("markdown");
  let renderedCode = $state("");
  let codeLineNumbersText = $state("");
  let imagePreviewUrl = $state("");
  let sourceLink = $state("");
  let localImageObjectUrl: string | null = null;
  let directoryCloseTimer: ReturnType<typeof setTimeout> | null = null;

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
  const IMAGE_EXTENSIONS = new Set([
    "png",
    "jpg",
    "jpeg",
    "gif",
    "webp",
    "svg",
    "bmp",
    "ico",
    "avif",
  ]);
  const CODE_EXTENSIONS = new Set([
    "js",
    "jsx",
    "ts",
    "tsx",
    "json",
    "py",
    "rb",
    "go",
    "rs",
    "java",
    "kt",
    "swift",
    "c",
    "h",
    "cpp",
    "hpp",
    "cs",
    "php",
    "sh",
    "bash",
    "zsh",
    "yaml",
    "yml",
    "toml",
    "ini",
    "xml",
    "html",
    "css",
    "scss",
    "less",
    "sql",
    "lua",
    "dart",
    "r",
    "mdx",
    "txt",
    "log",
    "env",
  ]);
  const LANGUAGE_BY_EXT: Record<string, string> = {
    js: "javascript",
    jsx: "javascript",
    ts: "typescript",
    tsx: "typescript",
    py: "python",
    rb: "ruby",
    rs: "rust",
    go: "go",
    kt: "kotlin",
    c: "c",
    cpp: "cpp",
    h: "c",
    hpp: "cpp",
    cs: "csharp",
    sh: "bash",
    zsh: "bash",
    yml: "yaml",
    mdx: "markdown",
    env: "bash",
  };
  const MIME_BY_IMAGE_EXT: Record<string, string> = {
    png: "image/png",
    jpg: "image/jpeg",
    jpeg: "image/jpeg",
    gif: "image/gif",
    webp: "image/webp",
    svg: "image/svg+xml",
    bmp: "image/bmp",
    ico: "image/x-icon",
    avif: "image/avif",
  };

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
  const getExtension = (filePath: string) => filePath.split(".").at(-1)?.toLowerCase() || "";

  const resolveFileViewMode = (filePath: string): FileViewMode => {
    if (isMarkdownFile(filePath)) return "markdown";
    const ext = getExtension(filePath);
    if (IMAGE_EXTENSIONS.has(ext)) return "image";
    if (CODE_EXTENSIONS.has(ext)) return "code";
    return "unsupported";
  };

  const resetBinaryPreview = () => {
    if (localImageObjectUrl) {
      URL.revokeObjectURL(localImageObjectUrl);
      localImageObjectUrl = null;
    }
    imagePreviewUrl = "";
  };

  const openDirectoryDrawer = () => {
    if (directoryCloseTimer) {
      clearTimeout(directoryCloseTimer);
      directoryCloseTimer = null;
    }
    directoryClosing = false;
    directoryOpen = true;
  };

  const closeDirectoryDrawer = () => {
    if (!directoryOpen || directoryClosing) return;
    directoryClosing = true;
    directoryCloseTimer = setTimeout(() => {
      directoryOpen = false;
      directoryClosing = false;
      directoryCloseTimer = null;
    }, 200);
  };

  const renderCode = (rawContent: string, filePath: string) => {
    const ext = getExtension(filePath);
    const lang = LANGUAGE_BY_EXT[ext] || ext;
    const lineCount = rawContent.split("\n").length;
    codeLineNumbersText = Array.from({ length: lineCount }, (_, i) => String(i + 1)).join("\n");
    try {
      if (lang && hljs.getLanguage(lang)) {
        renderedCode = hljs.highlight(rawContent, { language: lang }).value;
        return;
      }
      renderedCode = hljs.highlightAuto(rawContent).value;
    } catch {
      renderedCode = escapeHtml(rawContent);
    }
  };

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
    fileViewMode = resolveFileViewMode(filePath);
    renderedCode = "";
    codeLineNumbersText = "";
    sourceLink = "";
    resetBinaryPreview();
    contentLoading = true;
    contentError = "";
    try {
      if (currentType === "remote" && "url" in skill && skill.url) {
        const branch = skill.branch || "main";
        sourceLink = buildGitHubUrl(skill.url, skill.path || "", filePath, branch) || "";
      } else if (currentType === "local") {
        sourceLink = resolveLocalPath(filePath) || "";
      }

      if (fileViewMode === "unsupported") {
        content = "";
        parsedFrontmatter = {};
        hasFrontmatter = false;
        return;
      }

      if (fileViewMode === "image") {
        if (currentType === "local") {
          const localPath = getSkillRootPath();
          if (!localPath) throw new Error("Skill path is missing");
          const bytes = await readSkillRelativeFileBytes(localPath, filePath);
          const ext = getExtension(filePath);
          const blob = new Blob([new Uint8Array(bytes)], {
            type: MIME_BY_IMAGE_EXT[ext] || "application/octet-stream",
          });
          localImageObjectUrl = URL.createObjectURL(blob);
          imagePreviewUrl = localImageObjectUrl;
        } else {
          if (!("url" in skill)) throw new Error("Skill source URL is missing");
          const repoMatch = skill.url?.match(/github\.com\/([^\/]+)\/([^\/]+)/);
          if (!repoMatch) throw new Error("Unable to construct raw URL for this skill");
          const [, owner, repo] = repoMatch;
          const cleanRepo = repo.replace(/\.git$/, "");
          const branch = skill.branch || "main";
          const basePath = (skill.path || "").replace(/^\/+|\/+$/g, "");
          const fullPath = basePath ? `${basePath}/${filePath}` : filePath;
          imagePreviewUrl = `https://raw.githubusercontent.com/${owner}/${cleanRepo}/${encodeURIComponent(branch)}/${fullPath}`;
        }
        content = "";
        parsedFrontmatter = {};
        hasFrontmatter = false;
        return;
      }

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
      if (fileViewMode === "markdown") {
        const parsed = parseMarkdown(rawContent);
        parsedFrontmatter = parsed.frontmatter as Record<string, string>;
        hasFrontmatter = parsed.hasFrontmatter;
        content = parsed.content;
      } else if (fileViewMode === "code") {
        parsedFrontmatter = {};
        hasFrontmatter = false;
        content = rawContent;
        renderCode(rawContent, filePath);
      }
    } catch (err) {
      contentError = String(err);
      parsedFrontmatter = {};
      hasFrontmatter = false;
      content = "";
      renderedCode = "";
      codeLineNumbersText = "";
      resetBinaryPreview();
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

  const handleOpenSource = async () => {
    if (!sourceLink) return;
    if (currentType === "local") {
      await openInFileManager(sourceLink);
      return;
    }
    await open(sourceLink);
  };

  const handleMarkdownExternalLink = async (href: string) => {
    await open(href);
  };

  const handleMarkdownRelativeLink = async (href: string) => {
    if (!skill) return;
    const targetPath = normalizeRelativePath(activeFilePath, href);
    if (targetPath && directoryEntries.some((entry) => !entry.is_directory && entry.path === targetPath)) {
      await loadContent(targetPath);
      return;
    }

    if (currentType === "local") {
      const localPath = resolveLocalPath(href);
      if (localPath) await openInFileManager(localPath);
      return;
    }

    if ("url" in skill && skill.url) {
      const fullUrl = buildGitHubUrl(skill.url, skill.path || "", href, skill.branch || "main");
      if (fullUrl) await open(fullUrl);
    }
  };

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

  $effect(() => {
    return () => {
      if (directoryCloseTimer) clearTimeout(directoryCloseTimer);
      resetBinaryPreview();
    };
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
      if (!directoryLoading && !skillLoading) openDirectoryDrawer();
    }}
    onRefreshAgentApps={() => {}}
  />

  <SkillDirectoryDrawer
    open={directoryOpen}
    closing={directoryClosing}
    loading={directoryLoading}
    error={directoryError}
    entries={directoryEntries}
    activePath={activeFilePath}
    onClose={closeDirectoryDrawer}
    onSelect={(path) => {
      closeDirectoryDrawer();
      loadContent(path).catch(console.error);
    }}
  />

  <main class="flex-1 overflow-y-auto">
    <div
      class="mx-auto max-w-6xl px-6"
      class:py-0={fileViewMode === "code"}
      class:py-2={fileViewMode === "image"}
      class:py-6={fileViewMode !== "code" && fileViewMode !== "image"}
    >
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
            {#if fileViewMode === "markdown"}
              <MarkdownPreview
                htmlContent={renderMarkdownBody(content)}
                frontmatterDescription={hasFrontmatter ? (parsedFrontmatter.description ?? "") : ""}
                onOpenExternalLink={handleMarkdownExternalLink}
                onOpenRelativeLink={handleMarkdownRelativeLink}
              />
            {:else if fileViewMode === "code"}
              <CodePreview lineNumbersText={codeLineNumbersText} {renderedCode} />
            {:else if fileViewMode === "image"}
              <ImagePreview src={imagePreviewUrl} alt={activeFilePath} />
            {:else}
              <div class="text-sm">
                <p class="text-base-content mb-3">{$t("detail.unsupportedFile")}</p>
                {#if sourceLink}
                  <button
                    class="bg-primary text-primary-content hover:bg-primary-hover rounded-lg px-3 py-2 text-xs transition"
                    onclick={handleOpenSource}
                    type="button"
                  >
                    {$t(currentType === "local" ? "detail.openInFileManager" : "detail.openSource")}
                  </button>
                {/if}
              </div>
            {/if}
          {/if}
        </div>
      {/if}
    </div>
  </main>
</div>
