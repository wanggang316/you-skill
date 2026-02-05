<script>
  import { Blend, RefreshCw, Search, Trash2 } from '@lucide/svelte'

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
    onUnifySkill
  } = $props()
</script>

<section class="space-y-6">
  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-4">
    <div class="flex flex-wrap items-center gap-3">
      <div class="relative flex-1">
        <Search
          class="absolute left-3 top-3 text-[var(--base-content-subtle)]"
          size={16}
        />
        <input
          class="w-full rounded-xl border border-[var(--base-300)] bg-[var(--base-200)] px-9 py-2 text-sm text-[var(--base-content)] placeholder:text-[var(--base-content-subtle)] focus:border-[var(--base-300)] focus:outline-none"
          placeholder="搜索本地技能或路径"
          bind:value={localSearch}
        />
      </div>
      <select
        class="rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 py-2 text-sm text-[var(--base-content)]"
        bind:value={localAgent}
      >
        <option value="all">全部 Agent</option>
        {#each agents as agent}
          <option value={agent.id}>{agent.display_name}</option>
        {/each}
      </select>
      <button
        class="rounded-xl border border-[var(--base-300)] p-2 text-sm text-[var(--base-content)]"
        onclick={onRefresh}
        title="刷新"
        type="button"
      >
        <RefreshCw size={16} />
      </button>
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
        正在扫描本地 skill...
      </div>
    {:else if filteredLocalSkills.length === 0}
      <div
        class="rounded-2xl border border-dashed border-[var(--base-300)] bg-[var(--base-100)] p-6 text-center text-sm text-[var(--base-content-muted)]"
      >
        未找到本地技能
      </div>
    {:else}
      <div class="space-y-2">
        <p class="text-sm font-semibold text-[var(--base-content-muted)]">
          我的 Skills
        </p>
        {#if managedSkills.length === 0}
          <div
            class="rounded-xl border border-dashed border-[var(--base-300)] bg-[var(--base-100)] p-4 text-sm text-[var(--base-content-muted)]"
          >
            暂无统一管理的 skill
          </div>
        {:else}
          {#each managedSkills as skill}
            <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-4">
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div>
                  <p class="text-base font-semibold">{skill.name}</p>
                  {#if editingSkillKey !== skill.key}
                    <div class="mt-2 flex flex-wrap gap-2">
                      {#each skill.agents as agentId}
                        <div
                          class="inline-flex items-center rounded-full bg-[var(--base-200)] px-2.5 py-1 text-xs text-[var(--base-content-subtle)]"
                        >
                          {agentMap.get(agentId) || agentId}
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
                <div class="flex items-center gap-3 text-xs text-[var(--base-content-faint)]">
                  <button
                    class={`rounded-lg border p-2 text-xs ${editingSkillKey === skill.key ? 'border-[var(--base-content)] text-[var(--base-content)]' : 'border-[var(--base-300)] text-[var(--base-content-muted)]'}`}
                    onclick={() => onOpenLinkDialog(skill)}
                    title="安装到应用"
                    type="button"
                  >
                    <Blend size={14} />
                  </button>
                  <button
                    class="rounded-lg border border-[var(--error-border)] p-2 text-xs text-[var(--error)]"
                    type="button"
                    onclick={(event) => {
                      event.stopPropagation()
                      onDeleteSkill(skill)
                    }}
                    title="删除"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
              {#if editingSkillKey === skill.key}
                <div class="mt-4 rounded-xl border border-[var(--base-300)] bg-[var(--base-200)] p-3">
                  <div class="flex items-center justify-between text-xs text-[var(--base-content-muted)]">
                    <label class="inline-flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={allSelected}
                        onchange={(event) =>
                          onToggleSelectAll(event.target.checked)}
                        disabled={linkBusy}
                      />
                      全选
                    </label>
                    <button
                      class="rounded-lg bg-[var(--primary)] px-3 py-1.5 text-[var(--primary-content)]"
                      onclick={onConfirmAgentLinks}
                      disabled={linkBusy}
                      type="button"
                    >
                      确认
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
                            onToggleAgentSelection(agent.id, event.target.checked)}
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

      {#if unmanagedSkills.length > 0}
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <p class="text-sm font-semibold text-[var(--base-content-muted)]">
              独立安装（待导入）
            </p>
            <button
              class="rounded-lg border border-[var(--base-300)] px-3 py-1.5 text-xs text-[var(--base-content-muted)]"
              onclick={onBulkUnify}
              type="button"
            >
              一键导入
            </button>
          </div>
          {#each unmanagedSkills as skill}
            <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-4">
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div>
                  <p class="text-base font-semibold">{skill.name}</p>
                  <div class="mt-2 flex flex-wrap gap-2">
                    {#each skill.agents as agentId}
                      <div
                        class="inline-flex items-center rounded-full bg-[var(--base-200)] px-2.5 py-1 text-xs text-[var(--base-content-subtle)]"
                      >
                        {agentMap.get(agentId) || agentId}
                      </div>
                    {/each}
                  </div>
                </div>
                <div class="flex items-center gap-3 text-xs text-[var(--base-content-faint)]">
                  {#if skill.managed_status === 'mixed'}
                    <span class="tag tag-warning">独立副本</span>
                  {/if}
                  {#if skill.name_conflict}
                    <span class="tag tag-error">有同名</span>
                  {/if}
                  {#if skill.conflict_with_managed}
                    <span class="tag tag-neutral">与 .agents 重名</span>
                  {/if}
                  <button
                    class="rounded-lg border border-[var(--base-300)] px-3 py-1.5 text-xs text-[var(--base-content-muted)]"
                    onclick={() => onUnifySkill(skill)}
                    title="导入"
                    type="button"
                  >
                    导入
                  </button>
                  <button
                    class="rounded-lg border border-[var(--error-border)] px-3 py-1.5 text-xs text-[var(--error)]"
                    type="button"
                    onclick={(event) => {
                      event.stopPropagation()
                      onDeleteSkill(skill)
                    }}
                    title="删除"
                  >
                    删除
                  </button>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</section>
