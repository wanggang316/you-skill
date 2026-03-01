<script lang="ts">
  import Modal from "./ui/Modal.svelte";
  import PrimaryActionButton from "./ui/PrimaryActionButton.svelte";
  import SelectField from "./ui/SelectField.svelte";
  import { t } from "$lib/i18n";
  import { listOpenRouterModels, type OpenRouterModelOption } from "$lib/api/settings";

  let {
    open = $bindable(false),
    apiKey = "",
    targetLanguage = "zh-CN",
    model = "openai/gpt-4o-mini",
    saving = false,
    onSave = async () => {},
    onCancel = () => {},
  } = $props<{
    open?: boolean;
    apiKey?: string;
    targetLanguage?: string;
    model?: string;
    saving?: boolean;
    onSave?: (payload: { apiKey: string; targetLanguage: string; model: string }) => Promise<void> | void;
    onCancel?: () => void;
  }>();

  let draftApiKey = $state("");
  let draftTargetLanguage = $state("zh-CN");
  let draftModel = $state("openai/gpt-4o-mini");
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

  const fallbackModelOptions: OpenRouterModelOption[] = [
    { id: "openai/gpt-4o-mini", name: "GPT-4o Mini" },
    { id: "openai/gpt-4.1-mini", name: "GPT-4.1 Mini" },
    { id: "anthropic/claude-3.5-sonnet", name: "Claude 3.5 Sonnet" },
    { id: "google/gemini-2.0-flash-001", name: "Gemini 2.0 Flash" },
  ];

  $effect(() => {
    if (open) {
      draftApiKey = apiKey || "";
      draftTargetLanguage = targetLanguage || "zh-CN";
      draftModel = model || "openai/gpt-4o-mini";
    }
  });

  $effect(() => {
    if (!open) return;
    loadingModels = true;
    modelsError = "";
    listOpenRouterModels()
      .then((models) => {
        modelOptions = models.length > 0 ? models : fallbackModelOptions;
      })
      .catch((error) => {
        modelsError = error instanceof Error ? error.message : String(error);
        modelOptions = fallbackModelOptions;
      })
      .finally(() => {
        loadingModels = false;
      });
  });

  const closeModal = () => {
    open = false;
    onCancel();
  };

  const handleSave = async () => {
    await onSave({
      apiKey: draftApiKey.trim(),
      targetLanguage: draftTargetLanguage || "zh-CN",
      model: draftModel || "openai/gpt-4o-mini",
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
      <label class="text-base-content text-sm font-medium" for="translate-target-language">
        {$t("settings.translation.targetLanguage")}
      </label>
      <SelectField
        id="translate-target-language"
        bind:value={draftTargetLanguage}
        className="w-full"
        selectClassName="w-full"
      >
        {#each languageOptions as option}
          <option value={option.value}>{option.label}</option>
        {/each}
      </SelectField>
    </div>
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
      <label class="text-base-content text-sm font-medium" for="translate-model-input">
        {$t("settings.translation.model")}
      </label>
      <input
        id="translate-model-input"
        class="border-base-300 bg-base-100 text-base-content focus:ring-primary w-full rounded-lg border px-3 py-2 text-sm focus:ring-2 focus:outline-none"
        type="text"
        bind:value={draftModel}
        list="translate-model-options"
        placeholder="openai/gpt-4o-mini"
        autocomplete="off"
      />
      <datalist id="translate-model-options">
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
