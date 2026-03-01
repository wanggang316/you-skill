<script lang="ts">
  import Modal from "./ui/Modal.svelte";
  import PrimaryActionButton from "./ui/PrimaryActionButton.svelte";
  import { t } from "$lib/i18n";

  let {
    open = $bindable(false),
    apiKey = "",
    targetLanguage = "zh-CN",
    saving = false,
    onSave = async () => {},
    onCancel = () => {},
  } = $props<{
    open?: boolean;
    apiKey?: string;
    targetLanguage?: string;
    saving?: boolean;
    onSave?: (payload: { apiKey: string; targetLanguage: string }) => Promise<void> | void;
    onCancel?: () => void;
  }>();

  let draftApiKey = $state("");
  let draftTargetLanguage = $state("zh-CN");

  $effect(() => {
    if (open) {
      draftApiKey = apiKey || "";
      draftTargetLanguage = targetLanguage || "zh-CN";
    }
  });

  const closeModal = () => {
    open = false;
    onCancel();
  };

  const handleSave = async () => {
    await onSave({
      apiKey: draftApiKey.trim(),
      targetLanguage: draftTargetLanguage.trim() || "zh-CN",
    });
  };
</script>

<Modal
  bind:open
  title={$t("settings.translation.title")}
  onClose={closeModal}
  containerClass="max-w-lg"
>
  <div class="space-y-4 px-6 py-6">
    <div class="space-y-1.5">
      <label class="text-base-content text-sm font-medium" for="openrouter-api-key">
        {$t("settings.translation.apiKey")}
      </label>
      <input
        id="openrouter-api-key"
        class="border-base-300 bg-base-100 text-base-content focus:ring-primary w-full rounded-lg border px-3 py-2 text-sm focus:ring-2 focus:outline-none"
        type="password"
        bind:value={draftApiKey}
        placeholder="sk-or-v1-..."
        autocomplete="off"
      />
    </div>
    <div class="space-y-1.5">
      <label class="text-base-content text-sm font-medium" for="translate-target-language">
        {$t("settings.translation.targetLanguage")}
      </label>
      <input
        id="translate-target-language"
        class="border-base-300 bg-base-100 text-base-content focus:ring-primary w-full rounded-lg border px-3 py-2 text-sm focus:ring-2 focus:outline-none"
        type="text"
        bind:value={draftTargetLanguage}
        placeholder="zh-CN"
      />
      <p class="text-base-content-muted text-xs">
        {$t("settings.translation.targetLanguageHint")}
      </p>
    </div>
  </div>
  {#snippet footer()}
    <button
      class="bg-base-300 text-base-content hover:bg-base-200 rounded-lg px-4 py-2 text-sm transition"
      onclick={closeModal}
      type="button"
      disabled={saving}
    >
      {$t("addSkill.cancel")}
    </button>
    <PrimaryActionButton onclick={handleSave} disabled={saving}>
      {saving ? $t("settings.translation.saving") : $t("selectAgent.confirm")}
    </PrimaryActionButton>
  {/snippet}
</Modal>
