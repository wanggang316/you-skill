<script lang="ts">
  import {
    ArrowUpCircle,
    Folder,
    FolderOpen,
    FolderPlus,
    Globe2,
    LibraryBig,
    Loader2,
    Plus,
    Settings,
  } from "@lucide/svelte";
  import type { UserProject } from "$lib/api/user-projects";
  import { t } from "$lib/i18n";
  import {
    buildSkillsHref,
    projectScopeKey,
    type SidebarActiveKey,
  } from "$lib/navigation/app-shell";

  let {
    activeKey,
    projects,
    hasUpdate,
    updateLoading,
    onAddSkill,
    onOpenUpdate,
    onOpenProjectManage,
  }: {
    activeKey: SidebarActiveKey;
    projects: UserProject[];
    hasUpdate: boolean;
    updateLoading: boolean;
    onAddSkill: () => void;
    onOpenUpdate: () => void;
    onOpenProjectManage: () => void;
  } = $props();
</script>

<aside
  class="bg-base-200 border-base-300 flex min-w-0 flex-col border-r"
  aria-label={$t("sidebar.navigation")}
  data-window-drag-region
>
  <nav class="flex min-h-0 flex-1 flex-col px-2.5 pt-10 pb-4 max-[832px]:px-[0.45rem]">
    <div class="grid gap-0.5" aria-label={$t("sidebar.skills")}>
      <button
        class="text-base-content/80 hover:bg-base-300 hover:text-base-content focus-visible:outline-primary/60 flex w-full min-w-0 items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm leading-5 font-normal transition-[background-color,color,transform] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1 active:translate-y-px"
        type="button"
        onclick={onAddSkill}
      >
        <Plus size={17} strokeWidth={1.8} />
        <span class="min-w-0 truncate">{$t("addSkill.title")}</span>
      </button>
      <a
        class="text-base-content/80 hover:bg-base-300 hover:text-base-content focus-visible:outline-primary/60 flex w-full min-w-0 items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm leading-5 font-normal transition-[background-color,color,transform] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1 active:translate-y-px"
        class:bg-base-300={activeKey === "library"}
        class:text-base-content={activeKey === "library"}
        class:font-medium={activeKey === "library"}
        href={buildSkillsHref("remote")}
        aria-current={activeKey === "library" ? "page" : undefined}
      >
        <LibraryBig size={17} strokeWidth={1.8} />
        <span class="min-w-0 truncate">{$t("sidebar.library")}</span>
      </a>
      <a
        class="text-base-content/80 hover:bg-base-300 hover:text-base-content focus-visible:outline-primary/60 flex w-full min-w-0 items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm leading-5 font-normal transition-[background-color,color,transform] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1 active:translate-y-px"
        class:bg-base-300={activeKey === "global"}
        class:text-base-content={activeKey === "global"}
        class:font-medium={activeKey === "global"}
        href={buildSkillsHref("local")}
        aria-current={activeKey === "global" ? "page" : undefined}
      >
        <Globe2 size={17} strokeWidth={1.8} />
        <span class="min-w-0 truncate">{$t("sidebar.global")}</span>
      </a>
    </div>

    <section class="mt-3.5 flex min-h-0 flex-1 flex-col" aria-labelledby="project-heading">
      <div
        class="text-base-content-subtle flex items-center justify-between pt-1 pr-2 pb-1.5 pl-2.5 text-xs font-medium"
      >
        <span id="project-heading">{$t("sidebar.projects")}</span>
        <button
          class="hover:bg-base-300 hover:text-base-content focus-visible:outline-primary/60 inline-flex size-7 items-center justify-center rounded-md bg-transparent focus-visible:outline-2 focus-visible:outline-offset-1"
          type="button"
          onclick={onOpenProjectManage}
          title={$t("projectManage.title")}
          aria-label={$t("projectManage.title")}
        >
          <FolderPlus size={15} strokeWidth={1.8} />
        </button>
      </div>

      <div
        class="min-h-0 overflow-y-auto [scrollbar-color:var(--scrollbar-thumb)_transparent] [scrollbar-width:thin]"
      >
        {#each projects as project (project.path)}
          {@const key = projectScopeKey(project.path)}
          <a
            class="text-base-content/80 hover:bg-base-300 hover:text-base-content focus-visible:outline-primary/60 mb-0.5 flex w-full min-w-0 items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm leading-5 font-normal transition-[background-color,color,transform] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1 active:translate-y-px"
            class:bg-base-300={activeKey === key}
            class:text-base-content={activeKey === key}
            class:font-medium={activeKey === key}
            href={buildSkillsHref("local", project.path)}
            aria-current={activeKey === key ? "page" : undefined}
            title={project.path}
          >
            {#if activeKey === key}
              <FolderOpen size={17} strokeWidth={1.8} />
            {:else}
              <Folder size={17} strokeWidth={1.8} />
            {/if}
            <span class="min-w-0 truncate">{project.name}</span>
          </a>
        {:else}
          <button
            class="text-base-content-faint hover:text-base-content-subtle focus-visible:outline-primary/60 w-full rounded-lg bg-transparent p-2.5 text-left text-xs focus-visible:outline-2 focus-visible:outline-offset-1"
            type="button"
            onclick={onOpenProjectManage}
          >
            {$t("projectManage.empty")}
          </button>
        {/each}
      </div>
    </section>
  </nav>

  <div class="border-base-300 grid gap-0.5 border-t p-2.5 max-[832px]:px-[0.45rem]">
    {#if hasUpdate}
      <button
        class="text-error hover:bg-base-300 focus-visible:outline-primary/60 flex w-full min-w-0 items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm leading-5 font-normal transition-[background-color,color,transform] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1 active:translate-y-px disabled:cursor-default disabled:opacity-55"
        type="button"
        onclick={onOpenUpdate}
        disabled={updateLoading}
      >
        {#if updateLoading}
          <Loader2 size={17} class="animate-spin" />
        {:else}
          <ArrowUpCircle size={17} strokeWidth={1.8} />
        {/if}
        <span class="min-w-0 truncate">{$t("header.updateAvailable")}</span>
      </button>
    {/if}
    <a
      class="text-base-content/80 hover:bg-base-300 hover:text-base-content focus-visible:outline-primary/60 flex w-full min-w-0 items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm leading-5 font-normal transition-[background-color,color,transform] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1 active:translate-y-px"
      class:bg-base-300={activeKey === "settings"}
      class:text-base-content={activeKey === "settings"}
      class:font-medium={activeKey === "settings"}
      href="/settings"
      aria-current={activeKey === "settings" ? "page" : undefined}
    >
      <Settings size={17} strokeWidth={1.8} />
      <span class="min-w-0 truncate">{$t("header.settings")}</span>
    </a>
  </div>
</aside>
