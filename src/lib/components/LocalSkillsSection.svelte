<script>
  import {
    Blend,
    RefreshCw,
    Search,
    Trash2,
    ChevronsUpDown,
    Download,
  } from "@lucide/svelte";
  import IconButton from "./IconButton.svelte";
  import { t } from "../i18n";

  let {
    localSearch = $bindable(),
    localAgent = $bindable(),
    agents,
    localLoading,
    localError,
    filteredLocalSkills,
    managedSkills,
    unmanagedSkills,
    editingSkillKey,
    editSelection,
    allSelected,
    agentMap,
    linkBusy,
    onRefresh,
    onOpenLinkDialog,
    onDeleteSkill,
    onToggleSelectAll,
    onToggleAgentSelection,
    onConfirmAgentLinks,
    onBulkUnify,
    onUnifySkill,
    onViewSkill,
    onOpenPendingImport,
  } = $props();

  let expandedSkills = $state(new Set());

  function toggleSkillExpand(skill) {
    const key = skill.name;
    const newSet = new Set(expandedSkills);
    if (newSet.has(key)) {
      newSet.delete(key);
    } else {
      newSet.add(key);
    }
    expandedSkills = newSet;
  }
</script>

<section class="space-y-6">
  <div
    class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-4"
  >
    <div class="flex flex-wrap items-center gap-3">
      <div class="relative flex-1">
        <Search
          class="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--base-content-subtle)]"
          size={16}
        />
        <input
          class="h-9 w-full rounded-xl border border-[var(--base-300)] bg-[var(--base-200)] px-9 text-sm text-[var(--base-content)] placeholder:text-[var(--base-content-subtle)] focus:border-[var(--base-300)] focus:outline-none"
          placeholder={$t("local.search.placeholder")}
          bind:value={localSearch}
        />
      </div>
      <select
        class="h-9 rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 text-sm text-[var(--base-content)] focus:border-[var(--base-300)] focus:outline-none cursor-pointer"
        bind:value={localAgent}
      >
        <option value="all">{$t("local.agent.all")}</option>
        {#each agents as agent}
          <option value={agent.id}>{agent.display_name}</option>
        {/each}
      </select>
      <IconButton
        variant="outline"
        onclick={onRefresh}
        title={$t("local.refresh")}
        ariaLabel={$t("local.refresh")}
        class="h-9 w-9"
      >
        <RefreshCw size={16} />
      </IconButton>
    </div>
    {#if localError}
      <p class="mt-3 text-sm text-[var(--error)]">{localError}</p>
    {/if}
  </div>

  <div class="space-y-3">
    {#if localLoading}
      <div
        class="rounded-2xl border border-dashed border-[var(--base-300)] bg-[var(--base-100)] p-6 text-center text-sm text-[var(--base-content-muted)]"
      >
        {$t("local.loading")}
      </div>
    {:else if filteredLocalSkills.length === 0}
      <div
        class="rounded-2xl border border-dashed border-[var(--base-300)] bg-[var(--base-100)] p-6 text-center text-sm text-[var(--base-content-muted)]"
      >
        {$t("local.empty")}
      </div>
    {:else}
      <div class="space-y-2">
        <p class="text-sm font-semibold text-[var(--base-content-muted)]">
          {$t("local.section.managed")}
        </p>
        {#if managedSkills.length === 0}
          <div
            class="rounded-xl border border-dashed border-[var(--base-300)] bg-[var(--base-100)] p-6 text-center"
          >
            <p class="text-sm text-[var(--base-content-muted)] mb-4">
              {$t("local.section.emptyManaged")}
            </p>
            {#if unmanagedSkills.length > 0}
              <p class="text-sm text-[var(--base-content)] mb-4">
                {$t("local.section.pendingImportPrompt", { count: unmanagedSkills.length })}
              </p>
              <button
                class="inline-flex items-center gap-2 rounded-xl bg-[var(--warning)] px-4 py-2 text-sm font-medium text-[var(--warning-content)] transition hover:opacity-90"
                onclick={onOpenPendingImport}
                type="button"
              >
                <Download size={16} />
                {$t('header.pendingImport')}
              </button>
            {/if}
          </div>
        {:else}
          {#each managedSkills as skill}
            <div
              class="group rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-4 transition hover:bg-[var(--base-200)] hover:shadow-sm cursor-pointer"
              onclick={() => onViewSkill(skill)}
              onkeydown={(e) => (e.key === "Enter" || e.key === " ") && onViewSkill(skill)}
              role="button"
              tabindex="0"
              aria-label={`View ${skill.name}`}
            >
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div class="flex-1">
                  <button
                    class="text-base font-semibold hover:underline cursor-pointer bg-transparent border-none p-0"
                    onclick={(e) => {
                      e.stopPropagation();
                      onViewSkill(skill);
                    }}
                    type="button"
                  >
                    {skill.name}
                  </button>
                  {#if editingSkillKey !== skill.key}
                    <button
                      class="mt-2 flex items-center gap-1.5 text-xs text-[var(--base-content-subtle)] transition hover:bg-[var(--base-200)] rounded px-2 py-1 cursor-pointer"
                      onclick={(e) => {
                        e.stopPropagation();
                        toggleSkillExpand(skill);
                      }}
                      type="button"
                    >
                      <span
                        >{$t("local.section.managedCount", {
                          count: skill.agents.length,
                        })}</span
                      >
                      <ChevronsUpDown
                        size={12}
                        class={expandedSkills.has(skill.name)
                          ? "rotate-180"
                          : ""}
                      />
                    </button>
                    {#if expandedSkills.has(skill.name)}
                      <div class="mt-2 flex flex-wrap gap-2">
                        {#each skill.agents as agentId}
                          <div
                            class="inline-flex items-center rounded-full border border-[var(--base-300)] bg-[var(--base-100)] px-2.5 py-1 text-xs text-[var(--base-content-subtle)]"
                          >
                            {agentMap.get(agentId) || agentId}
                          </div>
                        {/each}
                      </div>
                    {/if}
                  {/if}
                </div>
                <div
                  class="flex items-center gap-3 text-xs text-[var(--base-content-faint)] opacity-0 group-hover:opacity-100 transition-opacity"
                  onclick={(e) => e.stopPropagation()}
                  onkeydown={(e) => e.stopPropagation()}
                  role="presentation"
                >
                  <IconButton
                    variant="outline"
                    class={`rounded-lg p-2 text-xs ${editingSkillKey === skill.key ? "border-[var(--base-content)] text-[var(--base-content)]" : "border-[var(--base-300)] text-[var(--base-content-muted)]"}`}
                    onclick={(e) => {
                      e?.stopPropagation();
                      onOpenLinkDialog(skill);
                    }}
                    title={$t("local.action.installToApps")}
                    ariaLabel={$t("local.action.installToApps")}
                  >
                    <Blend size={14} />
                  </IconButton>
                  <IconButton
                    variant="outline"
                    class="rounded-lg border-[var(--error-border)] p-2 text-xs text-[var(--error)]"
                    onclick={(event) => {
                      event.stopPropagation();
                      onDeleteSkill(skill);
                    }}
                    title={$t("local.action.delete")}
                    ariaLabel={$t("local.action.delete")}
                  >
                    <Trash2 size={14} />
                  </IconButton>
                </div>
              </div>
              {#if editingSkillKey === skill.key}
                <div
                  class="mt-4 rounded-xl border border-[var(--base-300)] bg-[var(--base-200)] p-3"
                >
                  <div
                    class="flex items-center justify-between text-xs text-[var(--base-content-muted)]"
                  >
                    <label class="inline-flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={allSelected}
                        onchange={(event) =>
                          onToggleSelectAll(event.target.checked)}
                        disabled={linkBusy}
                      />
                      {$t("local.action.selectAll")}
                    </label>
                    <button
                      class="rounded-lg bg-[var(--primary)] px-3 py-1.5 text-[var(--primary-content)] transition hover:opacity-90"
                      onclick={onConfirmAgentLinks}
                      disabled={linkBusy}
                      type="button"
                    >
                      {$t("local.action.confirm")}
                    </button>
                  </div>
                  <div class="mt-3 flex flex-wrap gap-2">
                    {#each agents as agent}
                      <label
                        class="inline-flex items-center gap-3 rounded-lg bg-[var(--base-100)] px-3 py-2 text-sm text-[var(--base-content)]"
                      >
                        <input
                          type="checkbox"
                          checked={editSelection.includes(agent.id)}
                          onchange={(event) =>
                            onToggleAgentSelection(
                              agent.id,
                              event.target.checked,
                            )}
                          disabled={linkBusy}
                        />
                        <span>{agent.display_name}</span>
                      </label>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        {/if}
      </div>
    {/if}
  </div>
</section>
