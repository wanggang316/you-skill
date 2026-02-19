<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { open } from "@tauri-apps/plugin-shell";
  import { Loader2 } from "@lucide/svelte";
  import PageHeader from "../../../../lib/components/PageHeader.svelte";
  import { parseMarkdown, renderMarkdownBody } from "../../../../lib/utils/markdown";
  import { t } from "../../../../lib/i18n";
  import {
    fetchSkillsByNames,
    listSkills,
    openInFileManager,
    readSkillReadme,
    type LocalSkill,
    type RemoteSkill,
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

  const params = $derived($page.params);

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

  const buildGitHubUrl = (url: string, path: string | null | undefined) => {
    if (!url || !url.includes("github.com")) return null;
    const match = url.match(/github\.com\/([^\/]+)\/([^\/]+)/);
    if (!match) return null;
    const [, owner, repo] = match;
    const cleanRepo = repo.replace(/\.git$/, "");
    return `https://github.com/${owner}/${cleanRepo}/tree/main/${path || ""}`;
  };

  const loadSkill = async () => {
    skillLoading = true;
    error = "";
    skill = null;

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
        const localSkills = await listSkills();
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

  const convertToRawUrl = (url: string | null | undefined, path: string | null | undefined) => {
    if (!url || !path || !url.includes("github.com")) return null;
    const match = url.match(/github\.com\/([^\/]+)\/([^\/]+)/);
    if (!match) return null;
    const [, owner, repo] = match;
    const cleanRepo = repo.replace(/\.git$/, "");
    return `https://raw.githubusercontent.com/${owner}/${cleanRepo}/refs/heads/main/${path}/SKILL.md`;
  };

  const getBaseUrl = () => {
    if (!skill || !("url" in skill) || !skill.url) return null;
    return buildGitHubUrl(skill.url, skill.path || "");
  };

  const resolveLocalPath = (relativePath: string) => {
    if (!skill || !("installed_agent_apps" in skill)) return null;
    const dirPath = skill.global_folder || skill.installed_agent_apps[0]?.skill_folder;
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

  const loadContent = async () => {
    if (!skill) return;
    contentLoading = true;
    contentError = "";
    try {
      let rawContent = "";
      if (currentType === "local") {
        if (!("installed_agent_apps" in skill)) {
          throw new Error("Skill path is missing");
        }
        const localPath = skill.global_folder || skill.installed_agent_apps[0]?.skill_folder;
        if (!localPath) {
          throw new Error("Skill path is missing");
        }
        rawContent = await readSkillReadme(localPath);
      } else {
        if (!("url" in skill)) {
          throw new Error("Skill source URL is missing");
        }
        const rawUrl = convertToRawUrl(skill.url, skill.path);
        if (!rawUrl) {
          throw new Error("Unable to construct raw URL for this skill");
        }
        const response = await fetch(rawUrl);
        if (!response.ok) {
          throw new Error(`Failed to fetch: ${response.status} ${response.statusText}`);
        }
        rawContent = await response.text();
      }
      const parsed = parseMarkdown(rawContent);
      parsedFrontmatter = parsed.frontmatter as Record<string, string>;
      hasFrontmatter = parsed.hasFrontmatter;
      content = parsed.content;
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
    goto("/");
  };

  const handleDetailAction = async () => {
    if (!skill) return;

    if (currentType === "remote" && "url" in skill && skill.url) {
      const fullUrl = buildGitHubUrl(skill.url, skill.path || "") || skill.url;
      await open(fullUrl);
      return;
    }

    if (currentType === "local" && "installed_agent_apps" in skill) {
      const localPath = skill.global_folder || skill.installed_agent_apps[0]?.skill_folder;
      if (localPath) {
        await openInFileManager(localPath);
      }
    }
  };

  const retryContent = () => {
    loadContent().catch(console.error);
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
      if (currentType === "local") {
        const localPath = resolveLocalPath(href);
        if (localPath) await openInFileManager(localPath);
      } else if (getBaseUrl() && "url" in skill) {
        const fullUrl = buildGitHubUrl(skill.url, skill.path || "", href);
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
    loadSkill().catch(console.error);
  });

  $effect(() => {
    if (skill) {
      loadContent().catch(console.error);
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
    onBack={handleBack}
    onDetailAction={handleDetailAction}
    onRefreshAgentApps={() => {}}
  />

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
            <div class="markdown-content" use:markdownInteractions>
              {@html renderMarkdownBody(content)}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </main>
</div>
