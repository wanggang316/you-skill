<script lang="ts">
  import { get } from "svelte/store";
  import { t } from "../i18n";
  import Modal from "./ui/Modal.svelte";
  import AgentSelector from "./AgentSelector.svelte";
  import PrimaryActionButton from "./ui/PrimaryActionButton.svelte";
  import SelectField from "./ui/SelectField.svelte";
  import { settings } from "../stores/settings";
  import { listUserProjects } from "../api/user-projects";
  import type { InstallScope } from "../api/skills";

  interface Props {
    open?: boolean;
    title?: string;
    confirmText?: string;
    agents?: Array<{ id: string }>;
    initialSelection?: string[];
    allowScopeChange?: boolean;
    initialScope?: InstallScope;
    initialProjectPath?: string | null;
    onConfirm?: (
      selectedAgents: string[],
      method: "symlink" | "copy",
      scope: InstallScope,
      projectPath: string | null
    ) => Promise<boolean>;
    onCancel?: () => void;
  }

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
  }: Props = $props();

  let selectedAgents = $state<string[]>([]);
  let selectedMethod = $state<"symlink" | "copy">("symlink");
  let selectedScope = $state<InstallScope>("global");
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
      selectedScope = initialScope;
      selectedProjectPath = initialScope === "project" ? initialProjectPath : null;
      void loadProjects();
    }
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
  const scopeLabel = $derived.by(() => {
    if (selectedScope !== "project") return $t("installScope.global");
    if (!selectedProjectPath) return $t("installScope.project");
    return (
      userProjects.find((project) => project.path === selectedProjectPath)?.name ??
      $t("installScope.project")
    );
  });
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
    <div class="text-base-content-subtle mr-auto text-xs">{scopeLabel}</div>
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
