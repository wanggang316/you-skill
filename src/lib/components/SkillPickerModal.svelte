<script lang="ts">
  import { Search } from "@lucide/svelte";
  import SkillIcon from "./SkillIcon.svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "../i18n";
  import { scopedInstalls, type HubSkillView } from "../api/hub";
  import { hubSkills } from "../stores/hub";
  import { closeSkillPickerModal, openInstallModal, skillPickerModal } from "../stores/modals";
  import { userProjects } from "../stores/user-projects";

  let open = $state(false);
  let search = $state("");
  let selected = $state<string[]>([]);

  const scopeRef = $derived($skillPickerModal.scope);
  const scopeName = $derived.by(() => {
    if (!scopeRef) return "";
    if (scopeRef.scope === "user") return $t("scope.user");
    const path = scopeRef.projectPath ?? "";
    return (
      $userProjects.find((project) => project.path === path)?.name ??
      path.split(/[/\\]/).filter(Boolean).pop() ??
      path
    );
  });

  const candidates = $derived.by((): HubSkillView[] => {
    const ref = scopeRef;
    if (!ref) return [];
    const needle = search.trim().toLowerCase();
    return $hubSkills.filter((skill) => {
      if (scopedInstalls(skill, ref).length > 0) return false;
      if (!needle) return true;
      return `${skill.name} ${skill.description ?? ""}`.toLowerCase().includes(needle);
    });
  });

  $effect(() => {
    const state = $skillPickerModal;
    if (state.open && !open) {
      search = "";
      selected = [];
    }
    open = state.open;
  });

  function toggle(name: string) {
    selected = selected.includes(name)
      ? selected.filter((item) => item !== name)
      : [...selected, name];
  }

  function handleClose() {
    closeSkillPickerModal();
    open = false;
  }

  function handleNext() {
    const ref = scopeRef;
    const names = [...selected];
    handleClose();
    if (!ref || names.length === 0) return;
    openInstallModal(names, { scope: ref.scope, projectPath: ref.projectPath, lockScope: true });
  }
</script>

<Modal
  bind:open
  title={$t("picker.title", { name: scopeName })}
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
          placeholder={$t("library.search")}
          bind:value={search}
        />
      </div>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto px-3 py-2">
      {#if candidates.length === 0}
        <p class="text-base-content-muted px-3 py-8 text-center text-xs">
          {search.trim() ? $t("library.emptyFiltered") : $t("picker.empty")}
        </p>
      {:else}
        <div class="grid grid-cols-2 gap-2">
          {#each candidates as skill (skill.name)}
            {@const checked = selected.includes(skill.name)}
            <label
              class={`flex cursor-pointer flex-col gap-1 rounded-xl border px-3 py-2.5 transition ${
                checked
                  ? "border-primary/60 bg-primary/10"
                  : "border-base-300 bg-base-100 hover:bg-base-200"
              }`}
              title={skill.description ?? skill.name}
            >
              <span class="flex min-w-0 items-center gap-2">
                <input
                  class="accent-primary"
                  type="checkbox"
                  {checked}
                  onchange={() => toggle(skill.name)}
                />
                <span class="text-base-content-subtle shrink-0">
                  <SkillIcon source={skill.source} />
                </span>
                <span class="text-base-content min-w-0 flex-1 truncate text-[13px] font-medium">
                  {skill.name}
                </span>
              </span>
              <span
                class="text-base-content-subtle line-clamp-2 min-h-[2lh] text-[11px] leading-[1.35]"
              >
                {skill.description ?? ""}
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
