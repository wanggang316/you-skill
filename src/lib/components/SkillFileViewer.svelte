<script lang="ts">
  import { open as openExternal } from "@tauri-apps/plugin-shell";
  import hljs from "highlight.js/lib/common";
  import { ExternalLink, Languages, List, Loader2 } from "@lucide/svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import SkillDirectoryDrawer from "$lib/components/SkillDirectoryDrawer.svelte";
  import MarkdownPreview from "$lib/components/MarkdownPreview.svelte";
  import CodePreview from "$lib/components/CodePreview.svelte";
  import ImagePreview from "$lib/components/ImagePreview.svelte";
  import TranslateSettingsModal from "$lib/components/TranslateSettingsModal.svelte";
  import MissingTranslationSettingsModal from "$lib/components/MissingTranslationSettingsModal.svelte";
  import { parseMarkdown, renderMarkdownBody } from "$lib/utils/markdown";
  import { t } from "$lib/i18n";
  import { settings, updateSettings } from "$lib/stores/settings";
  import {
    listSkillDirectory,
    openInFileManager,
    readSkillFile,
    readSkillRelativeFile,
    readSkillRelativeFileBytes,
    translateSkillMarkdown,
    type RemoteSkill,
    type SkillDirectoryEntry,
  } from "$lib/api/skills";

  /** Where the files live: a hub directory on disk, or a GitHub-hosted marketplace skill. */
  export type FileSource =
    | { kind: "hub"; name: string; rootPath: string }
    | { kind: "remote"; name: string; skill: RemoteSkill };

  type FileViewMode = "markdown" | "code" | "image" | "unsupported";

  let {
    source,
    dense = false,
  }: {
    source: FileSource;
    /** Tighter paddings when embedded in another pane. */
    dense?: boolean;
  } = $props();

  let contentLoading = $state(true);
  let contentError = $state("");
  let content = $state("");
  let originalMarkdownContent = $state("");
  let translatedMarkdownContent = $state<string | null>(null);
  let showingTranslated = $state(false);
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
  let translating = $state(false);
  let translateSettingsOpen = $state(false);
  let savingTranslateSettings = $state(false);
  let missingTranslationSettingsOpen = $state(false);
  let missingTranslationSettingsFields = $state<string[]>([]);
  let loadedKey = $state("");
  let localImageObjectUrl: string | null = null;
  let directoryCloseTimer: ReturnType<typeof setTimeout> | null = null;
  const translationCache = new Map<string, string>();

  const sourceKey = $derived(
    source.kind === "hub"
      ? `hub|${source.name}|${source.rootPath}`
      : `remote|${source.name}|${source.skill.url ?? ""}|${source.skill.path ?? ""}`
  );

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
    const resolved = baseFile.split("/").slice(0, -1);
    for (const segment of withoutHash.split("/")) {
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
      const aParts = a.path.split("/");
      const bParts = b.path.split("/");
      const minLen = Math.min(aParts.length, bParts.length);
      for (let i = 0; i < minLen; i++) {
        const cmp = aParts[i].localeCompare(bParts[i], undefined, { sensitivity: "base" });
        if (cmp !== 0) return cmp;
      }
      if (aParts.length !== bParts.length) return aParts.length - bParts.length;
      if (a.is_directory !== b.is_directory) return a.is_directory ? -1 : 1;
      return 0;
    });

  const getExtension = (filePath: string) => filePath.split(".").at(-1)?.toLowerCase() || "";
  const isTranslatableMarkdown = $derived(fileViewMode === "markdown");
  const translateButtonLabel = $derived.by(() => {
    if (showingTranslated) return $t("detail.showOriginal");
    if (translatedMarkdownContent) return $t("detail.showTranslated");
    return $t("detail.translate");
  });

  const contentHash = (input: string): string => {
    let hash = 5381;
    for (let i = 0; i < input.length; i += 1) {
      hash = (hash * 33) ^ input.charCodeAt(i);
    }
    return (hash >>> 0).toString(36);
  };

  const applyMarkdownContent = (markdown: string) => {
    const parsed = parseMarkdown(markdown);
    parsedFrontmatter = parsed.frontmatter as Record<string, string>;
    hasFrontmatter = parsed.hasFrontmatter;
    content = parsed.content;
  };

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

  const renderCode = (raw: string, filePath: string) => {
    const ext = getExtension(filePath);
    const lang = LANGUAGE_BY_EXT[ext] || ext;
    const lineCount = raw.split("\n").length;
    codeLineNumbersText = Array.from({ length: lineCount }, (_, i) => String(i + 1)).join("\n");
    try {
      if (lang && hljs.getLanguage(lang)) {
        renderedCode = hljs.highlight(raw, { language: lang }).value;
        return;
      }
      renderedCode = hljs.highlightAuto(raw).value;
    } catch {
      renderedCode = escapeHtml(raw);
    }
  };

  const githubRepo = (
    skill: RemoteSkill
  ): { owner: string; repo: string; branch: string } | null => {
    const match = skill.url?.match(/github\.com\/([^/]+)\/([^/]+)/);
    if (!match) return null;
    return {
      owner: match[1],
      repo: match[2].replace(/\.git$/, ""),
      branch: skill.branch || "main",
    };
  };

  const buildGitHubUrl = (skill: RemoteSkill, relativePath?: string) => {
    const repo = githubRepo(skill);
    if (!repo) return null;
    const basePath = (skill.path || "").replace(/^\/+|\/+$/g, "");
    if (!relativePath) {
      return basePath
        ? `https://github.com/${repo.owner}/${repo.repo}/tree/${repo.branch}/${basePath}`
        : `https://github.com/${repo.owner}/${repo.repo}`;
    }
    const normalized = relativePath.replace(/^\/+/, "");
    const fullPath = basePath ? `${basePath}/${normalized}` : normalized;
    return `https://github.com/${repo.owner}/${repo.repo}/tree/${repo.branch}/${fullPath}`;
  };

  const buildRawUrl = (skill: RemoteSkill, filePath: string) => {
    const repo = githubRepo(skill);
    if (!repo) return null;
    const basePath = (skill.path || "").replace(/^\/+|\/+$/g, "");
    const fullPath = basePath ? `${basePath}/${filePath}` : filePath;
    return `https://raw.githubusercontent.com/${repo.owner}/${repo.repo}/${encodeURIComponent(repo.branch)}/${fullPath}`;
  };

  const resolveLocalPath = (relativePath: string) => {
    if (source.kind !== "hub") return null;
    const dirPath = source.rootPath;
    const separator = dirPath.includes("\\") && !dirPath.includes("/") ? "\\" : "/";
    if (relativePath.startsWith("/")) {
      return dirPath + relativePath.split("/").join(separator);
    }
    const parts = dirPath.split(/[/\\]/);
    for (const part of relativePath.split("/")) {
      if (part === "..") {
        parts.pop();
      } else if (part && part !== ".") {
        parts.push(part);
      }
    }
    return parts.join(separator);
  };

  const loadDirectory = async (): Promise<SkillDirectoryEntry[]> => {
    directoryLoading = true;
    directoryError = "";
    try {
      if (source.kind === "hub") {
        const formatted = formatDirectoryEntries(await listSkillDirectory(source.rootPath));
        directoryEntries = formatted;
        return formatted;
      }
      const repo = githubRepo(source.skill);
      if (!repo) throw new Error("Unable to resolve GitHub repository");
      const response = await fetch(
        `https://api.github.com/repos/${repo.owner}/${repo.repo}/git/trees/${encodeURIComponent(repo.branch)}?recursive=1`
      );
      if (!response.ok) {
        throw new Error(`Failed to fetch tree: ${response.status} ${response.statusText}`);
      }
      const payload = await response.json();
      const basePath = (source.skill.path || "").replace(/^\/+|\/+$/g, "");
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
    } catch (err) {
      directoryError = String(err);
      directoryEntries = [];
      return [];
    } finally {
      directoryLoading = false;
    }
  };

  const clearContentState = () => {
    content = "";
    originalMarkdownContent = "";
    translatedMarkdownContent = null;
    showingTranslated = false;
    parsedFrontmatter = {};
    hasFrontmatter = false;
    renderedCode = "";
    codeLineNumbersText = "";
  };

  const loadContent = async (filePath = "SKILL.md") => {
    activeFilePath = filePath;
    fileViewMode = resolveFileViewMode(filePath);
    sourceLink = "";
    translating = false;
    clearContentState();
    resetBinaryPreview();
    contentLoading = true;
    contentError = "";
    try {
      sourceLink =
        source.kind === "remote"
          ? buildGitHubUrl(source.skill, filePath) || ""
          : resolveLocalPath(filePath) || "";

      if (fileViewMode === "unsupported") return;

      if (fileViewMode === "image") {
        if (source.kind === "hub") {
          const bytes = await readSkillRelativeFileBytes(source.rootPath, filePath);
          const blob = new Blob([new Uint8Array(bytes)], {
            type: MIME_BY_IMAGE_EXT[getExtension(filePath)] || "application/octet-stream",
          });
          localImageObjectUrl = URL.createObjectURL(blob);
          imagePreviewUrl = localImageObjectUrl;
        } else {
          const raw = buildRawUrl(source.skill, filePath);
          if (!raw) throw new Error("Unable to construct raw URL for this skill");
          imagePreviewUrl = raw;
        }
        return;
      }

      let fetched = "";
      if (source.kind === "hub") {
        fetched =
          filePath === "SKILL.md"
            ? await readSkillFile(source.rootPath)
            : await readSkillRelativeFile(source.rootPath, filePath);
      } else {
        const raw = buildRawUrl(source.skill, filePath);
        if (!raw) throw new Error("Unable to construct raw URL for this skill");
        const response = await fetch(raw);
        if (!response.ok) {
          throw new Error(`Failed to fetch: ${response.status} ${response.statusText}`);
        }
        fetched = await response.text();
      }
      if (fileViewMode === "markdown") {
        originalMarkdownContent = fetched;
        applyMarkdownContent(fetched);
      } else {
        content = fetched;
        renderCode(fetched, filePath);
      }
    } catch (err) {
      contentError = String(err);
      clearContentState();
      resetBinaryPreview();
    } finally {
      contentLoading = false;
    }
  };

  const handleOpenExternal = async () => {
    if (source.kind === "remote") {
      const url = buildGitHubUrl(source.skill) || source.skill.url;
      if (url) await openExternal(url);
      return;
    }
    await openInFileManager(source.rootPath);
  };

  const handleOpenSource = async () => {
    if (!sourceLink) return;
    if (source.kind === "hub") {
      await openInFileManager(sourceLink);
      return;
    }
    await openExternal(sourceLink);
  };

  const handleMarkdownRelativeLink = async (href: string) => {
    const targetPath = normalizeRelativePath(activeFilePath, href);
    if (
      targetPath &&
      directoryEntries.some((entry) => !entry.is_directory && entry.path === targetPath)
    ) {
      await loadContent(targetPath);
      return;
    }
    if (source.kind === "hub") {
      const localPath = resolveLocalPath(href);
      if (localPath) await openInFileManager(localPath);
      return;
    }
    const url = buildGitHubUrl(source.skill, href);
    if (url) await openExternal(url);
  };

  const missingTranslationFields = (): string[] => {
    const fields: string[] = [];
    if (!($settings.translate_target_language || "").trim()) {
      fields.push($t("settings.translation.targetLanguage"));
    }
    if (!($settings.openrouter_api_key || "").trim()) {
      fields.push($t("settings.translation.apiKey"));
    }
    if (!($settings.translate_model || "").trim()) {
      fields.push($t("settings.translation.model"));
    }
    return fields;
  };

  const handleTranslate = async () => {
    if (!isTranslatableMarkdown || !originalMarkdownContent || translating) return;

    if (showingTranslated) {
      showingTranslated = false;
      applyMarkdownContent(originalMarkdownContent);
      return;
    }
    if (translatedMarkdownContent) {
      showingTranslated = true;
      applyMarkdownContent(translatedMarkdownContent);
      return;
    }

    const cacheKey = [
      sourceKey,
      activeFilePath,
      ($settings.translate_target_language || "").trim(),
      ($settings.translate_model || "").trim(),
      contentHash(originalMarkdownContent),
    ].join("|");
    const cached = translationCache.get(cacheKey);
    if (cached) {
      translatedMarkdownContent = cached;
      showingTranslated = true;
      applyMarkdownContent(cached);
      return;
    }

    const missing = missingTranslationFields();
    if (missing.length > 0) {
      missingTranslationSettingsFields = missing;
      missingTranslationSettingsOpen = true;
      return;
    }

    translating = true;
    contentError = "";
    try {
      const translated = await translateSkillMarkdown(originalMarkdownContent);
      translatedMarkdownContent = translated;
      translationCache.set(cacheKey, translated);
      showingTranslated = true;
      applyMarkdownContent(translated);
    } catch (err) {
      const errorText = String(err);
      const notConfigured =
        errorText.includes("未配置") || errorText.toLowerCase().includes("not configured");
      if (notConfigured) {
        const fields = missingTranslationFields();
        missingTranslationSettingsFields = fields.length
          ? fields
          : [$t("settings.translation.apiKey")];
        missingTranslationSettingsOpen = true;
      } else {
        contentError = errorText;
      }
    } finally {
      translating = false;
    }
  };

  const handleSaveTranslateSettings = async (payload: {
    apiKey: string;
    targetLanguage: string;
    model: string;
  }) => {
    savingTranslateSettings = true;
    try {
      await updateSettings({
        openrouter_api_key: payload.apiKey || null,
        translate_target_language: payload.targetLanguage.trim(),
        translate_model: payload.model.trim(),
      });
      translateSettingsOpen = false;
    } finally {
      savingTranslateSettings = false;
    }
  };

  $effect(() => {
    const key = sourceKey;
    if (loadedKey === key) return;
    loadedKey = key;
    directoryOpen = false;
    loadDirectory()
      .then((entries) => {
        if (entries.some((entry) => !entry.is_directory && entry.path === "SKILL.md")) {
          return loadContent("SKILL.md");
        }
        const firstFile = entries.find((entry) => !entry.is_directory)?.path;
        return loadContent(firstFile || "SKILL.md");
      })
      .catch(console.error);
  });

  $effect(() => {
    return () => {
      if (directoryCloseTimer) clearTimeout(directoryCloseTimer);
      resetBinaryPreview();
    };
  });
</script>

<div class="flex min-h-0 min-w-0 flex-1 flex-col">
  <div
    class="border-base-300 flex h-10 flex-none items-center justify-between gap-3 border-b px-6 text-sm"
  >
    <div class="text-base-content-subtle flex min-w-0 items-center gap-2">
      <span class="truncate" title={activeFilePath}>{activeFilePath}</span>
    </div>
    <div class="flex shrink-0 items-center gap-1.5">
      {#if isTranslatableMarkdown}
        <button
          class="border-base-300 text-base-content hover:bg-base-200 flex h-7 items-center gap-1.5 rounded-lg border px-2.5 text-xs transition disabled:opacity-50"
          onclick={handleTranslate}
          disabled={translating || contentLoading}
          type="button"
        >
          {#if translating}
            <Loader2 size={13} class="animate-spin" />
            {$t("detail.translating")}
          {:else}
            <Languages size={13} />
            {translateButtonLabel}
          {/if}
        </button>
      {/if}
      <IconButton
        variant="outline"
        onclick={() => {
          if (!directoryLoading) openDirectoryDrawer();
        }}
        title={$t("detail.catalog")}
        class="h-7 w-7 rounded-lg p-0"
      >
        <List size={13} />
      </IconButton>
      <IconButton
        variant="outline"
        onclick={handleOpenExternal}
        title={source.kind === "hub" ? $t("detail.openInFileManager") : $t("detail.openInBrowser")}
        class="h-7 w-7 rounded-lg p-0"
      >
        <ExternalLink size={13} />
      </IconButton>
    </div>
  </div>

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

  <div class="min-h-0 flex-1 overflow-y-auto">
    <div
      class={`mx-auto ${dense ? "max-w-4xl px-6" : "max-w-6xl px-6"}`}
      class:py-0={fileViewMode === "code"}
      class:py-2={fileViewMode === "image"}
      class:py-6={fileViewMode !== "code" && fileViewMode !== "image"}
    >
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
              onclick={() => loadContent(activeFilePath).catch(console.error)}
              type="button"
            >
              {$t("detail.retry")}
            </button>
          </div>
        {:else if fileViewMode === "markdown"}
          <MarkdownPreview
            htmlContent={renderMarkdownBody(content)}
            frontmatterDescription={hasFrontmatter ? (parsedFrontmatter.description ?? "") : ""}
            onOpenExternalLink={(href) => openExternal(href)}
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
                {$t(source.kind === "hub" ? "detail.openInFileManager" : "detail.openSource")}
              </button>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<MissingTranslationSettingsModal
  bind:open={missingTranslationSettingsOpen}
  missingFields={missingTranslationSettingsFields}
  onSettings={() => {
    missingTranslationSettingsOpen = false;
    translateSettingsOpen = true;
  }}
/>

<TranslateSettingsModal
  bind:open={translateSettingsOpen}
  apiKey={$settings.openrouter_api_key ?? ""}
  targetLanguage={$settings.translate_target_language || ""}
  model={$settings.translate_model || ""}
  saving={savingTranslateSettings}
  onSave={handleSaveTranslateSettings}
/>
