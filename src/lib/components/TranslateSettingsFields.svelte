<script lang="ts">
  import { onMount } from "svelte";
  import SelectField from "$lib/components/ui/SelectField.svelte";
  import { t } from "$lib/i18n";
  import {
    listOpenRouterModels,
    type OpenRouterModelOption,
    type TranslateDisplayMode,
    type TranslateTextStyle,
  } from "$lib/api/settings";

  /** Translation form fields, shared by the settings page and the translate modal. */
  let {
    apiKey = $bindable(""),
    targetLanguage = $bindable(""),
    model = $bindable(""),
    displayMode = $bindable<TranslateDisplayMode>("bilingual"),
    textStyle = $bindable<TranslateTextStyle>("none"),
    idPrefix = "translate",
  }: {
    apiKey?: string;
    targetLanguage?: string;
    model?: string;
    displayMode?: TranslateDisplayMode;
    textStyle?: TranslateTextStyle;
    /** Keeps the field ids unique when two forms can exist at the same time. */
    idPrefix?: string;
  } = $props();

  let modelOptions = $state<OpenRouterModelOption[]>([]);
  let loadingModels = $state(false);
  let modelsError = $state("");

  const languageOptions = [
    { value: "zh-CN", label: "简体中文 (zh-CN)" },
    { value: "en", label: "English (en)" },
    { value: "ja", label: "日本語 (ja)" },
    { value: "ko", label: "한국어 (ko)" },
    { value: "es", label: "Español (es)" },
    { value: "fr", label: "Français (fr)" },
    { value: "de", label: "Deutsch (de)" },
  ];

  const inputClass =
    "border-base-300 bg-base-100 text-base-content focus:ring-primary w-full rounded-lg border px-3 py-2 text-sm focus:ring-2 focus:outline-none";

  onMount(() => {
    loadingModels = true;
    listOpenRouterModels()
      .then((models) => {
        modelOptions = models;
      })
      .catch((error) => {
        modelsError = error instanceof Error ? error.message : String(error);
        modelOptions = [];
      })
      .finally(() => {
        loadingModels = false;
      });
  });
</script>

<div class="space-y-4">
  <div class="space-y-1.5">
    <label class="text-base-content text-sm font-medium" for={`${idPrefix}-target-language`}>
      {$t("settings.translation.targetLanguage")}
    </label>
    <SelectField id={`${idPrefix}-target-language`} bind:value={targetLanguage} className="w-full">
      <option value="">{$t("settings.selectPlaceholder")}</option>
      {#each languageOptions as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </SelectField>
  </div>
  <div class="space-y-1.5">
    <label class="text-base-content text-sm font-medium" for={`${idPrefix}-api-key`}>
      {$t("settings.translation.apiKey")}
    </label>
    <input
      id={`${idPrefix}-api-key`}
      class={inputClass}
      type="password"
      bind:value={apiKey}
      placeholder="sk-or-v1-..."
      autocomplete="off"
      required
    />
  </div>
  <div class="space-y-1.5">
    <label class="text-base-content text-sm font-medium" for={`${idPrefix}-model`}>
      {$t("settings.translation.model")}
    </label>
    <input
      id={`${idPrefix}-model`}
      class={inputClass}
      type="text"
      bind:value={model}
      list={`${idPrefix}-model-options`}
      placeholder="Model ID"
      autocomplete="off"
      required
    />
    <datalist id={`${idPrefix}-model-options`}>
      {#each modelOptions as option}
        <option value={option.id}>{option.name}</option>
      {/each}
    </datalist>
    <p class="text-base-content-muted text-xs">
      {#if loadingModels}
        {$t("settings.translation.loadingModels")}
      {:else if modelsError}
        {$t("settings.translation.loadModelsFailed")}
      {:else}
        {$t("settings.translation.modelHint")}
      {/if}
    </p>
  </div>
  <div class="space-y-1.5">
    <label class="text-base-content text-sm font-medium" for={`${idPrefix}-display-mode`}>
      {$t("settings.translation.displayMode")}
    </label>
    <SelectField id={`${idPrefix}-display-mode`} bind:value={displayMode} className="w-full">
      <option value="bilingual">{$t("settings.translation.displayModeBilingual")}</option>
      <option value="translation">{$t("settings.translation.displayModeTranslation")}</option>
    </SelectField>
  </div>
  <div class="space-y-1.5">
    <label class="text-base-content text-sm font-medium" for={`${idPrefix}-text-style`}>
      {$t("settings.translation.textStyle")}
    </label>
    <SelectField id={`${idPrefix}-text-style`} bind:value={textStyle} className="w-full">
      <option value="none">{$t("settings.translation.textStyleNone")}</option>
      <option value="dashed_underline">
        {$t("settings.translation.textStyleDashedUnderline")}
      </option>
    </SelectField>
  </div>
</div>
