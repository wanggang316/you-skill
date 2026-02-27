<script>
  import { RefreshCw, Search, ChevronsUpDown, Loader2 } from "@lucide/svelte";
  import IconButton from "./ui/IconButton.svelte";
  import SelectField from "./ui/SelectField.svelte";
  import { t } from "../i18n";

  let {
    localSearch = $bindable(),
    localAgent = $bindable(),
    agents,
    localLoading,
    localError,
    filteredLocalSkills,
    agentMap,
    skillsWithUpdate = [],
    updatingSkills = [],
    onRefresh,
    onDeleteSkill,
    onViewSkill,
    onOpenSelectAgentModal,
    onUpdateSkill,
  } = $props();

  // Check if a skill has an update available
  /** @param {import('../api/skills').LocalSkill} skill */
  function hasUpdate(skill) {
    return skillsWithUpdate.some((s) => s.name === skill.name);
  }

  // Check if a skill is currently updating
  /** @param {import('../api/skills').LocalSkill} skill */
  function isUpdating(skill) {
    return updatingSkills.includes(skill.name);
  }

  /** @param {{ installed_agent_apps?: { id: string }[] }} skill */
  function appCount(skill) {
    return new Set((skill.installed_agent_apps || []).map((app) => app.id)).size;
  }
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
      <SelectField bind:value={localAgent}>
          <option value="all">{$t("local.agent.all")}</option>
          {#each agents as agent}
            <option value={agent.id}>{agent.display_name}</option>
          {/each}
      </SelectField>
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
        {#each filteredLocalSkills as skill}
          <div
            class="border-base-300 bg-base-100 hover:bg-base-200 rounded-2xl border p-4 transition"
          >
            <div class="flex flex-wrap items-center justify-between gap-3">
              <div class="flex items-center gap-2">
                <button
                  class="cursor-pointer border-none bg-transparent p-0 text-base"
                  onclick={() => onViewSkill(skill)}
                  type="button"
                >
                  {skill.name}
                </button>
                <!-- <span
                  class="bg-base-300 text-base-content-subtle rounded-full px-2 py-0.5 text-[11px]"
                >
                  {skill.source_type}
                </span> -->
              </div>
              <div class="flex items-center gap-2">
                {#if hasUpdate(skill) && onUpdateSkill}
                  <button
                    class="border-base-300 bg-base-300 text-primary hover:bg-primary hover:text-primary-content inline-flex h-6 items-center rounded-lg border px-2 py-0.5 text-xs transition disabled:cursor-not-allowed disabled:opacity-50"
                    onclick={(e) => {
                      e?.stopPropagation();
                      onUpdateSkill(skill);
                    }}
                    disabled={isUpdating(skill)}
                    type="button"
                  >
                    {#if isUpdating(skill)}
                      <Loader2 size={12} class="animate-spin" />
                    {:else}
                      {$t("remote.update")}
                    {/if}
                  </button>
                {/if}
                <button
                  class="border-base-300 bg-base-300 text-error hover:bg-error hover:text-primary-content inline-flex items-center rounded-lg border px-2 py-0.5 text-xs transition"
                  onclick={(e) => {
                    e?.stopPropagation();
                    onDeleteSkill(skill);
                  }}
                  type="button"
                >
                  {$t("local.action.delete")}
                </button>
              </div>
            </div>

            <button
              class="text-base-content/55 hover:text-base-content inline-flex cursor-pointer items-center gap-1 rounded-md border border-transparent p-0 text-[11px] transition"
              onclick={() => onOpenSelectAgentModal(skill)}
              type="button"
            >
              <span>{$t("local.section.appCount", { count: appCount(skill) })}</span>
              <ChevronsUpDown size={10} />
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</section>
