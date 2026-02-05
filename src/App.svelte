<script>
  import { onMount } from "svelte";
  import {
    ChevronLeft,
    ChevronDown,
    Blend,
    CloudDownload,
    HardDrive,
    Link2,
    Plus,
    RefreshCw,
    Search,
    Trash2,
    X,
  } from "lucide-svelte";
  import { api } from "./lib/tauri";

  let currentView = "list";
  let activeTab = "local";

  let localSkills = [];
  let localSearch = "";
  let localAgent = "all";
  let localLoading = false;
  let localError = "";
  let scanRoots = [];
  let newScanRoot = "";

  let remoteSkills = [];
  let remoteQuery = "";
  let remotePage = 1;
  let remotePageSize = 50;
  let remoteHasMore = false;
  let remoteLoading = false;
  let remoteError = "";

  let agents = [];
  let installAgent = "cursor";
  let installGlobal = true;
  let installLog = "";
  let installingSkill = "";
  let linkBusy = false;
  let editingSkillKey = "";
  let editSelection = [];

  $: allSelected = agents.length > 0 && editSelection.length === agents.length;

  $: agentMap = new Map(agents.map((agent) => [agent.id, agent.display_name]));

  $: filteredLocalSkills = (
    Array.isArray(localSkills) ? localSkills : []
  ).filter((skill) => {
    const needle = localSearch.trim().toLowerCase();
    const matchesSearch =
      !needle ||
      skill.name.toLowerCase().includes(needle) ||
      (skill.description || "").toLowerCase().includes(needle);
    const matchesAgent =
      localAgent === "all" || (skill.agents || []).includes(localAgent);
    return matchesSearch && matchesAgent;
  });

  $: managedSkills = filteredLocalSkills
    .filter((skill) => skill.managed_status === "managed")
    .map((skill) => ({
      ...skill,
      key: `${skill.name}::${skill.scope}::${skill.canonical_path}`,
    }))
    .sort((a, b) => (b.created_at || 0) - (a.created_at || 0));
  $: unmanagedSkills = filteredLocalSkills
    .filter((skill) =>
      ["unmanaged", "mixed", "unknown"].includes(skill.managed_status),
    )
    .sort((a, b) => (b.created_at || 0) - (a.created_at || 0));

  onMount(async () => {
    await loadAgents();
    await refreshLocal();
    await loadRemote(true);
  });

  async function loadAgents() {
    try {
      agents = await api.listAgents();
      if (agents.length > 0) {
        installAgent = agents[0].id;
      }
    } catch (error) {
      console.error(error);
    }
  }

  async function refreshLocal() {
    localLoading = true;
    localError = "";
    try {
      localSkills = await api.scanLocalSkills();
      scanRoots = await api.getScanRoots();
    } catch (error) {
      localError = String(error);
    } finally {
      localLoading = false;
    }
  }

  async function addRoot() {
    if (!newScanRoot.trim()) return;
    try {
      scanRoots = await api.addScanRoot(newScanRoot.trim());
      newScanRoot = "";
      await refreshLocal();
    } catch (error) {
      localError = String(error);
    }
  }

  async function removeRoot(path) {
    try {
      scanRoots = await api.removeScanRoot(path);
      await refreshLocal();
    } catch (error) {
      localError = String(error);
    }
  }

  async function loadRemote(reset = false) {
    remoteLoading = true;
    remoteError = "";
    try {
      if (reset) {
        remotePage = 1;
      }
      const response = await api.fetchRemoteSkills(
        remotePage,
        remotePageSize,
        remoteQuery,
      );
      remoteHasMore = response.has_more;
      if (reset) {
        remoteSkills = response.skills;
      } else {
        remoteSkills = [...remoteSkills, ...response.skills];
      }
    } catch (error) {
      remoteError = String(error);
    } finally {
      remoteLoading = false;
    }
  }

  async function handleSearchRemote() {
    await loadRemote(true);
  }

  async function loadMoreRemote() {
    if (!remoteHasMore) return;
    remotePage += 1;
    await loadRemote(false);
  }

  async function handleInstall(skill) {
    installLog = "";
    installingSkill = skill.id;
    try {
      const result = await api.installSkill({
        source: skill.source,
        skill_id: skill.skill_id,
        agent: installAgent,
        global: installGlobal,
        project_dir: null,
      });
      installLog = result.message;
      if (!result.success) {
        installLog = `${result.message}\n${result.stderr || result.stdout}`;
      } else {
        await refreshLocal();
      }
    } catch (error) {
      installLog = String(error);
    } finally {
      installingSkill = "";
    }
  }

  async function handleUnify(skill) {
    if (!skill || !skill.agents || skill.agents.length === 0) {
      localError = "无法识别当前 skill 的应用信息";
      return;
    }
    const agent = skill.agents[0];
    try {
      const check = await api.checkCanonicalSkill(skill.name, skill.scope);
      let prefer = "current";
      if (check.exists) {
        const keepCanonical = window.confirm(
          "检测到 .agents 下已存在同名 skill。\n确定：保留 .agents 版本\n取消：保留当前版本",
        );
        prefer = keepCanonical ? "canonical" : "current";
      }
      const result = await api.unifySkill({
        name: skill.name,
        agent,
        scope: skill.scope,
        current_path: skill.canonical_path,
        prefer,
      });
      if (!result.success) {
        localError = result.message;
      } else {
        await refreshLocal();
      }
    } catch (error) {
      localError = String(error);
    }
  }

  async function handleBulkUnify() {
    if (!unmanagedSkills.length) return;
    for (const skill of unmanagedSkills) {
      if (!skill || !skill.agents || skill.agents.length === 0) continue;
      try {
        const check = await api.checkCanonicalSkill(skill.name, skill.scope);
        let prefer = "current";
        if (check.exists) {
          const keepCanonical = window.confirm(
            `检测到 .agents 下已存在同名 skill：${skill.name}\n确定：保留 .agents 版本\n取消：保留当前版本`,
          );
          prefer = keepCanonical ? "canonical" : "current";
        }
        const result = await api.unifySkill({
          name: skill.name,
          agent: skill.agents[0],
          scope: skill.scope,
          current_path: skill.canonical_path,
          prefer,
        });
        if (!result.success) {
          localError = result.message;
        }
      } catch (error) {
        localError = String(error);
      }
    }
    await refreshLocal();
  }

  async function handleDeleteSkill(skill) {
    const confirmed = window.confirm(
      `确认删除 ${skill.name}？此操作不可恢复。`,
    );
    if (!confirmed) return;
    try {
      await api.deleteSkill(skill.canonical_path);
      await refreshLocal();
    } catch (error) {
      localError = String(error);
    }
  }

  function openLinkDialog(skill) {
    if (editingSkillKey === skill.key) {
      editingSkillKey = "";
      editSelection = [];
      return;
    }
    editingSkillKey = skill.key;
    editSelection = [...(skill.agents || [])];
  }

  function toggleAgentSelection(agentId, enabled) {
    if (enabled) {
      if (!editSelection.includes(agentId)) {
        editSelection = [...editSelection, agentId];
      }
    } else {
      editSelection = editSelection.filter((id) => id !== agentId);
    }
  }

  function toggleSelectAll(enabled) {
    if (enabled) {
      editSelection = agents.map((agent) => agent.id);
    } else {
      editSelection = [];
    }
  }

  async function confirmAgentLinks() {
    const skill = managedSkills.find((item) => item.key === editingSkillKey);
    if (!skill || linkBusy) return;
    linkBusy = true;
    try {
      const currentSet = new Set(skill.agents || []);
      const targetSet = new Set(editSelection);
      for (const agent of agents) {
        const shouldLink = targetSet.has(agent.id);
        const isLinked = currentSet.has(agent.id);
        if (shouldLink !== isLinked) {
          await api.setAgentLink(skill.name, agent.id, skill.scope, shouldLink);
        }
      }
      await refreshLocal();
      editingSkillKey = "";
      editSelection = [];
    } catch (error) {
      localError = String(error);
    } finally {
      linkBusy = false;
    }
  }
</script>

<div class="min-h-screen bg-slate-50 text-slate-900">
  <header class="border-b border-slate-200 bg-white">
    <div class="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
      <div class="flex items-center gap-3">
        <div class="rounded-xl bg-slate-900 p-2 text-white">
          <HardDrive size={18} />
        </div>
        <div>
          <p class="text-lg font-semibold">Skill Kit</p>
          <p class="text-xs text-slate-500">本地管理与一键安装</p>
        </div>
      </div>
      <div class="flex items-center gap-3 text-sm">
        {#if currentView === "list"}
          <div class="flex items-center gap-2 rounded-full bg-slate-100 p-1">
            <button
              class={`rounded-full px-4 py-2 ${activeTab === "local" ? "bg-white shadow-sm" : "text-slate-500"}`}
              on:click={() => (activeTab = "local")}
            >
              本地技能
            </button>
            <button
              class={`rounded-full px-4 py-2 ${activeTab === "remote" ? "bg-white shadow-sm" : "text-slate-500"}`}
              on:click={() => (activeTab = "remote")}
            >
              远程技能库
            </button>
          </div>
          <button
            class="flex items-center gap-2 rounded-xl bg-slate-900 px-3 py-2 text-sm text-white"
            on:click={() => (currentView = "add")}
            title="新增"
          >
            <Plus size={16} />
            新增
          </button>
        {:else}
          <button
            class="flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-sm"
            on:click={() => (currentView = "list")}
            title="返回"
          >
            <ChevronLeft size={16} />
            返回
          </button>
        {/if}
      </div>
    </div>
  </header>

  <main class="mx-auto max-w-6xl px-6 py-6">
    {#if currentView === "add"}
      <section class="space-y-6">
        <div class="rounded-2xl border border-slate-200 bg-white p-4">
          <p class="mb-3 text-sm font-semibold text-slate-700">新增扫描路径</p>
          <div class="flex flex-wrap items-center gap-3">
            <input
              class="flex-1 rounded-xl border border-slate-200 bg-slate-50 px-4 py-2 text-sm"
              placeholder="添加自定义扫描路径（项目根目录）"
              bind:value={newScanRoot}
            />
            <button
              class="rounded-xl bg-slate-900 p-2 text-sm text-white"
              on:click={addRoot}
              title="添加路径"
            >
              <Plus size={16} />
            </button>
          </div>
          {#if localError}
            <p class="mt-3 text-sm text-rose-500">{localError}</p>
          {/if}
          {#if scanRoots.length > 0}
            <div class="mt-3 space-y-2 text-sm text-slate-600">
              {#each scanRoots as root}
                <div
                  class="flex items-center justify-between rounded-lg bg-slate-50 px-3 py-2"
                >
                  <span>{root}</span>
                  <button
                    class="text-rose-500"
                    on:click={() => removeRoot(root)}
                    title="移除路径"
                  >
                    <X size={14} />
                  </button>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </section>
    {:else if activeTab === "local"}
      <section class="space-y-6">
        <div class="rounded-2xl border border-slate-200 bg-white p-4">
          <div class="flex flex-wrap items-center gap-3">
            <div class="relative flex-1">
              <Search class="absolute left-3 top-3 text-slate-400" size={16} />
              <input
                class="w-full rounded-xl border border-slate-200 bg-slate-50 px-9 py-2 text-sm focus:border-slate-400 focus:outline-none"
                placeholder="搜索本地技能或路径"
                bind:value={localSearch}
              />
            </div>
            <select
              class="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
              bind:value={localAgent}
            >
              <option value="all">全部 Agent</option>
              {#each agents as agent}
                <option value={agent.id}>{agent.display_name}</option>
              {/each}
            </select>
            <button
              class="rounded-xl border border-slate-200 p-2 text-sm"
              on:click={refreshLocal}
              title="刷新"
            >
              <RefreshCw size={16} />
            </button>
          </div>
          {#if localError}
            <p class="mt-3 text-sm text-rose-500">{localError}</p>
          {/if}
        </div>

        <div class="space-y-3">
          {#if localLoading}
            <div
              class="rounded-2xl border border-dashed border-slate-200 bg-white p-6 text-center text-sm text-slate-500"
            >
              正在扫描本地 skill...
            </div>
          {:else if filteredLocalSkills.length === 0}
            <div
              class="rounded-2xl border border-dashed border-slate-200 bg-white p-6 text-center text-sm text-slate-500"
            >
              未找到本地技能
            </div>
          {:else}
            <div class="space-y-2">
              <p class="text-sm font-semibold text-slate-700">我的 Skills</p>
              {#if managedSkills.length === 0}
                <div
                  class="rounded-xl border border-dashed border-slate-200 bg-white p-4 text-sm text-slate-500"
                >
                  暂无统一管理的 skill
                </div>
              {:else}
                {#each managedSkills as skill}
                  <div class="rounded-2xl border border-slate-200 bg-white p-4">
                    <div
                      class="flex flex-wrap items-center justify-between gap-3"
                    >
                      <div>
                        <p class="text-base font-semibold">{skill.name}</p>
                        {#if editingSkillKey !== skill.key}
                          <div class="mt-2 flex flex-wrap gap-2">
                            {#each skill.agents as agentId}
                              <div
                                class="inline-flex items-center rounded-full bg-slate-100 px-2.5 py-1 text-xs text-slate-600"
                              >
                                {agentMap.get(agentId) || agentId}
                              </div>
                            {/each}
                          </div>
                        {/if}
                      </div>
                      <div
                        class="flex items-center gap-3 text-xs text-slate-400"
                      >
                        <button
                          class={`rounded-lg border p-2 text-xs ${editingSkillKey === skill.key ? "border-slate-900 text-slate-900" : "border-slate-200 text-slate-600"}`}
                          on:click={() => openLinkDialog(skill)}
                          title="安装到应用"
                        >
                          <Blend size={14} />
                        </button>
                        <button
                          class="rounded-lg border border-rose-200 p-2 text-xs text-rose-500"
                          on:click={() => handleDeleteSkill(skill)}
                          title="删除"
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </div>
                    {#if editingSkillKey === skill.key}
                      <div
                        class="mt-4 rounded-xl border border-slate-200 bg-slate-50 p-3"
                      >
                        <div
                          class="flex items-center justify-between text-xs text-slate-500"
                        >
                          <label class="inline-flex items-center gap-2">
                            <input
                              type="checkbox"
                              checked={allSelected}
                              on:change={(event) =>
                                toggleSelectAll(event.target.checked)}
                              disabled={linkBusy}
                            />
                            全选
                          </label>
                          <button
                            class="rounded-lg bg-slate-900 px-3 py-1.5 text-white"
                            on:click={confirmAgentLinks}
                            disabled={linkBusy}
                          >
                            确认
                          </button>
                        </div>
                        <div class="mt-3 flex flex-wrap gap-2">
                          {#each agents as agent}
                            <label
                              class="inline-flex items-center gap-3 rounded-lg bg-white px-3 py-2 text-sm"
                            >
                              <input
                                type="checkbox"
                                checked={editSelection.includes(agent.id)}
                                on:change={(event) =>
                                  toggleAgentSelection(
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

            {#if unmanagedSkills.length > 0}
              <div class="space-y-2">
                <div class="flex items-center justify-between">
                  <p class="text-sm font-semibold text-slate-700">
                    独立安装（待导入）
                  </p>
                  <button
                    class="rounded-lg border border-slate-200 px-3 py-1.5 text-xs text-slate-600"
                    on:click={handleBulkUnify}
                  >
                    一键导入
                  </button>
                </div>
                {#each unmanagedSkills as skill}
                  <div class="rounded-2xl border border-slate-200 bg-white p-4">
                    <div
                      class="flex flex-wrap items-center justify-between gap-3"
                    >
                      <div>
                        <p class="text-base font-semibold">{skill.name}</p>
                        <div class="mt-2 flex flex-wrap gap-2">
                          {#each skill.agents as agentId}
                            <div
                              class="inline-flex items-center rounded-full bg-slate-100 px-2.5 py-1 text-xs text-slate-600"
                            >
                              {agentMap.get(agentId) || agentId}
                            </div>
                          {/each}
                        </div>
                      </div>
                      <div
                        class="flex items-center gap-3 text-xs text-slate-400"
                      >
                        {#if skill.managed_status === "mixed"}
                          <span
                            class="rounded-full bg-amber-100 px-2 py-0.5 text-[10px] text-amber-700"
                          >
                            独立副本
                          </span>
                        {/if}
                        {#if skill.name_conflict}
                          <span
                            class="rounded-full bg-rose-100 px-2 py-0.5 text-[10px] text-rose-700"
                          >
                            有同名
                          </span>
                        {/if}
                        <button
                          class="rounded-lg border border-slate-200 px-3 py-1.5 text-xs text-slate-600"
                          on:click={() => handleUnify(skill)}
                          title="导入"
                        >
                          导入
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
    {:else}
      <section class="space-y-6">
        <div class="rounded-2xl border border-slate-200 bg-white p-4">
          <div class="flex flex-wrap items-center gap-3">
            <div class="relative flex-1">
              <Search class="absolute left-3 top-3 text-slate-400" size={16} />
              <input
                class="w-full rounded-xl border border-slate-200 bg-slate-50 px-9 py-2 text-sm focus:border-slate-400 focus:outline-none"
                placeholder="搜索远程技能（名称或仓库）"
                bind:value={remoteQuery}
                on:keydown={(event) =>
                  event.key === "Enter" && handleSearchRemote()}
              />
            </div>
            <select
              class="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
              bind:value={installAgent}
            >
              {#each agents as agent}
                <option value={agent.id}>{agent.display_name}</option>
              {/each}
            </select>
            <label class="flex items-center gap-2 text-xs text-slate-500">
              <input type="checkbox" bind:checked={installGlobal} />
              全局安装
            </label>
            <button
              class="rounded-xl border border-slate-200 p-2 text-sm"
              on:click={handleSearchRemote}
              title="搜索"
            >
              <Search size={16} />
            </button>
          </div>
          {#if remoteError}
            <p class="mt-3 text-sm text-rose-500">{remoteError}</p>
          {/if}
          {#if installLog}
            <div
              class="mt-3 whitespace-pre-wrap rounded-lg bg-slate-50 px-3 py-2 text-xs text-slate-600"
            >
              {installLog}
            </div>
          {/if}
        </div>

        <div class="space-y-3">
          {#if remoteLoading && remoteSkills.length === 0}
            <div
              class="rounded-2xl border border-dashed border-slate-200 bg-white p-6 text-center text-sm text-slate-500"
            >
              正在加载技能库...
            </div>
          {:else if remoteSkills.length === 0}
            <div
              class="rounded-2xl border border-dashed border-slate-200 bg-white p-6 text-center text-sm text-slate-500"
            >
              暂无技能数据
            </div>
          {:else}
            {#each remoteSkills as skill}
              <div class="rounded-2xl border border-slate-200 bg-white p-4">
                <div class="flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <p class="text-base font-semibold">{skill.name}</p>
                    <p class="text-xs text-slate-500">{skill.source}</p>
                    <p class="mt-1 text-xs text-slate-400">
                      {skill.installs} installs
                    </p>
                  </div>
                  <button
                    class="rounded-xl bg-slate-900 p-2 text-xs text-white"
                    on:click={() => handleInstall(skill)}
                    disabled={installingSkill === skill.id}
                    title="一键安装"
                  >
                    {#if installingSkill === skill.id}
                      <RefreshCw size={14} class="animate-spin" />
                    {:else}
                      <CloudDownload size={14} />
                    {/if}
                  </button>
                </div>
              </div>
            {/each}
          {/if}
        </div>

        <div class="flex items-center justify-center">
          <button
            class="rounded-xl border border-slate-200 p-2 text-sm"
            on:click={loadMoreRemote}
            disabled={!remoteHasMore || remoteLoading}
            title={remoteHasMore ? "加载更多" : "没有更多了"}
          >
            {#if remoteLoading}
              <RefreshCw size={16} class="animate-spin" />
            {:else}
              <ChevronDown size={16} />
            {/if}
          </button>
        </div>
      </section>
    {/if}
  </main>
</div>
