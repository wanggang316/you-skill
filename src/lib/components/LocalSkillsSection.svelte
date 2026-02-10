<script>
  import { RefreshCw, Search, Trash2, Blend, Download, ChevronsUpDown } from "@lucide/svelte";
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

  // Track which skill has expanded agents view
  let expandedSkillId = $state(null);
</script>

<section class="space-y-6">
  <div class="border-base-300 bg-base-100 rounded-2xl py-1">
    <div class="flex flex-wrap items-center gap-3">
      <div class="relative flex-1">
        <Search
          class="text-base-content-subtle absolute top-1/2 left-3 -translate-y-1/2"
          size={16}
        />
        <input
          class="border-base-300 bg-base-200 text-base-content placeholder:text-base-content-subtle focus:border-base-300 h-9 w-full rounded-xl border px-9 text-sm focus:outline-none"
          placeholder={$t("local.search.placeholder")}
          bind:value={localSearch}
        />
      </div>
      <div class="relative">
        <select
          class="border-base-300 bg-base-100 text-base-content focus:border-base-300 h-9 cursor-pointer appearance-none rounded-xl border pr-8 pl-3 text-sm focus:outline-none"
          bind:value={localAgent}
        >
          <option value="all">{$t("local.agent.all")}</option>
          {#each agents as agent}
            <option value={agent.id}>{agent.display_name}</option>
          {/each}
        </select>
        <ChevronsUpDown
          class="text-base-content-subtle pointer-events-none absolute top-1/2 right-3 -translate-y-1/2"
          size={14}
        />
      </div>
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
      <p class="text-error mt-3 text-sm">{localError}</p>
    {/if}
  </div>

  <div class="space-y-3">
    {#if localLoading}
      <div
        class="border-base-300 bg-base-100 text-base-content-muted rounded-2xl border border-dashed p-6 text-center text-sm"
      >
        {$t("local.loading")}
      </div>
    {:else if filteredLocalSkills.length === 0}
      <div
        class="border-base-300 bg-base-100 text-base-content-muted rounded-2xl border border-dashed p-6 text-center text-sm"
      >
        {$t("local.empty")}
      </div>
    {:else}
      <div class="space-y-2">
        <!-- <p class="text-base-content-muted text-sm font-semibold">
          {$t("local.section.managed")}
        </p> -->
        {#if managedSkills.length === 0}
          <div class="border-base-300 bg-base-100 rounded-xl border border-dashed p-6 text-center">
            <p class="text-base-content-muted mb-4 text-sm">
              {$t("local.section.emptyManaged")}
            </p>
            {#if unmanagedSkills.length > 0}
              <p class="text-base-content mb-4 text-sm">
                {$t("local.section.pendingImportPrompt", { count: unmanagedSkills.length })}
              </p>
              <button
                class="bg-warning text-warning-content hover:bg-warning-hover inline-flex items-center rounded-xl px-4 py-2 text-sm font-medium transition"
                onclick={onOpenPendingImport}
                type="button"
              >
                <Download size={16} class="mr-2" />
                {$t("header.pendingImport")}
              </button>
            {/if}
          </div>
        {:else}
          {#each managedSkills as skill}
            <div
              class="border-base-300 bg-base-100  rounded-2xl border p-4 transition"
            >
              <!-- First row: title and action buttons -->
              <div class="flex flex-wrap items-center justify-between gap-3">
                <button
                  class="cursor-pointer border-none bg-transparent p-0 text-base font-medium"
                  onclick={() => onViewSkill(skill)}
                  type="button"
                >
                  {skill.name}
                </button>
                <div
                  class="text-base-content-faint flex items-center gap-3 text-xs opacity-100"
                >
                  <IconButton
                    variant="outline"
                    class="border-base-300 text-base-content-muted rounded-lg p-2 text-xs"
                    onclick={() => onOpenSelectAgentModal(skill)}
                    title={$t("local.action.installToApps")}
                    ariaLabel={$t("local.action.installToApps")}
                  >
                    <Blend size={14} />
                  </IconButton>
                  <IconButton
                    variant="outline"
                    class="border-error-border text-error rounded-lg p-2 text-xs"
                    onclick={() => onDeleteSkill(skill)}
                    title={$t("local.action.delete")}
                    ariaLabel={$t("local.action.delete")}
                  >
                    <Trash2 size={14} />
                  </IconButton>
                </div>
              </div>

              <!-- Second row: agents toggle button -->
              <button
                class="text-base-content-subtle hover:border-base-300 hover:bg-base-300 inline-flex items-center gap-1 rounded-md border border-transparent px-1 py-0.5 text-[11px] transition cursor-pointer"
                onclick={() => {
                  expandedSkillId =
                    expandedSkillId === skill.key ? null : skill.key;
                }}
                type="button"
              >
                <span>{$t("local.section.managedCount", { count: skill.agents.length })}</span>
                <ChevronsUpDown
                  class={expandedSkillId === skill.key ? "rotate-180" : ""}
                  size={12}
                />
              </button>

              <!-- Expanded Agents Grid -->
              {#if expandedSkillId === skill.key}
                <div class="mt-3 flex flex-wrap gap-2">
                  {#each skill.agents as agentId}
                    {@const displayName = agentMap.get(agentId)}
                    {#if displayName}
                      <div class="border-base-300 bg-base-300 text-base-content-subtle rounded-lg border px-3 py-1.5 text-xs">
                        {displayName}
                      </div>
                    {/if}
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        {/if}
      </div>
    {/if}
  </div>
</section>
