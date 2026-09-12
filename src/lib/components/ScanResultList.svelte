<script lang="ts">
  import { ChevronDown, ChevronRight } from "@lucide/svelte";
  import { t } from "../i18n";
  import type { ScanItem } from "../api/hub";
  import {
    actionsFor,
    copyRelation,
    groupScanItems,
    hubBound,
    preferredHubCopy,
    withAction,
    type ScanAction,
    type ScanGroup,
  } from "../scan";

  let {
    items = [],
    actions = $bindable<Record<string, ScanAction>>({}),
    rootPath = "",
    disabled = false,
  }: {
    items?: ScanItem[];
    actions?: Record<string, ScanAction>;
    rootPath?: string;
    disabled?: boolean;
  } = $props();

  let inertOpen = $state(false);

  const sections = $derived(groupScanItems(items));
  const allFreshChosen = $derived(
    sections.fresh.length > 0 && sections.fresh.every((group) => hubBound(group, actions))
  );

  function relativePath(path: string): string {
    if (rootPath && path.startsWith(rootPath)) {
      const rest = path.slice(rootPath.length).replace(/^[/\\]/, "");
      return rest || ".";
    }
    return path;
  }

  function setAction(group: ScanGroup, item: ScanItem, action: ScanAction) {
    actions = withAction(actions, group, item, action);
  }

  /** Take every new skill in, or leave every one of them out. */
  function toggleFresh() {
    if (disabled) return;
    let next = actions;
    for (const group of sections.fresh) {
      const bound = hubBound(group, next);
      if (allFreshChosen) {
        if (bound) next = withAction(next, group, bound, "ignore");
      } else if (!bound) {
        next = withAction(next, group, preferredHubCopy(group), "to_hub");
      }
    }
    actions = next;
  }
</script>

{#snippet row(group: ScanGroup, item: ScanItem)}
  {@const action = actions[item.path] ?? "ignore"}
  {@const relation = copyRelation(group, item, actions)}
  <div
    class={`flex items-start gap-2 rounded-lg px-2.5 py-1.5 transition ${
      action === "ignore" ? "opacity-60" : ""
    }`}
  >
    <span class="min-w-0 flex-1">
      <span class="text-base-content-subtle block truncate text-[11px]" title={item.path}>
        {relativePath(item.path)}
      </span>
      <span class="flex flex-wrap items-center gap-1.5">
        {#if group.inHub}
          <span class={`tag ${item.status === "different" ? "tag-warning" : "tag-neutral"}`}>
            {$t(`scan.status.${item.status}`)}
          </span>
        {:else if relation}
          <span class={`tag ${relation === "same" ? "tag-neutral" : "tag-warning"}`}>
            {$t(`scan.relation.${relation}`)}
          </span>
        {/if}
        {#if item.error}
          <span class="text-error text-[11px]">{item.error}</span>
        {/if}
      </span>
    </span>
    <select
      class="border-base-300 bg-base-100 text-base-content h-7 shrink-0 rounded-lg border px-2 text-xs focus:outline-none"
      value={action}
      {disabled}
      onchange={(event) => setAction(group, item, event.currentTarget.value as ScanAction)}
    >
      {#each actionsFor(group, item, actions) as option (option)}
        <option value={option}>
          {$t(group.inHub && option === "to_hub" ? "scan.action.adopt" : `scan.action.${option}`)}
        </option>
      {/each}
    </select>
  </div>
{/snippet}

{#snippet groupBlock(group: ScanGroup)}
  <div class="bg-base-100 rounded-lg px-1 py-1.5">
    <div class="flex items-center gap-2 px-2">
      <span class="text-base-content truncate text-[13px] font-medium">{group.name}</span>
      {#if group.items.length > 1}
        <span class="text-base-content-faint text-[11px]">
          {$t("scan.group.copies", { count: group.items.length })}
        </span>
      {/if}
    </div>
    {#each group.items as item (item.path)}
      {@render row(group, item)}
    {/each}
  </div>
{/snippet}

<div class="space-y-2">
  <div class="border-base-300 bg-base-200 max-h-72 space-y-3 overflow-y-auto rounded-xl border p-2">
    {#if sections.fresh.length > 0}
      <section class="space-y-1.5">
        <div class="flex items-center justify-between gap-2 px-1">
          <p class="text-base-content text-[12px] font-medium">
            {$t("scan.section.fresh", { count: sections.fresh.length })}
          </p>
          <label class="text-base-content-muted inline-flex items-center gap-1.5 text-[11px]">
            <input type="checkbox" checked={allFreshChosen} onchange={toggleFresh} {disabled} />
            {$t("import.selectAll")}
          </label>
        </div>
        {#each sections.fresh as group (group.name)}
          {@render groupBlock(group)}
        {/each}
      </section>
    {/if}

    {#if sections.known.length > 0}
      <section class="space-y-1.5">
        <p class="text-base-content px-1 text-[12px] font-medium">
          {$t("scan.section.known", { count: sections.known.length })}
        </p>
        {#each sections.known as group (group.name)}
          {@render groupBlock(group)}
        {/each}
      </section>
    {/if}

    {#if sections.inert.length > 0}
      <section class="space-y-1">
        <button
          class="text-base-content-muted hover:text-base-content flex w-full items-center gap-1 px-1 text-[12px] transition"
          type="button"
          onclick={() => (inertOpen = !inertOpen)}
        >
          {#if inertOpen}
            <ChevronDown size={13} />
          {:else}
            <ChevronRight size={13} />
          {/if}
          {$t("scan.section.inert", { count: sections.inert.length })}
        </button>
        {#if inertOpen}
          {#each sections.inert as item (item.path)}
            <div class="flex items-center gap-2 px-2.5 py-1 opacity-70">
              <span class="text-base-content min-w-0 flex-1 truncate text-[12px]" title={item.path}>
                {item.name}
                <span class="text-base-content-faint">· {relativePath(item.path)}</span>
              </span>
              <span class="tag tag-neutral shrink-0">{$t(`scan.status.${item.status}`)}</span>
            </div>
          {/each}
        {/if}
      </section>
    {/if}
  </div>
  <p class="text-base-content-faint text-[11px]">{$t("scan.rule")}</p>
</div>
