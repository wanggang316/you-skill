<script lang="ts">
  import { t } from "../i18n";
  import type { ScanItem, ScanResolution } from "../api/hub";
  import { agentsById } from "../stores/hub";

  export interface ScanChoice {
    selected: boolean;
    resolution: ScanResolution;
    registerInstall: boolean;
  }

  let {
    items = [],
    choices = $bindable<Record<string, ScanChoice>>({}),
    rootPath = "",
    disabled = false,
  }: {
    items?: ScanItem[];
    choices?: Record<string, ScanChoice>;
    rootPath?: string;
    disabled?: boolean;
  } = $props();

  const selectableItems = $derived(items.filter((item) => isSelectable(item)));
  const allSelected = $derived(
    selectableItems.length > 0 && selectableItems.every((item) => choices[item.path]?.selected)
  );

  function isSelectable(item: ScanItem): boolean {
    return item.status !== "hub" && item.status !== "invalid_name" && item.status !== "linked";
  }

  function relativePath(path: string): string {
    if (rootPath && path.startsWith(rootPath)) {
      const rest = path.slice(rootPath.length).replace(/^[/\\]/, "");
      return rest || ".";
    }
    return path;
  }

  function statusClass(item: ScanItem): string {
    switch (item.status) {
      case "new":
        return "tag-success";
      case "identical":
      case "linked":
        return "tag-neutral";
      case "different":
        return "tag-warning";
      default:
        return "tag-error";
    }
  }

  function resolutionOptions(item: ScanItem): ScanResolution[] {
    if (item.status === "new") return ["import", "skip"];
    if (item.status === "different") return ["adopt_into_hub", "push_from_hub", "skip"];
    return ["import", "skip"];
  }

  function agentNames(ids: string[]): string {
    return ids.map((id) => $agentsById.get(id)?.display_name ?? id).join(", ");
  }

  function toggle(item: ScanItem) {
    if (disabled || !isSelectable(item)) return;
    const current = choices[item.path];
    if (!current) return;
    choices = { ...choices, [item.path]: { ...current, selected: !current.selected } };
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

  function setResolution(item: ScanItem, resolution: ScanResolution) {
    const current = choices[item.path];
    if (!current) return;
    choices = {
      ...choices,
      [item.path]: { ...current, resolution, selected: resolution !== "skip" },
    };
  }

  function setRegister(item: ScanItem, registerInstall: boolean) {
    const current = choices[item.path];
    if (!current) return;
    choices = { ...choices, [item.path]: { ...current, registerInstall } };
  }
</script>

<div class="space-y-2">
  <div class="flex items-center justify-between">
    <p class="text-base-content text-sm">{$t("import.selectSkills")}</p>
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
              <span class="text-base-content font-medium">{item.name}</span>
              <span class={`tag ${statusClass(item)}`}>{$t(`scan.status.${item.status}`)}</span>
            </div>
            <p class="text-base-content-subtle truncate text-[11px]" title={item.path}>
              {relativePath(item.path)}
            </p>
            {#if item.inAgentRoot}
              <p class="text-base-content-muted mt-1 text-[11px]">
                {item.inAgentRoot.scope === "user"
                  ? $t("scan.inAgentRoot.user", { agents: agentNames(item.inAgentRoot.agentIds) })
                  : $t("scan.inAgentRoot.project", {
                      agents: agentNames(item.inAgentRoot.agentIds),
                    })}
                {#if item.inAgentRoot.scope === "project" && !item.inAgentRoot.registeredProject}
                  <span class="text-warning-content"> · {$t("scan.unregisteredProject")}</span>
                {/if}
              </p>
            {/if}
            {#if item.error}
              <p class="text-error mt-1 text-[11px]">{item.error}</p>
            {/if}
          </div>
          {#if selectable && choice}
            <div class="flex shrink-0 flex-col items-end gap-1">
              <select
                class="border-base-300 bg-base-100 text-base-content h-7 rounded-lg border px-2 text-xs focus:outline-none"
                value={choice.resolution}
                {disabled}
                onchange={(event) =>
                  setResolution(item, event.currentTarget.value as ScanResolution)}
              >
                {#each resolutionOptions(item) as option}
                  <option value={option}>{$t(`scan.resolution.${option}`)}</option>
                {/each}
              </select>
              {#if item.inAgentRoot}
                <label class="text-base-content-muted inline-flex items-center gap-1 text-[11px]">
                  <input
                    type="checkbox"
                    checked={choice.registerInstall}
                    {disabled}
                    onchange={(event) => setRegister(item, event.currentTarget.checked)}
                  />
                  {$t("scan.registerInstall")}
                </label>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>
