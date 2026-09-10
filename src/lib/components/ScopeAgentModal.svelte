<script lang="ts">
  import { AlertCircle } from "@lucide/svelte";
  import { get } from "svelte/store";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import SelectField from "$lib/components/ui/SelectField.svelte";
  import AgentPicker from "./AgentPicker.svelte";
  import { t } from "../i18n";
  import { installSkill, type InstallMode } from "../api/hub";
  import { agents, refreshHub } from "../stores/hub";
  import { closeScopeAgentModal, performAction, scopeAgentModal } from "../stores/modals";
  import { settings } from "../stores/settings";

  let open = $state(false);
  let selectedIds = $state<string[]>([]);
  let mode = $state<InstallMode>("copy");
  let applying = $state(false);
  let error = $state("");

  const modalState = $derived($scopeAgentModal);
  const candidates = $derived(
    $agents.filter((agent) => {
      if (modalState.existingIds.includes(agent.id)) return false;
      return modalState.scope?.scope === "project"
        ? Boolean(agent.project_path)
        : Boolean(agent.global_path);
    })
  );

  $effect(() => {
    const next = $scopeAgentModal;
    if (next.open && !open) {
      selectedIds = [];
      mode = get(settings).sync_mode === "symlink" ? "symlink" : "copy";
      error = "";
      applying = false;
    }
    open = next.open;
  });

  function handleClose() {
    open = false;
    closeScopeAgentModal();
  }

  async function handleApply() {
    const scope = modalState.scope;
    if (!scope || selectedIds.length === 0 || applying) return;
    applying = true;
    error = "";
    const targets = selectedIds.map((agentId) => ({
      scope: scope.scope,
      projectPath: scope.projectPath,
      agentId,
    }));
    const failures: string[] = [];
    try {
      for (const name of modalState.skillNames) {
        const result = await performAction((force) => installSkill({ name, targets, mode, force }));
        if (!result.applied) failures.push(`${name}: ${result.blockers.join("; ")}`);
      }
      await refreshHub();
      if (failures.length > 0) {
        error = failures.join("\n");
        return;
      }
      handleClose();
    } catch (err) {
      error = String(err);
      await refreshHub().catch(console.error);
    } finally {
      applying = false;
    }
  }
</script>

<Modal
  bind:open
  title={$t("agentAdd.title", { name: modalState.scopeName })}
  onClose={handleClose}
  containerClass="max-w-xl"
>
  <div class="space-y-5 px-6 pt-2 pb-6">
    <p class="text-base-content-muted text-[13px]">
      {$t("agentAdd.hint", { count: modalState.skillNames.length })}
    </p>

    {#if candidates.length === 0}
      <p class="text-base-content-muted text-sm">{$t("agentAdd.empty")}</p>
    {:else}
      <AgentPicker
        agents={candidates}
        scope={modalState.scope?.scope ?? "user"}
        bind:selectedIds
        disabled={applying}
      />
      <div class="flex items-center justify-between gap-3">
        <p class="text-base-content-muted text-[13px]">{$t("install.mode")}</p>
        <SelectField bind:value={mode} disabled={applying}>
          <option value="copy">{$t("install.mode.copy")}</option>
          <option value="symlink">{$t("install.mode.symlink")}</option>
        </SelectField>
      </div>
    {/if}

    {#if error}
      <div class="text-error flex items-start gap-2 text-sm whitespace-pre-wrap">
        <AlertCircle size={16} class="mt-0.5 shrink-0" />
        <span>{error}</span>
      </div>
    {/if}
  </div>
  {#snippet footer()}
    <button
      class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
      type="button"
      onclick={handleClose}
      disabled={applying}
    >
      {$t("common.cancel")}
    </button>
    <PrimaryActionButton
      onclick={handleApply}
      disabled={selectedIds.length === 0 || applying}
      loading={applying}
      loadingText={$t("install.applying")}
    >
      {$t("install.confirm")}
    </PrimaryActionButton>
  {/snippet}
</Modal>
