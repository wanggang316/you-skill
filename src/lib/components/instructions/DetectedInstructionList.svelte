<script lang="ts">
  import { t } from "$lib/i18n";
  import type { DetectedInstruction } from "$lib/api/instructions";

  export interface DetectedChoice {
    selected: boolean;
    /** Library name to import under. */
    name: string;
  }

  let {
    items = [],
    choices = $bindable<Record<string, DetectedChoice>>({}),
    disabled = false,
  }: {
    items?: DetectedInstruction[];
    choices?: Record<string, DetectedChoice>;
    disabled?: boolean;
  } = $props();

  const allSelected = $derived(
    items.length > 0 && items.every((item) => choices[item.path]?.selected)
  );

  function update(item: DetectedInstruction, patch: Partial<DetectedChoice>) {
    const current = choices[item.path];
    if (!current) return;
    choices = { ...choices, [item.path]: { ...current, ...patch } };
  }

  function toggleAll() {
    if (disabled) return;
    const next = { ...choices };
    for (const item of items) {
      const current = next[item.path];
      if (current) next[item.path] = { ...current, selected: !allSelected };
    }
    choices = next;
  }
</script>

<div class="space-y-2">
  <div class="flex items-center justify-between">
    <p class="text-base-content text-sm">{$t("instructions.import.select")}</p>
    {#if items.length > 1}
      <label class="text-base-content-muted inline-flex items-center gap-2 text-[13px]">
        <input type="checkbox" checked={allSelected} onchange={toggleAll} {disabled} />
        {$t("import.selectAll")}
      </label>
    {/if}
  </div>
  <div class="border-base-300 bg-base-200 max-h-72 space-y-2 overflow-y-auto rounded-xl border p-2">
    {#each items as item (item.path)}
      {@const choice = choices[item.path]}
      <div
        class={`rounded-lg px-3 py-2 text-sm transition ${
          choice?.selected ? "bg-base-100 ring-primary/40 ring-1" : "bg-base-100/60"
        }`}
      >
        <div class="flex items-start gap-2">
          <input
            type="checkbox"
            class="mt-1"
            checked={Boolean(choice?.selected)}
            {disabled}
            onchange={() => update(item, { selected: !choice?.selected })}
          />
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <input
                class="border-base-300 bg-base-100 text-base-content focus:border-primary h-7 w-44 rounded-lg border px-2 text-[13px] focus:outline-none"
                value={choice?.name ?? item.name}
                {disabled}
                aria-label={$t("instructions.import.name")}
                oninput={(event) => update(item, { name: event.currentTarget.value })}
              />
              <span class="text-base-content-subtle font-mono text-[11px]">{item.fileName}</span>
            </div>
            <p class="text-base-content-subtle truncate text-[11px]" title={item.path}>
              {item.relPath}
            </p>
          </div>
        </div>
      </div>
    {/each}
  </div>
</div>
