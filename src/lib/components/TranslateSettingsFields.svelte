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
    layout = "stacked",
  }: {
    apiKey?: string;
    targetLanguage?: string;
    model?: string;
    displayMode?: TranslateDisplayMode;
    textStyle?: TranslateTextStyle;
    /** Keeps the field ids unique when two forms can exist at the same time. */
    idPrefix?: string;
    /** `stacked`: label above the control (modal). `rows`: label left, control right (settings card). */
    layout?: "stacked" | "rows";
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

  const rows = $derived(layout === "rows");
  const containerClass = $derived(rows ? "settings-card" : "space-y-4");
  const fieldClass = $derived(
    rows ? "flex min-h-14 items-center justify-between gap-6 px-4 py-3" : "space-y-1.5"
  );
  const labelClass = $derived(
    rows ? "text-base-content text-[13.5px]" : "text-base-content text-sm font-medium"
  );
  const controlClass = $derived(rows ? "w-64 shrink-0" : "w-full");
  // Selects take the width of the selected option instead of a shared column width.
  const selectFieldClass = $derived(rows ? "shrink-0" : "w-full");
  const selectClass = $derived(rows ? "h-8 text-[13px]" : "");
  const inputClass = $derived(
    `border-base-300 bg-base-100 text-base-content focus:ring-primary w-full rounded-lg border px-3 focus:ring-2 focus:outline-none ${
      rows ? "h-8 text-[13px]" : "py-2 text-sm"
    }`
  );

  const modelHint = $derived(
    loadingModels
      ? $t("settings.translation.loadingModels")
      : modelsError
        ? $t("settings.translation.loadModelsFailed")
        : $t("settings.translation.modelHint")
  );

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

<div class={containerClass}>
  <div class={fieldClass}>
    <label class={labelClass} for={`${idPrefix}-target-language`}>
      {$t("settings.translation.targetLanguage")}
    </label>
    <SelectField
      id={`${idPrefix}-target-language`}
      bind:value={targetLanguage}
      fit={rows}
      className={selectFieldClass}
      selectClassName={selectClass}
    >
      <option value="">{$t("settings.selectPlaceholder")}</option>
      {#each languageOptions as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </SelectField>
  </div>
  <!-- The key is long, so it gets the full row width below its label in both layouts. -->
  <div class={rows ? "space-y-2 px-4 py-3" : "space-y-1.5"}>
    <label class={`${labelClass} block`} for={`${idPrefix}-api-key`}>
      {$t("settings.translation.apiKey")}
    </label>
    <div class="w-full">
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
  </div>
  <div class={fieldClass}>
    {#if rows}
      <div class="min-w-0">
        <label class={labelClass} for={`${idPrefix}-model`}>
          {$t("settings.translation.model")}
        </label>
        <p class="text-base-content-muted mt-0.5 text-xs">{modelHint}</p>
      </div>
    {:else}
      <label class={labelClass} for={`${idPrefix}-model`}>
        {$t("settings.translation.model")}
      </label>
    {/if}
    <div class={controlClass}>
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
    </div>
    {#if !rows}
      <p class="text-base-content-muted text-xs">{modelHint}</p>
    {/if}
  </div>
  <div class={fieldClass}>
    <label class={labelClass} for={`${idPrefix}-display-mode`}>
      {$t("settings.translation.displayMode")}
    </label>
    <SelectField
      id={`${idPrefix}-display-mode`}
      bind:value={displayMode}
      fit={rows}
      className={selectFieldClass}
      selectClassName={selectClass}
    >
      <option value="bilingual">{$t("settings.translation.displayModeBilingual")}</option>
      <option value="translation">{$t("settings.translation.displayModeTranslation")}</option>
    </SelectField>
  </div>
  <div class={fieldClass}>
    <label class={labelClass} for={`${idPrefix}-text-style`}>
      {$t("settings.translation.textStyle")}
    </label>
    <SelectField
      id={`${idPrefix}-text-style`}
      bind:value={textStyle}
      fit={rows}
      className={selectFieldClass}
      selectClassName={selectClass}
    >
      <option value="none">{$t("settings.translation.textStyleNone")}</option>
      <option value="dashed_underline">
        {$t("settings.translation.textStyleDashedUnderline")}
      </option>
    </SelectField>
  </div>
</div>
