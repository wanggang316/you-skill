<script>
  import { get } from "svelte/store";
  import { t } from "../i18n";
  import Modal from "./ui/Modal.svelte";
  import AgentSelector from "./AgentSelector.svelte";
  import PrimaryActionButton from "./ui/PrimaryActionButton.svelte";
  import SelectField from "./ui/SelectField.svelte";
  import { settings } from "../stores/settings";
  import { listUserProjects } from "../api/user-projects";

  let {
    open = $bindable(false),
    title = "",
    confirmText = "",
    agents = [],
    initialSelection = [],
    onConfirm = async () => true,
    onCancel = () => {},
  } = $props();

  /** @type {string[]} */
  let selectedAgents = $state([]);
  /** @type {"symlink" | "copy"} */
  let selectedMethod = $state("symlink");
  /** @type {"global" | "project"} */
  let selectedScope = $state("global");
  /** @type {import('../api/user-projects').UserProject[]} */
  let userProjects = $state([]);
  /** @type {string | null} */
  let selectedProjectPath = $state(null);
  let projectLoading = $state(false);
  let isInstalling = $state(false);

  // Reset state when modal opens
  $effect(() => {
    if (open) {
      // Use initialSelection if provided, otherwise select all
      selectedAgents =
        initialSelection.length > 0 ? [...initialSelection] : agents.map((a) => a.id);
      selectedMethod = get(settings).sync_mode || "symlink";
      selectedScope = "global";
      selectedProjectPath = null;
      void loadProjects();
    }
  });

  async function loadProjects() {
    projectLoading = true;
    try {
      userProjects = await listUserProjects();
    } catch {
      userProjects = [];
    } finally {
      projectLoading = false;
    }
  }

  /** @param {string} projectPath */
  function toggleProject(projectPath) {
    selectedProjectPath = selectedProjectPath === projectPath ? null : projectPath;
  }

  function closeModal() {
    open = false;
    onCancel();
  }

  async function handleConfirm() {
    if (selectedAgents.length === 0) return;
    isInstalling = true;
    try {
      const shouldClose = await onConfirm(
        selectedAgents,
        selectedMethod,
        selectedScope,
        selectedProjectPath
      );
      if (shouldClose !== false) {
        closeModal();
      }
    } finally {
      isInstalling = false;
    }
  }

  const hasSelection = $derived(
    selectedAgents.length > 0 &&
      (selectedScope === "global" || !!selectedProjectPath)
  );
  const finalConfirmText = $derived(confirmText || $t("selectAgent.confirm"));
</script>

<Modal
  bind:open
  title={title || $t("selectAgent.defaultTitle")}
  onClose={closeModal}
  containerClass="max-w-lg min-w-md"
>
  <div class="w-full px-6 pt-2 pb-6">
    <p class="text-base-content-muted mb-4 text-sm">
      {$t("selectAgent.description")}
    </p>
    <div class="mb-4 space-y-3">
      <SelectField bind:value={selectedScope}>
        <option value="global">{$t("installScope.global")}</option>
        <option value="project">{$t("installScope.project")}</option>
      </SelectField>

      {#if selectedScope === "project"}
        <div class="space-y-2">
          <p class="text-base-content text-sm">
            {$t("selectAgent.selectProjects")}
          </p>
          {#if projectLoading}
            <p class="text-base-content-muted text-xs">{$t("projectManage.loading")}</p>
          {:else if userProjects.length === 0}
            <p class="text-base-content-muted text-xs">{$t("projectManage.empty")}</p>
          {:else}
            <div class="mt-1 flex flex-wrap gap-2">
              {#each userProjects as project}
                <label
                  class="bg-base-200 text-base-content hover:bg-base-300 inline-flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1 text-[13px] transition"
                >
                  <input
                    type="checkbox"
                    checked={selectedProjectPath === project.path}
                    onchange={() => toggleProject(project.path)}
                  />
                  <span>{project.name}</span>
                </label>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Agent List -->
    <AgentSelector {agents} bind:selectedIds={selectedAgents} />
  </div>
  {#snippet footer()}
    <SelectField bind:value={selectedMethod} className="mr-3">
        <option value="symlink">{$t("settings.syncMode.symlink")}</option>
        <option value="copy">{$t("settings.syncMode.copy")}</option>
    </SelectField>
    <PrimaryActionButton
      onclick={handleConfirm}
      disabled={!hasSelection || isInstalling}
      loading={isInstalling}
      loadingText={finalConfirmText}
      className="px-6"
    >
      {finalConfirmText}
    </PrimaryActionButton>
  {/snippet}
</Modal>
