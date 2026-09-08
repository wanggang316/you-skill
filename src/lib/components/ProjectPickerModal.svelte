<script lang="ts">
  import { Search } from "@lucide/svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "../i18n";
  import type { UserProject } from "../api/user-projects";
  import { hubSkillsByName } from "../stores/hub";
  import { closeProjectPickerModal, openInstallModal, projectPickerModal } from "../stores/modals";
  import { userProjects, workspaces } from "../stores/user-projects";

  let open = $state(false);
  let search = $state("");
  let selected = $state<string[]>([]);

  const skillName = $derived($projectPickerModal.skillName);

  /** Projects that already have the skill; listed for context, not selectable. */
  const installed = $derived(
    new Set(
      ($hubSkillsByName.get(skillName)?.installs ?? [])
        .filter((install) => install.scope === "project" && install.projectPath)
        .map((install) => install.projectPath as string)
    )
  );

  const candidates = $derived.by((): UserProject[] => {
    const needle = search.trim().toLowerCase();
    return $userProjects.filter(
      (project) => !needle || `${project.name} ${project.path}`.toLowerCase().includes(needle)
    );
  });

  type Group = { key: string; label: string | null; projects: UserProject[] };

  /** Candidates under the workspace they were found in, then the rest. */
  const groups = $derived.by((): Group[] => {
    const known = new Set($workspaces.map((workspace) => workspace.path));
    const result: Group[] = $workspaces
      .map((workspace) => ({
        key: workspace.path,
        label: workspace.name,
        projects: candidates.filter((project) => project.workspacePath === workspace.path),
      }))
      .filter((group) => group.projects.length > 0);
    const rest = candidates.filter(
      (project) => !project.workspacePath || !known.has(project.workspacePath)
    );
    if (rest.length > 0) {
      result.push({
        key: "other",
        label: $workspaces.length > 0 ? $t("workspace.other") : null,
        projects: rest,
      });
    }
    return result;
  });

  $effect(() => {
    const state = $projectPickerModal;
    if (state.open && !open) {
      search = "";
      selected = [];
    }
    open = state.open;
  });

  function toggle(path: string) {
    selected = selected.includes(path)
      ? selected.filter((item) => item !== path)
      : [...selected, path];
  }

  function handleClose() {
    closeProjectPickerModal();
    open = false;
  }

  function handleNext() {
    const name = skillName;
    const paths = [...selected];
    handleClose();
    if (!name || paths.length === 0) return;
    openInstallModal([name], { scope: "project", projectPaths: paths, lockScope: true });
  }
</script>

<Modal
  bind:open
  title={$t("projectPicker.title", { name: skillName })}
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
          placeholder={$t("projectPicker.search")}
          bind:value={search}
        />
      </div>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto px-3 py-2">
      {#if candidates.length === 0}
        <p class="text-base-content-muted px-3 py-8 text-center text-xs">
          {#if $userProjects.length === 0}
            {$t("install.noProjects")}
          {:else}
            {$t("library.emptyFiltered")}
          {/if}
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
          {#each group.projects as project (project.path)}
            {@const has = installed.has(project.path)}
            {@const checked = has || selected.includes(project.path)}
            <label
              class={`flex items-center gap-2.5 rounded-lg px-2.5 py-1.5 transition ${
                has ? "opacity-60" : "hover:bg-base-200 cursor-pointer"
              } ${checked && !has ? "bg-base-200" : ""}`}
              title={project.path}
            >
              <input
                class="accent-primary"
                type="checkbox"
                {checked}
                disabled={has}
                onchange={() => toggle(project.path)}
              />
              <span class="min-w-0 flex-1">
                <span class="flex min-w-0 items-center gap-1.5">
                  <span class="text-base-content truncate text-[13px] font-medium">
                    {project.name}
                  </span>
                  {#if has}
                    <span class="tag tag-neutral shrink-0">{$t("projectPicker.installed")}</span>
                  {/if}
                </span>
                <span class="text-base-content-faint block truncate text-[11px]">
                  {project.path}
                </span>
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
    <PrimaryActionButton onclick={handleNext} disabled={selected.length === 0}>
      {$t("picker.next")}
    </PrimaryActionButton>
  {/snippet}
</Modal>
