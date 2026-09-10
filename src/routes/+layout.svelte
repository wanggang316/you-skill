<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import { afterNavigate } from "$app/navigation";
  import { page } from "$app/state";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import AppSidebar from "$lib/components/AppSidebar.svelte";
  import DiffModal from "$lib/components/DiffModal.svelte";
  import ForceConfirmModal from "$lib/components/ForceConfirmModal.svelte";
  import ImportInstructionModal from "$lib/components/ImportInstructionModal.svelte";
  import ImportSkillModal from "$lib/components/ImportSkillModal.svelte";
  import InstallSkillModal from "$lib/components/InstallSkillModal.svelte";
  import LocationPickerModal from "$lib/components/LocationPickerModal.svelte";
  import ScopeAgentModal from "$lib/components/ScopeAgentModal.svelte";
  import SkillPickerModal from "$lib/components/SkillPickerModal.svelte";
  import WorkspaceModal from "$lib/components/WorkspaceModal.svelte";
  import { getAppLocation } from "$lib/navigation/app-shell";
  import { goBack, goForward, trackNavigation } from "$lib/navigation/history";
  import { readSidebarWidth, SIDEBAR, writeSidebarWidth } from "$lib/navigation/sidebar";
  import { t } from "$lib/i18n";
  import { loadHomePath } from "$lib/stores/env";
  import { loadAgents, loadMigrationReport, refreshHub } from "$lib/stores/hub";
  import { refreshInstructions } from "$lib/stores/instructions";
  import { openImportModal } from "$lib/stores/modals";
  import { loadSettings } from "$lib/stores/settings";
  import { ensureUpdateChecked, installAvailableUpdate, updaterState } from "$lib/stores/updater";
  import { refreshUserProjects, refreshWorkspaces } from "$lib/stores/user-projects";

  let { children } = $props();

  const location = $derived(getAppLocation(page.url));
  let sidebarWidth = $state(readSidebarWidth());
  let resizing = $state(false);
  const sidebarCollapsed = $derived(sidebarWidth <= SIDEBAR.collapsed);

  afterNavigate(trackNavigation);

  /** Drag the sidebar edge; below the threshold it snaps to icons only. */
  function startSidebarResize(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    resizing = true;
    const handle = event.currentTarget as HTMLElement;
    handle.setPointerCapture(event.pointerId);
    const onMove = (move: PointerEvent) => {
      const width = move.clientX;
      sidebarWidth =
        width < SIDEBAR.collapseBelow
          ? SIDEBAR.collapsed
          : Math.min(SIDEBAR.max, Math.max(SIDEBAR.min, width));
    };
    const onUp = () => {
      resizing = false;
      handle.releasePointerCapture(event.pointerId);
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", onUp);
      handle.removeEventListener("pointercancel", onUp);
      writeSidebarWidth(sidebarWidth);
    };
    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
    handle.addEventListener("pointercancel", onUp);
  }

  function resetSidebarWidth() {
    sidebarWidth = SIDEBAR.initial;
    writeSidebarWidth(sidebarWidth);
  }

  /** ⌘[ and ⌘] (Ctrl on other platforms) go back and forward, like a browser. */
  const handleHistoryKeys = (event: KeyboardEvent) => {
    if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) return;
    if (event.key === "[") {
      event.preventDefault();
      goBack();
    } else if (event.key === "]") {
      event.preventDefault();
      goForward();
    }
  };
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
    if (action === "add") openImportModal();
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
    refreshWorkspaces().catch(console.error);
    loadHomePath().catch(console.error);
    refreshHub()
      .then(() => loadMigrationReport())
      .catch(console.error);
    refreshInstructions().catch(console.error);
    ensureUpdateChecked().catch(console.error);
    listen("open-install-modal", () => {
      openImportModal();
    })
      .then((unlisten) => {
        unlistenOpenInstallModal = unlisten;
      })
      .catch(console.error);
    document.addEventListener("mousedown", handleWindowDrag);
    document.addEventListener("keydown", handleHistoryKeys);

    return () => {
      unlistenOpenInstallModal?.();
      document.removeEventListener("mousedown", handleWindowDrag);
      document.removeEventListener("keydown", handleHistoryKeys);
    };
  });
</script>

<div
  class={`bg-base-100 text-base-content relative grid h-dvh min-h-0 overflow-hidden ${
    resizing ? "cursor-col-resize select-none" : ""
  }`}
  style={`grid-template-columns: ${sidebarWidth}px minmax(0, 1fr);`}
>
  <AppSidebar
    activeKey={location.activeKey}
    collapsed={sidebarCollapsed}
    hasUpdate={$updaterState.hasUpdate}
    updateLoading={$updaterState.installing}
    onImportSkill={() => openImportModal()}
    onOpenUpdate={() => installAvailableUpdate().catch(console.error)}
  />

  <section class="flex min-h-0 min-w-0 flex-col overflow-hidden">
    {@render children()}
  </section>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="absolute top-0 bottom-0 z-20 w-1.5 -translate-x-1/2 cursor-col-resize"
    style={`left: ${sidebarWidth}px;`}
    title={$t("sidebar.resize")}
    onpointerdown={startSidebarResize}
    ondblclick={resetSidebarWidth}
  ></div>
</div>

<ImportSkillModal />
<ImportInstructionModal />
<InstallSkillModal />
<SkillPickerModal />
<LocationPickerModal />
<ScopeAgentModal />
<ForceConfirmModal />
<DiffModal />
<WorkspaceModal />
