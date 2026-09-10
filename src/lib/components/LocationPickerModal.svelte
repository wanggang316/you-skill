<script lang="ts">
  import { Search } from "@lucide/svelte";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "../i18n";
  import { scopedInstalls, uninstallSkill, type InstallView, type ScopeRef } from "../api/hub";
  import { uninstallInstruction } from "../api/instructions";
  import { baseName } from "../scopes";
  import { homePath } from "../stores/env";
  import { hubSkillsByName, refreshHub } from "../stores/hub";
  import { instructionsByName, refreshInstructions } from "../stores/instructions";
  import {
    closeLocationPickerModal,
    locationPickerModal,
    openInstallModal,
    performAction,
    performInstructionAction,
  } from "../stores/modals";
  import { userProjects, workspaces } from "../stores/user-projects";

  type Location = {
    key: string;
    ref: ScopeRef;
    name: string;
    path: string;
    installs: InstallView[];
  };
  type Group = { key: string; label: string | null; locations: Location[] };
  type ContextMenu = { x: number; y: number; location: Location };

  let open = $state(false);
  let search = $state("");
  let selected = $state<string[]>([]);
  let menu = $state<ContextMenu | null>(null);
  let error = $state("");

  const skillName = $derived($locationPickerModal.skillName);
  const kind = $derived($locationPickerModal.kind);
  const isInstruction = $derived(kind === "instruction");
  const skill = $derived(
    isInstruction ? $instructionsByName.get(skillName) : $hubSkillsByName.get(skillName)
  );
  const refresh = () => (isInstruction ? refreshInstructions() : refreshHub());

  const matches = (location: { name: string; path: string }) => {
    const needle = search.trim().toLowerCase();
    return !needle || `${location.name} ${location.path}`.toLowerCase().includes(needle);
  };

  const location = (ref: ScopeRef, name: string, path: string): Location => ({
    key: ref.scope === "user" ? "user" : `project:${path}`,
    ref,
    name,
    path,
    installs: skill ? scopedInstalls(skill, ref) : [],
  });

  /** The user level first, then every project under its workspace, then the rest. */
  const groups = $derived.by((): Group[] => {
    const result: Group[] = [];
    const user = location(
      { scope: "user", projectPath: null },
      baseName($homePath) || $t("scope.user"),
      $homePath
    );
    if (matches(user)) result.push({ key: "user", label: null, locations: [user] });

    const projects = $userProjects
      .map((project) =>
        location({ scope: "project", projectPath: project.path }, project.name, project.path)
      )
      .filter(matches);
    const known = new Set($workspaces.map((workspace) => workspace.path));
    for (const workspace of $workspaces) {
      const locations = projects.filter((item) => {
        const project = $userProjects.find((candidate) => candidate.path === item.path);
        return project?.workspacePath === workspace.path;
      });
      if (locations.length > 0)
        result.push({ key: workspace.path, label: workspace.name, locations });
    }
    const rest = projects.filter((item) => {
      const project = $userProjects.find((candidate) => candidate.path === item.path);
      return !project?.workspacePath || !known.has(project.workspacePath);
    });
    if (rest.length > 0) {
      result.push({
        key: "other",
        label: $workspaces.length > 0 ? $t("workspace.other") : null,
        locations: rest,
      });
    }
    return result;
  });

  const allLocations = $derived(groups.flatMap((group) => group.locations));

  /** Every location that has the skill, whether or not the search shows it. */
  const installedKeys = $derived.by((): string[] => {
    if (!skill) return [];
    const keys = skill.installs.map((install) =>
      install.scope === "user" ? "user" : `project:${install.projectPath ?? ""}`
    );
    return [...new Set(keys)];
  });
  const added = $derived(selected.filter((key) => !installedKeys.includes(key)));
  const removed = $derived(installedKeys.filter((key) => !selected.includes(key)));
  const hasChanges = $derived(added.length > 0 || removed.length > 0);

  $effect(() => {
    const state = $locationPickerModal;
    if (state.open && !open) {
      search = "";
      // The checklist starts as the current state: checked means installed there.
      selected = [...installedKeys];
      menu = null;
      error = "";
    }
    open = state.open;
  });

  // The context menu closes on any click elsewhere, Escape or scrolling.
  $effect(() => {
    if (!menu) return;
    const close = () => (menu = null);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", onKeyDown);
    window.addEventListener("scroll", close, true);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("scroll", close, true);
    };
  });

  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return {
      destroy() {
        node.remove();
      },
    };
  }

  function toggle(key: string) {
    selected = selected.includes(key)
      ? selected.filter((item) => item !== key)
      : [...selected, key];
  }

  function openMenu(event: MouseEvent, item: Location) {
    event.preventDefault();
    menu = { x: event.clientX, y: event.clientY, location: item };
  }

  function handleClose() {
    closeLocationPickerModal();
    open = false;
  }

  function manage(item: Location) {
    const name = skillName;
    handleClose();
    openInstallModal([name], { targets: [item.ref], lockScope: true, kind });
  }

  async function uninstall(item: Location) {
    const paths = item.installs.map((install) => install.path);
    if (paths.length === 0) return;
    const confirmed = await confirm(
      $t("locationPicker.uninstallConfirm", { name: skillName, location: item.name }),
      { title: $t("scope.uninstall"), kind: "warning" }
    );
    if (!confirmed) return;
    error = "";
    try {
      const result = isInstruction
        ? await performInstructionAction((force) =>
            uninstallInstruction({ name: skillName, targets: [], paths, force })
          )
        : await performAction((force) =>
            uninstallSkill({ name: skillName, targets: [], paths, force })
          );
      if (!result.applied) error = result.blockers.join("; ");
      await refresh();
    } catch (err) {
      error = String(err);
    }
  }

  /** Unchecked places are uninstalled first; newly checked ones go on to the agent dialog. */
  async function handleNext() {
    const name = skillName;
    const view = skill;
    if (!name || !view || !hasChanges) return;
    const toRemove = view.installs.filter((install) =>
      removed.includes(install.scope === "user" ? "user" : `project:${install.projectPath ?? ""}`)
    );
    if (toRemove.length > 0) {
      const confirmed = await confirm(
        $t("locationPicker.uninstallCount", { name, count: removed.length }),
        { title: $t("scope.uninstall"), kind: "warning" }
      );
      if (!confirmed) return;
      error = "";
      try {
        const paths = toRemove.map((install) => install.path);
        const result = isInstruction
          ? await performInstructionAction((force) =>
              uninstallInstruction({ name, targets: [], paths, force })
            )
          : await performAction((force) => uninstallSkill({ name, targets: [], paths, force }));
        await refresh();
        if (!result.applied) {
          error = result.blockers.join("; ");
          return;
        }
      } catch (err) {
        error = String(err);
        return;
      }
    }
    const targets = allLocations.filter((item) => added.includes(item.key)).map((item) => item.ref);
    handleClose();
    if (targets.length > 0) openInstallModal([name], { targets, lockScope: true, kind });
  }
</script>

<Modal
  bind:open
  title={$t("locationPicker.title", { name: skillName })}
  onClose={handleClose}
  containerClass="max-w-lg"
>
  <div class="flex h-[60vh] min-h-0 flex-col">
    <div class="border-base-200 flex-none border-b px-5 py-3">
      <div class="relative">
        <Search
          class="text-base-content-subtle absolute top-1/2 left-3 -translate-y-1/2"
          size={14}
        />
        <input
          class="border-base-300 bg-base-200 text-base-content placeholder:text-base-content-subtle focus:border-base-300 h-8 w-full rounded-xl border pr-3 pl-8 text-[13px] focus:outline-none"
          placeholder={$t("locationPicker.search")}
          bind:value={search}
        />
      </div>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto px-3 py-2">
      {#if error}
        <p class="text-error px-2.5 py-1 text-xs whitespace-pre-wrap">{error}</p>
      {/if}
      {#if allLocations.length === 0}
        <p class="text-base-content-muted px-3 py-8 text-center text-xs">
          {$t("library.emptyFiltered")}
        </p>
      {:else}
        {#each groups as group (group.key)}
          {#if group.label}
            <p
              class="text-base-content-subtle truncate px-2.5 pt-2 pb-1 text-[11px]"
              title={group.key}
            >
              {group.label}
            </p>
          {/if}
          {#each group.locations as item (item.key)}
            {@const checked = selected.includes(item.key)}
            <label
              class={`mb-1.5 flex cursor-pointer items-center gap-2.5 rounded-xl border px-3 py-2 transition ${
                checked
                  ? "border-primary/60 bg-primary/10"
                  : "border-base-300 bg-base-100 hover:bg-base-200"
              }`}
              title={item.path}
              oncontextmenu={(event) => openMenu(event, item)}
            >
              <input
                class="accent-primary"
                type="checkbox"
                {checked}
                onchange={() => toggle(item.key)}
              />
              <span class="min-w-0 flex-1">
                <span class="text-base-content block truncate text-[13px] font-medium">
                  {item.name}
                </span>
                <span class="text-base-content-faint block truncate text-[11px]">{item.path}</span>
              </span>
            </label>
          {/each}
        {/each}
      {/if}
    </div>
  </div>
  {#snippet footer()}
    <span class="text-base-content-muted mr-auto text-xs">
      {$t("picker.selected", { count: selected.length })}
    </span>
    <button
      class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
      type="button"
      onclick={handleClose}
    >
      {$t("common.cancel")}
    </button>
    <PrimaryActionButton onclick={() => void handleNext()} disabled={!hasChanges}>
      {added.length > 0 ? $t("picker.next") : $t("install.confirm")}
    </PrimaryActionButton>
  {/snippet}
</Modal>

{#if menu}
  {@const item = menu.location}
  <div
    use:portal
    role="menu"
    tabindex="-1"
    class="border-base-300 bg-base-100 fixed z-[10020] min-w-28 rounded-xl border p-1 shadow-lg"
    style={`left:${Math.min(menu.x, window.innerWidth - 160)}px; top:${Math.min(menu.y, window.innerHeight - 96)}px;`}
    onmousedown={(event) => event.stopPropagation()}
  >
    <button
      class="text-base-content hover:bg-base-200 w-full rounded-lg px-2.5 py-1.5 text-left text-[13px] whitespace-nowrap transition"
      type="button"
      role="menuitem"
      onclick={() => {
        menu = null;
        manage(item);
      }}
    >
      {$t("scope.skill.manage")}
    </button>
    <button
      class="text-error hover:bg-error/10 w-full rounded-lg px-2.5 py-1.5 text-left text-[13px] whitespace-nowrap transition disabled:opacity-40"
      type="button"
      role="menuitem"
      disabled={item.installs.length === 0}
      onclick={() => {
        menu = null;
        void uninstall(item);
      }}
    >
      {$t("scope.uninstall")}
    </button>
  </div>
{/if}
