<script lang="ts">
  import { get } from "svelte/store";
  import { t } from "../i18n";
  import Modal from "./ui/Modal.svelte";
  import AgentSelector from "./AgentSelector.svelte";
  import PrimaryActionButton from "./ui/PrimaryActionButton.svelte";
  import SelectField from "./ui/SelectField.svelte";
  import InstallScopeSelect from "./InstallScopeSelect.svelte";
  import { settings } from "../stores/settings";
  import { listUserProjects } from "../api/user-projects";
  import type { InstallScope } from "../api/skills";

  let {
    open = $bindable(false),
    title = "",
    confirmText = "",
    agents = [],
    initialSelection = [],
    allowScopeChange = true,
    initialScope = "global",
    initialProjectPath = null,
    onConfirm = async () => true,
    onCancel = () => {},
  } = $props();

  let selectedAgents = $state<string[]>([]);
  let selectedMethod = $state<"symlink" | "copy">("symlink");
  let selectedScope = $state<InstallScope>("global");
  let selectedScopeKey = $state("global");
  let userProjects = $state<import("../api/user-projects").UserProject[]>([]);
  let selectedProjectPath = $state<string | null>(null);
  let isInstalling = $state(false);

  // Reset state when modal opens
  $effect(() => {
    if (open) {
      // Use initialSelection if provided, otherwise select all
      selectedAgents =
        initialSelection.length > 0 ? [...initialSelection] : agents.map((a) => a.id);
      selectedMethod = get(settings).sync_mode || "symlink";
      if (initialScope === "project" && initialProjectPath) {
        selectedScopeKey = `project:${encodeURIComponent(initialProjectPath)}`;
      } else {
        selectedScopeKey = "global";
      }
      if (allowScopeChange || initialScope === "project") {
        void loadProjects();
      }
    }
  });

  $effect(() => {
    if (selectedScopeKey.startsWith("project:")) {
      selectedScope = "project";
      selectedProjectPath = decodeURIComponent(selectedScopeKey.slice("project:".length));
      return;
    }
    selectedScope = "global";
    selectedProjectPath = null;
  });

  async function loadProjects() {
    try {
      userProjects = await listUserProjects();
    } catch {
      userProjects = [];
    }
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
    <!-- Agent List -->
    <AgentSelector {agents} bind:selectedIds={selectedAgents} />
  </div>
  {#snippet footer()}
    {#if allowScopeChange}
      <InstallScopeSelect bind:value={selectedScopeKey} projects={userProjects} className="mr-3" />
    {/if}
    <SelectField bind:value={selectedMethod} className="mr-3" disabled={isInstalling}>
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
