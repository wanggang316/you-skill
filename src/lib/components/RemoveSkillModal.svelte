<script lang="ts">
  import { AlertCircle } from "@lucide/svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "../i18n";
  import { removeHubSkill, type HubSkillView } from "../api/hub";

  let {
    open = $bindable(false),
    skill = null,
    onRemoved = () => {},
  }: {
    open?: boolean;
    skill?: HubSkillView | null;
    onRemoved?: () => void | Promise<void>;
  } = $props();

  let removeInstalls = $state(false);
  let removing = $state(false);
  let error = $state("");

  $effect(() => {
    if (open) {
      removeInstalls = false;
      removing = false;
      error = "";
    }
  });

  async function handleRemove() {
    if (!skill) return;
    removing = true;
    error = "";
    try {
      await removeHubSkill(skill.name, removeInstalls);
      open = false;
      await onRemoved();
    } catch (err) {
      error = String(err);
    } finally {
      removing = false;
    }
  }
</script>

<Modal
  bind:open
  title={$t("remove.title", { name: skill?.name ?? "" })}
  onClose={() => (open = false)}
  containerClass="max-w-md"
>
  <div class="space-y-4 px-6 pt-2 pb-6 text-sm">
    <p class="text-base-content-muted">{$t("remove.description")}</p>
    {#if skill && skill.installs.length > 0}
      <label class="text-base-content flex cursor-pointer items-start gap-2">
        <input type="checkbox" class="mt-0.5" bind:checked={removeInstalls} disabled={removing} />
        <span>
          {$t("remove.alsoInstalls", { count: skill.installs.length })}
          {#if !removeInstalls}
            <span class="text-base-content-faint block text-xs">
              {$t("remove.keepInstallsHint")}
            </span>
          {/if}
        </span>
      </label>
    {/if}
    {#if error}
      <div class="text-error flex items-start gap-2 text-sm">
        <AlertCircle size={16} class="mt-0.5 shrink-0" />
        <span>{error}</span>
      </div>
    {/if}
  </div>
  {#snippet footer()}
    <button
      class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
      type="button"
      onclick={() => (open = false)}
      disabled={removing}
    >
      {$t("common.cancel")}
    </button>
    <PrimaryActionButton
      onclick={handleRemove}
      loading={removing}
      loadingText={$t("remove.removing")}
      className="bg-error hover:bg-error/85 text-error-content"
    >
      {$t("remove.confirm")}
    </PrimaryActionButton>
  {/snippet}
</Modal>
