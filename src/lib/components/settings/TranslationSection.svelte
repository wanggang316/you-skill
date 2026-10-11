<script lang="ts">
  import { Check } from "@lucide/svelte";
  import TranslateSettingsFields from "$lib/components/TranslateSettingsFields.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "$lib/i18n";
  import type { TranslateDisplayMode, TranslateTextStyle } from "$lib/api/settings";
  import { settings, updateSettings } from "$lib/stores/settings";
  import SettingsPane from "./SettingsPane.svelte";

  let apiKey = $state("");
  let targetLanguage = $state("");
  let model = $state("");
  let displayMode = $state<TranslateDisplayMode>("bilingual");
  let textStyle = $state<TranslateTextStyle>("none");
  let saving = $state(false);
  let saved = $state(false);
  let error = $state("");

  // Load the stored values, and again when they arrive or are saved.
  $effect.pre(() => {
    apiKey = $settings.openrouter_api_key ?? "";
    targetLanguage = $settings.translate_target_language || "";
    model = $settings.translate_model || "";
    displayMode = $settings.translate_display_mode;
    textStyle = $settings.translate_text_style;
  });

  const isFormValid = $derived(
    targetLanguage.trim().length > 0 && apiKey.trim().length > 0 && model.trim().length > 0
  );

  async function handleSave() {
    if (!isFormValid || saving) return;
    saving = true;
    saved = false;
    error = "";
    try {
      await updateSettings({
        openrouter_api_key: apiKey.trim() || null,
        translate_target_language: targetLanguage.trim(),
        translate_model: model.trim(),
        translate_display_mode: displayMode,
        translate_text_style: textStyle,
      });
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } catch (err) {
      error = String(err);
    } finally {
      saving = false;
    }
  }
</script>

<SettingsPane
  title={$t("settings.translation.title")}
  description={$t("settings.translation.description")}
>
  <TranslateSettingsFields
    bind:apiKey
    bind:targetLanguage
    bind:model
    bind:displayMode
    bind:textStyle
    idPrefix="translate-settings"
    layout="rows"
  />

  <div class="flex items-center justify-end gap-3">
    {#if error}
      <span class="text-error min-w-0 flex-1 text-xs break-all">{error}</span>
    {:else if saved}
      <span class="text-base-content-muted flex items-center gap-1 text-xs">
        <Check size={13} />
        {$t("settings.translation.saved")}
      </span>
    {/if}
    <PrimaryActionButton
      className="h-8 rounded-lg px-4 py-0 text-[13px]"
      onclick={handleSave}
      disabled={!isFormValid}
      loading={saving}
    >
      {$t("settings.translation.save")}
    </PrimaryActionButton>
  </div>
</SettingsPane>
