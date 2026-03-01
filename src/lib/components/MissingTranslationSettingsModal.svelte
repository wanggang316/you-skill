<script lang="ts">
  import Modal from "./ui/Modal.svelte";
  import { t } from "$lib/i18n";

  let {
    open = $bindable(false),
    missingFields = [],
    onCancel = () => {},
    onSettings = () => {},
  } = $props<{
    open?: boolean;
    missingFields?: string[];
    onCancel?: () => void;
    onSettings?: () => void;
  }>();

  const handleCancel = () => {
    open = false;
    onCancel();
  };

  const handleSettings = () => {
    open = false;
    onSettings();
  };
</script>

<Modal
  bind:open
  title={$t("settings.translation.missingConfigTitle")}
  onClose={handleCancel}
  containerClass="max-w-md"
>
  <div class="space-y-3 px-6 py-6">
    <p class="text-base-content text-sm">{$t("settings.translation.missingConfigDescription")}</p>
    <ul class="text-base-content-muted list-disc space-y-1 pl-5 text-sm">
      {#each missingFields as field}
        <li>{field}</li>
      {/each}
    </ul>
  </div>
  {#snippet footer()}
    <button
      class="bg-base-300 text-base-content hover:bg-base-200 rounded-lg px-4 py-2 text-sm transition"
      onclick={handleCancel}
      type="button"
    >
      {$t("addSkill.cancel")}
    </button>
    <button
      class="bg-primary text-primary-content hover:bg-primary-hover rounded-lg px-4 py-2 text-sm transition"
      onclick={handleSettings}
      type="button"
    >
      {$t("agentApps.configButton")}
    </button>
  {/snippet}
</Modal>
