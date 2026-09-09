<script lang="ts">
  import { t } from "$lib/i18n";
  import type { ScanResolution } from "$lib/api/hub";
  import type { InstructionScanItem } from "$lib/api/instructions";
  import { baseName } from "$lib/scopes";
  import { agentsById } from "$lib/stores/hub";

  export interface InstructionScanChoice {
    selected: boolean;
    resolution: ScanResolution;
    registerInstall: boolean;
    /** Library name for a new file; ignored for files that belong to an entry. */
    name: string;
  }

  let {
    items = [],
    choices = $bindable<Record<string, InstructionScanChoice>>({}),
    disabled = false,
  }: {
    items?: InstructionScanItem[];
    choices?: Record<string, InstructionScanChoice>;
    disabled?: boolean;
  } = $props();

  /** New files, drifted files, and files identical to an entry but not registered yet. */
  export function isSelectable(item: InstructionScanItem): boolean {
    if (item.status === "new" || item.status === "different") return true;
    return item.status === "identical" && !item.registered;
  }

  const selectableItems = $derived(items.filter((item) => isSelectable(item)));
  const allSelected = $derived(
    selectableItems.length > 0 && selectableItems.every((item) => choices[item.path]?.selected)
  );

  function statusClass(item: InstructionScanItem): string {
    switch (item.status) {
      case "new":
        return "tag-success";
      case "different":
        return "tag-warning";
      default:
        return "tag-neutral";
    }
  }

  function agentNames(ids: string[]): string {
    return ids.map((id) => $agentsById.get(id)?.display_name ?? id).join(", ");
  }

  function locationLabel(item: InstructionScanItem): string {
    const agents = agentNames(item.location.agentIds);
    if (item.location.scope === "user") {
      return $t("instructions.import.scan.location.user", { agents });
    }
    return $t("instructions.import.scan.location.project", {
      project: baseName(item.location.projectPath ?? ""),
      agents,
    });
  }

  function hint(item: InstructionScanItem): string {
    const name = item.hubName ?? "";
    if (!name) return "";
    if (item.registered) return $t("instructions.import.scan.registered", { name });
    if (item.status === "identical") return $t("instructions.import.scan.sameAs", { name });
    return $t("instructions.import.scan.driftedFrom", { name });
  }

  function update(item: InstructionScanItem, patch: Partial<InstructionScanChoice>) {
    const current = choices[item.path];
    if (!current) return;
    choices = { ...choices, [item.path]: { ...current, ...patch } };
  }

  function toggle(item: InstructionScanItem) {
    if (disabled || !isSelectable(item)) return;
    update(item, { selected: !choices[item.path]?.selected });
  }

  function toggleAll() {
    if (disabled) return;
    const next = { ...choices };
    for (const item of selectableItems) {
      const current = next[item.path];
      if (current) next[item.path] = { ...current, selected: !allSelected };
    }
    choices = next;
  }
</script>

<div class="space-y-2">
  <div class="flex items-center justify-between">
    <p class="text-base-content text-sm">{$t("instructions.import.scan.select")}</p>
    {#if selectableItems.length > 1}
      <label class="text-base-content-muted inline-flex items-center gap-2 text-[13px]">
        <input type="checkbox" checked={allSelected} onchange={toggleAll} {disabled} />
        {$t("import.selectAll")}
      </label>
    {/if}
  </div>
  <div class="border-base-300 bg-base-200 max-h-72 space-y-2 overflow-y-auto rounded-xl border p-2">
    {#each items as item (item.path)}
      {@const choice = choices[item.path]}
      {@const selectable = isSelectable(item)}
      <div
        class={`rounded-lg px-3 py-2 text-sm transition ${
          choice?.selected ? "bg-base-100 ring-primary/40 ring-1" : "bg-base-100/60"
        } ${selectable ? "" : "opacity-60"}`}
      >
        <div class="flex items-start gap-2">
          <input
            type="checkbox"
            class="mt-1"
            checked={Boolean(choice?.selected)}
            disabled={disabled || !selectable}
            onchange={() => toggle(item)}
          />
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              {#if item.status === "new" && choice}
                <input
                  class="border-base-300 bg-base-100 text-base-content focus:border-primary h-7 w-44 rounded-lg border px-2 text-[13px] focus:outline-none"
                  value={choice.name}
                  {disabled}
                  aria-label={$t("instructions.import.name")}
                  oninput={(event) => update(item, { name: event.currentTarget.value })}
                />
              {:else}
                <span class="text-base-content font-medium">{item.hubName ?? item.name}</span>
              {/if}
              <span class="text-base-content-subtle font-mono text-[11px]">{item.fileName}</span>
              <span class={`tag ${statusClass(item)}`}>{$t(`scan.status.${item.status}`)}</span>
            </div>
            <p class="text-base-content-subtle truncate text-[11px]" title={item.path}>
              {item.path}
            </p>
            <p class="text-base-content-muted mt-1 text-[11px]">
              {locationLabel(item)}
              {#if hint(item)}
                <span> · {hint(item)}</span>
              {/if}
            </p>
            {#if item.error}
              <p class="text-error mt-1 text-[11px]">{item.error}</p>
            {/if}
          </div>
          {#if selectable && choice && item.status === "different"}
            <select
              class="border-base-300 bg-base-100 text-base-content h-7 shrink-0 rounded-lg border px-2 text-xs focus:outline-none"
              value={choice.resolution}
              {disabled}
              onchange={(event) => {
                const resolution = event.currentTarget.value as ScanResolution;
                update(item, { resolution, selected: resolution !== "skip" });
              }}
            >
              {#each ["adopt_into_hub", "push_from_hub", "skip"] as option}
                <option value={option}>{$t(`scan.resolution.${option}`)}</option>
              {/each}
            </select>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>
