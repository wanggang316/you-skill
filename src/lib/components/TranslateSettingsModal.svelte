<script lang="ts" module>
  import type { TranslateDisplayMode, TranslateTextStyle } from "$lib/api/settings";

  export interface TranslateSettingsPayload {
    apiKey: string;
    targetLanguage: string;
    model: string;
    displayMode: TranslateDisplayMode;
    textStyle: TranslateTextStyle;
  }
</script>

<script lang="ts">
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import TranslateSettingsFields from "$lib/components/TranslateSettingsFields.svelte";
  import { t } from "$lib/i18n";

  let {
    open = $bindable(false),
    apiKey = "",
    targetLanguage = "",
    model = "",
    displayMode = "bilingual",
    textStyle = "none",
    saving = false,
    onSave = async () => {},
    onCancel = () => {},
  } = $props<{
    open?: boolean;
    apiKey?: string;
    targetLanguage?: string;
    model?: string;
    displayMode?: TranslateDisplayMode;
    textStyle?: TranslateTextStyle;
    saving?: boolean;
    onSave?: (payload: TranslateSettingsPayload) => Promise<void> | void;
    onCancel?: () => void;
  }>();

  let draftApiKey = $state("");
  let draftTargetLanguage = $state("");
  let draftModel = $state("");
  let draftDisplayMode = $state<TranslateDisplayMode>("bilingual");
  let draftTextStyle = $state<TranslateTextStyle>("none");
  const isFormValid = $derived.by(
    () =>
      draftTargetLanguage.trim().length > 0 &&
      draftApiKey.trim().length > 0 &&
      draftModel.trim().length > 0
  );

  $effect(() => {
    if (open) {
      draftApiKey = apiKey || "";
      draftTargetLanguage = targetLanguage || "";
      draftModel = model || "";
      draftDisplayMode = displayMode;
      draftTextStyle = textStyle;
    }
  });

  const closeModal = () => {
    open = false;
    onCancel();
  };

  const handleSave = async () => {
    if (!isFormValid) return;
    await onSave({
      apiKey: draftApiKey.trim(),
      targetLanguage: draftTargetLanguage.trim(),
      model: draftModel.trim(),
      displayMode: draftDisplayMode,
      textStyle: draftTextStyle,
    });
  };
</script>

<Modal
  bind:open
  title={$t("settings.translation.title")}
  onClose={closeModal}
  containerClass="max-w-lg"
>
  <div class="px-6 py-6">
    <TranslateSettingsFields
      bind:apiKey={draftApiKey}
      bind:targetLanguage={draftTargetLanguage}
      bind:model={draftModel}
      bind:displayMode={draftDisplayMode}
      bind:textStyle={draftTextStyle}
      idPrefix="translate-modal"
    />
  </div>
  {#snippet footer()}
    <button
      class="bg-base-300 text-base-content hover:bg-base-200 rounded-lg px-4 py-2 text-sm transition"
      onclick={closeModal}
      type="button"
      disabled={saving}
    >
      {$t("common.cancel")}
    </button>
    <PrimaryActionButton onclick={handleSave} disabled={saving || !isFormValid}>
      {saving ? $t("settings.translation.saving") : $t("common.confirm")}
    </PrimaryActionButton>
  {/snippet}
</Modal>
