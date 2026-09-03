<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import { page } from "$app/state";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import AppSidebar from "$lib/components/AppSidebar.svelte";
  import DiffModal from "$lib/components/DiffModal.svelte";
  import ForceConfirmModal from "$lib/components/ForceConfirmModal.svelte";
  import ImportSkillModal from "$lib/components/ImportSkillModal.svelte";
  import InstallSkillModal from "$lib/components/InstallSkillModal.svelte";
  import SkillPickerModal from "$lib/components/SkillPickerModal.svelte";
  import UserProjectFormModal from "$lib/components/UserProjectFormModal.svelte";
  import { getAppLocation } from "$lib/navigation/app-shell";
  import { loadAgents, loadMigrationReport, refreshHub } from "$lib/stores/hub";
  import { openImportModal } from "$lib/stores/modals";
  import { loadSettings } from "$lib/stores/settings";
  import { ensureUpdateChecked, installAvailableUpdate, updaterState } from "$lib/stores/updater";
  import { refreshUserProjects, userProjects } from "$lib/stores/user-projects";

  let { children } = $props();
  let userProjectsModalOpen = $state(false);
  let userProjectsModalWasOpen = $state(false);

  const location = $derived(getAppLocation(page.url));
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

  $effect(() => {
    const action = page.url.searchParams.get("action");
    if (action === "add") {
      openImportModal();
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
      refreshUserProjects()
        .then(() => refreshHub())
        .catch(console.error);
    }
  });

  onMount(() => {
    if (!browser) {
      return () => {};
    }

    let unlistenOpenInstallModal: UnlistenFn | null = null;

    // Load shared application state without blocking the first render.
    loadSettings().catch(console.error);
    loadAgents().catch(console.error);
    refreshUserProjects().catch(console.error);
    refreshHub()
      .then(() => loadMigrationReport())
      .catch(console.error);
    ensureUpdateChecked().catch(console.error);
    listen("open-install-modal", () => {
      openImportModal();
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
    activeKey={location.activeKey}
    projects={$userProjects}
    hasUpdate={$updaterState.hasUpdate}
    updateLoading={$updaterState.installing}
    onImportSkill={() => openImportModal()}
    onOpenUpdate={() => installAvailableUpdate().catch(console.error)}
    onOpenProjectManage={() => (userProjectsModalOpen = true)}
  />

  <section class="flex min-h-0 min-w-0 flex-col overflow-hidden">
    {@render children()}
  </section>
</div>

<ImportSkillModal />
<InstallSkillModal />
<SkillPickerModal />
<ForceConfirmModal />
<DiffModal />
<UserProjectFormModal bind:open={userProjectsModalOpen} />
