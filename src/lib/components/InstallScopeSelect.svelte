<script lang="ts">
  import { t } from "../i18n";
  import SelectField from "./ui/SelectField.svelte";

  type ProjectOption = { name: string; path: string };

  let {
    value = $bindable("global"),
    projects = [] as ProjectOption[],
    disabled = false,
    className = "",
  } = $props<{
    value?: string;
    projects?: ProjectOption[];
    disabled?: boolean;
    className?: string;
  }>();

  let measureEl = $state<HTMLSpanElement | null>(null);
  let selectWidthPx = $state(112);
  const MAX_WIDTH_PX = 180;

  const selectedLabel = $derived.by(() => {
    if (!value.startsWith("project:")) return $t("installScope.global");
    const selectedPath = decodeURIComponent(value.slice("project:".length));
    return (
      projects.find((project: ProjectOption) => project.path === selectedPath)?.name ??
      $t("installScope.global")
    );
  });

  $effect(() => {
    const _label = selectedLabel;
    if (!measureEl) return;
    const textWidth = Math.ceil(measureEl.getBoundingClientRect().width);
    const iconAndPaddingWidth = 44;
    const width = Math.min(Math.max(textWidth + iconAndPaddingWidth, 96), MAX_WIDTH_PX);
    selectWidthPx = width;
  });
</script>

<div class={`relative shrink-0 ${className}`.trim()}>
  <SelectField
    bind:value
    {disabled}
    style={`width: ${selectWidthPx}px; min-width: ${selectWidthPx}px; max-width: ${MAX_WIDTH_PX}px;`}
  >
  <option value="global">{$t("installScope.global")}</option>
  {#if projects.length > 0}
    <option value="" disabled>------</option>
  {/if}
  {#each projects as project}
    <option value={`project:${encodeURIComponent(project.path)}`}>{project.name}</option>
  {/each}
  </SelectField>
  <span bind:this={measureEl} class="pointer-events-none absolute invisible text-sm whitespace-nowrap">
    {selectedLabel}
  </span>
</div>
