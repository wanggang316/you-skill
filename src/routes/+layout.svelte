<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import AddSkillModal from "$lib/components/AddSkillModal.svelte";
  import AppSidebar from "$lib/components/AppSidebar.svelte";
  import UserProjectFormModal from "$lib/components/UserProjectFormModal.svelte";
  import { buildSkillsHref, getSkillsLocation } from "$lib/navigation/app-shell";
  import { loadSettings } from "$lib/stores/settings";
  import {
    agents as agentsStore,
    loadAgents,
    refreshLocal as refreshLocalSkills,
  } from "$lib/stores/skills";
  import { ensureUpdateChecked, installAvailableUpdate, updaterState } from "$lib/stores/updater";
  import { refreshUserProjects, userProjects } from "$lib/stores/user-projects";

  let { children } = $props();
  let addSkillModalOpen = $state(false);
  let userProjectsModalOpen = $state(false);
  let userProjectsModalWasOpen = $state(false);

  const skillsLocation = $derived(getSkillsLocation(page.url));
  const currentSection = $derived<"skills" | "settings">(
    skillsLocation.activeKey === "settings" ? "settings" : "skills"
  );
  const scopeKey = $derived(
    skillsLocation.activeKey.startsWith("project:") ? skillsLocation.activeKey : "global"
  );

  const dragExcludedSelector = [
    "a",
    "button",
    "input",
    "select",
    "textarea",
    "[contenteditable='true']",
    "[data-window-drag-exclude]",
  ].join(",");

  const handleWindowDrag = (event: MouseEvent) => {
    if (event.button !== 0 || event.buttons !== 1) return;

    const target = event.target;
    if (!(target instanceof Element)) return;
    if (!target.closest("[data-window-drag-region]")) return;
    if (target.closest(dragExcludedSelector)) return;

    getCurrentWindow()
      .startDragging()
      .catch((error) => {
        console.error("Failed to start window dragging:", error);
      });
  };

  const navigateToSkills = (tab: "local" | "remote", selectedScopeKey = "global") => {
    let projectPath: string | null = null;
    if (tab === "local" && selectedScopeKey.startsWith("project:")) {
      try {
        projectPath = decodeURIComponent(selectedScopeKey.slice("project:".length));
      } catch {
        projectPath = null;
      }
    }
    goto(buildSkillsHref(tab, projectPath));
  };

  const refreshCurrentSkills = () =>
    refreshLocalSkills({
      scope: skillsLocation.scope,
      project_path: skillsLocation.projectPath,
    });

  $effect(() => {
    const action = page.url.searchParams.get("action");
    if (action === "add") {
      addSkillModalOpen = true;
    } else if (action === "manage-projects") {
      userProjectsModalOpen = true;
    }
  });

  $effect(() => {
    if (userProjectsModalOpen) {
      userProjectsModalWasOpen = true;
      return;
    }
    if (userProjectsModalWasOpen) {
      userProjectsModalWasOpen = false;
      refreshUserProjects().catch(console.error);
    }
  });

  onMount(() => {
    if (!browser) {
      return () => {};
    }

    const allowedThemes = new Set<string>(["light", "dark", "system"]);
    const savedTheme = localStorage.getItem("theme");
    if (savedTheme && allowedThemes.has(savedTheme)) {
      // You can set theme via store or CSS variables
      // For now, we'll use the existing settings store
    } else {
      localStorage.setItem("theme", "system");
    }

    let unlistenOpenInstallModal: UnlistenFn | null = null;

    // Load shared application state without blocking the first render.
    loadSettings().catch(console.error);
    loadAgents().catch(console.error);
    refreshUserProjects().catch(console.error);
    ensureUpdateChecked().catch(console.error);
    listen("open-install-modal", () => {
      addSkillModalOpen = true;
    })
      .then((unlisten) => {
        unlistenOpenInstallModal = unlisten;
      })
      .catch(console.error);
    document.addEventListener("mousedown", handleWindowDrag);

    return () => {
      unlistenOpenInstallModal?.();
      document.removeEventListener("mousedown", handleWindowDrag);
    };
  });
</script>

<div
  class="bg-base-100 text-base-content grid h-dvh min-h-0 grid-cols-[15.5rem_minmax(0,1fr)] overflow-hidden max-[832px]:grid-cols-[13.5rem_minmax(0,1fr)]"
>
  <AppSidebar
    activeTab={skillsLocation.tab}
    {currentSection}
    {scopeKey}
    projects={$userProjects}
    hasUpdate={$updaterState.hasUpdate}
    updateLoading={$updaterState.installing}
    onAddSkill={() => (addSkillModalOpen = true)}
    onSelectTab={navigateToSkills}
    onOpenUpdate={() => installAvailableUpdate().catch(console.error)}
    onOpenProjectManage={() => (userProjectsModalOpen = true)}
    onOpenSettings={() => goto("/settings")}
  />

  <section class="flex min-h-0 min-w-0 flex-col overflow-hidden">
    {@render children()}
  </section>
</div>

<AddSkillModal
  bind:open={addSkillModalOpen}
  agents={$agentsStore}
  initialScope={skillsLocation.scope}
  initialProjectPath={skillsLocation.projectPath}
  onSuccess={refreshCurrentSkills}
/>
<UserProjectFormModal bind:open={userProjectsModalOpen} />
