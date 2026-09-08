<script lang="ts">
  import { Folder, Search } from "@lucide/svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "../i18n";
  import type { UserProject } from "../api/user-projects";
  import { hubSkillsByName } from "../stores/hub";
  import { closeProjectPickerModal, openInstallModal, projectPickerModal } from "../stores/modals";
  import { userProjects } from "../stores/user-projects";

  let open = $state(false);
  let search = $state("");
  let selected = $state<string[]>([]);

  const skillName = $derived($projectPickerModal.skillName);

  /** Registered projects that do not have the skill yet. */
  const candidates = $derived.by((): UserProject[] => {
    const skill = $hubSkillsByName.get(skillName);
    const taken = new Set(
      (skill?.installs ?? [])
        .filter((install) => install.scope === "project" && install.projectPath)
        .map((install) => install.projectPath as string)
    );
    const needle = search.trim().toLowerCase();
    return $userProjects.filter((project) => {
      if (taken.has(project.path)) return false;
      if (!needle) return true;
      return `${project.name} ${project.path}`.toLowerCase().includes(needle);
    });
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
  containerClass="max-w-2xl"
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
          {:else if search.trim()}
            {$t("library.emptyFiltered")}
          {:else}
            {$t("projectPicker.empty")}
          {/if}
        </p>
      {:else}
        <div class="grid grid-cols-2 gap-2">
          {#each candidates as project (project.path)}
            {@const checked = selected.includes(project.path)}
            <label
              class={`flex cursor-pointer items-center gap-2.5 rounded-xl border px-3 py-2.5 transition ${
                checked
                  ? "border-primary/60 bg-primary/10"
                  : "border-base-300 bg-base-100 hover:bg-base-200"
              }`}
              title={project.path}
            >
              <input
                class="accent-primary"
                type="checkbox"
                {checked}
                onchange={() => toggle(project.path)}
              />
              <span class="text-base-content-subtle shrink-0"><Folder size={15} /></span>
              <span class="min-w-0 flex-1">
                <span class="text-base-content block truncate text-[13px] font-medium">
                  {project.name}
                </span>
                <span class="text-base-content-faint block truncate text-[11px]">
                  {project.path}
                </span>
              </span>
            </label>
          {/each}
        </div>
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
