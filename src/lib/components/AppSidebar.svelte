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

  let {
    activeTab,
    currentSection = "skills",
    scopeKey = $bindable("global"),
    projects,
    hasUpdate,
    updateLoading,
    onAddSkill,
    onSelectTab,
    onOpenUpdate,
    onOpenProjectManage,
    onOpenSettings,
  }: {
    activeTab: string;
    currentSection?: "skills" | "settings";
    scopeKey: string;
    projects: UserProject[];
    hasUpdate: boolean;
    updateLoading: boolean;
    onAddSkill: () => void;
    onSelectTab: (tab: "local" | "remote", scopeKey?: string) => void;
    onOpenUpdate: () => void;
    onOpenProjectManage: () => void;
    onOpenSettings: () => void;
  } = $props();

  const projectScopeKey = (path: string) => `project:${encodeURIComponent(path)}`;

  const selectGlobal = () => {
    scopeKey = "global";
    onSelectTab("local", scopeKey);
  };

  const selectProject = (path: string) => {
    scopeKey = projectScopeKey(path);
    onSelectTab("local", scopeKey);
  };
</script>

<aside class="sidebar-shell" aria-label={$t("sidebar.navigation")} data-window-drag-region>
  <nav class="sidebar-nav">
    <button class="nav-item add-item" type="button" onclick={onAddSkill}>
      <Plus size={17} strokeWidth={1.8} />
      <span>{$t("addSkill.title")}</span>
    </button>

    <div class="nav-group" aria-label={$t("sidebar.skills")}>
      <button
        class:active={currentSection === "skills" && activeTab === "remote"}
        class="nav-item"
        type="button"
        onclick={() => onSelectTab("remote")}
        aria-current={currentSection === "skills" && activeTab === "remote" ? "page" : undefined}
      >
        <LibraryBig size={17} strokeWidth={1.8} />
        <span>{$t("sidebar.library")}</span>
      </button>
      <button
        class:active={currentSection === "skills" && activeTab === "local" && scopeKey === "global"}
        class="nav-item"
        type="button"
        onclick={selectGlobal}
        aria-current={currentSection === "skills" && activeTab === "local" && scopeKey === "global"
          ? "page"
          : undefined}
      >
        <Globe2 size={17} strokeWidth={1.8} />
        <span>{$t("sidebar.global")}</span>
      </button>
    </div>

    <section class="project-section" aria-labelledby="project-heading">
      <div class="section-heading">
        <span id="project-heading">{$t("sidebar.projects")}</span>
        <button
          class="section-action"
          type="button"
          onclick={onOpenProjectManage}
          title={$t("projectManage.title")}
          aria-label={$t("projectManage.title")}
        >
          <FolderPlus size={15} strokeWidth={1.8} />
        </button>
      </div>

      <div class="project-list">
        {#each projects as project (project.path)}
          {@const key = projectScopeKey(project.path)}
          <button
            class:active={currentSection === "skills" && activeTab === "local" && scopeKey === key}
            class="nav-item project-item"
            type="button"
            onclick={() => selectProject(project.path)}
            aria-current={currentSection === "skills" && activeTab === "local" && scopeKey === key
              ? "page"
              : undefined}
            title={project.path}
          >
            {#if currentSection === "skills" && activeTab === "local" && scopeKey === key}
              <FolderOpen size={17} strokeWidth={1.8} />
            {:else}
              <Folder size={17} strokeWidth={1.8} />
            {/if}
            <span>{project.name}</span>
          </button>
        {:else}
          <button class="empty-projects" type="button" onclick={onOpenProjectManage}>
            {$t("projectManage.empty")}
          </button>
        {/each}
      </div>
    </section>
  </nav>

  <div class="sidebar-footer">
    {#if hasUpdate}
      <button
        class="nav-item update-item"
        type="button"
        onclick={onOpenUpdate}
        disabled={updateLoading}
      >
        {#if updateLoading}
          <Loader2 size={17} class="animate-spin" />
        {:else}
          <ArrowUpCircle size={17} strokeWidth={1.8} />
        {/if}
        <span>{$t("header.updateAvailable")}</span>
      </button>
    {/if}
    <button
      class:active={currentSection === "settings"}
      class="nav-item"
      type="button"
      onclick={onOpenSettings}
      aria-current={currentSection === "settings" ? "page" : undefined}
    >
      <Settings size={17} strokeWidth={1.8} />
      <span>{$t("header.settings")}</span>
    </button>
  </div>
</aside>

<style>
  .sidebar-shell {
    display: flex;
    min-width: 0;
    flex-direction: column;
    border-right: 1px solid var(--base-300);
    background: color-mix(in oklch, var(--base-200) 82%, var(--base-100));
  }

  .sidebar-nav {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    gap: 0;
    padding: 0.4rem 0.625rem 1rem;
  }

  .nav-group {
    display: grid;
    gap: 0.125rem;
  }

  .nav-item {
    display: flex;
    width: 100%;
    min-width: 0;
    align-items: center;
    gap: 0.65rem;
    border: 0;
    border-radius: 0.55rem;
    background: transparent;
    padding: 0.55rem 0.625rem;
    color: color-mix(in oklch, var(--base-content) 84%, transparent);
    font: inherit;
    font-size: 0.875rem;
    font-weight: 450;
    line-height: 1.25;
    text-align: left;
    cursor: pointer;
    transition:
      background-color 160ms ease,
      color 160ms ease,
      transform 160ms ease;
  }

  .nav-item span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .nav-item:hover {
    background: color-mix(in oklch, var(--base-300) 72%, transparent);
    color: var(--base-content);
  }

  .nav-item:active {
    transform: translateY(1px);
  }

  .nav-item:focus-visible,
  .section-action:focus-visible,
  .empty-projects:focus-visible {
    outline: 2px solid color-mix(in oklch, var(--primary) 58%, transparent);
    outline-offset: 1px;
  }

  .nav-item.active {
    background: var(--base-300);
    color: var(--base-content);
    font-weight: 520;
  }

  .add-item {
    width: calc(100% - 4.25rem);
    margin-left: 4.25rem;
  }

  .project-section {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    margin-top: 0.85rem;
  }

  .section-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.25rem 0.5rem 0.35rem 0.625rem;
    color: var(--base-content-subtle);
    font-size: 0.75rem;
    font-weight: 520;
  }

  .section-action {
    display: inline-flex;
    width: 1.7rem;
    height: 1.7rem;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: 0.45rem;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .section-action:hover {
    background: var(--base-300);
    color: var(--base-content);
  }

  .project-list {
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar-thumb) transparent;
  }

  .project-item {
    margin-bottom: 0.125rem;
  }

  .empty-projects {
    width: 100%;
    border: 0;
    border-radius: 0.5rem;
    background: transparent;
    padding: 0.6rem;
    color: var(--base-content-faint);
    font: inherit;
    font-size: 0.78rem;
    text-align: left;
    cursor: pointer;
  }

  .empty-projects:hover {
    color: var(--base-content-subtle);
  }

  .sidebar-footer {
    display: grid;
    gap: 0.125rem;
    border-top: 1px solid var(--base-300);
    padding: 0.65rem;
  }

  .update-item {
    color: var(--error);
  }

  .update-item:disabled {
    cursor: default;
    opacity: 0.55;
  }

  @media (max-width: 52rem) {
    .sidebar-nav,
    .sidebar-footer {
      padding-inline: 0.45rem;
    }
  }
</style>
