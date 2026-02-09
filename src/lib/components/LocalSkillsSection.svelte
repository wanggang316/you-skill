<script>
  import {
    RefreshCw,
    Search,
    Trash2,
    Blend,
    Download,
  } from "@lucide/svelte";
  import IconButton from "./ui/IconButton.svelte";
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
    agentMap,
    onRefresh,
    onDeleteSkill,
    onBulkUnify,
    onUnifySkill,
    onViewSkill,
    onOpenPendingImport,
    onOpenSelectAgentModal,
  } = $props();
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
                class="inline-flex items-center rounded-xl bg-[var(--warning)] px-4 py-2 text-sm font-medium text-[var(--warning-content)] transition hover:bg-[var(--warning-hover)]"
                onclick={onOpenPendingImport}
                type="button"
              >
                <Download size={16} class="mr-2" />
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
                  <p class="mt-2 text-xs text-[var(--base-content-subtle)]">
                    {$t("local.section.managedCount", {
                      count: skill.agents.length,
                    })}
                  </p>
                </div>
                <div
                  class="flex items-center gap-3 text-xs text-[var(--base-content-faint)] opacity-0 group-hover:opacity-100 transition-opacity"
                  onclick={(e) => e.stopPropagation()}
                  onkeydown={(e) => e.stopPropagation()}
                  role="presentation"
                >
                  <IconButton
                    variant="outline"
                    class="rounded-lg p-2 text-xs border-[var(--base-300)] text-[var(--base-content-muted)]"
                    onclick={(e) => {
                      e?.stopPropagation();
                      onOpenSelectAgentModal(skill);
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
            </div>
          {/each}
        {/if}
      </div>
    {/if}
  </div>
</section>
