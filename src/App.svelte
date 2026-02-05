<script>
  import { onMount } from 'svelte'
  import {
    CloudDownload,
    Copy,
    HardDrive,
    MoveRight,
    RefreshCw,
    Search,
    Trash2
  } from 'lucide-svelte'
  import { api } from './lib/tauri'

  let activeTab = 'local'

  let localSkills = []
  let localSearch = ''
  let localAgent = 'all'
  let localLoading = false
  let localError = ''
  let scanRoots = []
  let newScanRoot = ''

  let remoteSkills = []
  let remoteQuery = ''
  let remotePage = 1
  let remotePageSize = 50
  let remoteHasMore = false
  let remoteLoading = false
  let remoteError = ''

  let agents = []
  let installAgent = 'cursor'
  let installGlobal = true
  let installLog = ''
  let installingSkill = ''

  $: filteredLocalSkills = localSkills.filter((skill) => {
    const needle = localSearch.trim().toLowerCase()
    const matchesSearch =
      !needle ||
      skill.name.toLowerCase().includes(needle) ||
      (skill.description || '').toLowerCase().includes(needle) ||
      skill.path.toLowerCase().includes(needle)
    const matchesAgent = localAgent === 'all' || skill.agent === localAgent
    return matchesSearch && matchesAgent
  })

  onMount(async () => {
    await loadAgents()
    await refreshLocal()
    await loadRemote(true)
  })

  async function loadAgents() {
    try {
      agents = await api.listAgents()
      if (agents.length > 0) {
        installAgent = agents[0].id
      }
    } catch (error) {
      console.error(error)
    }
  }

  async function refreshLocal() {
    localLoading = true
    localError = ''
    try {
      localSkills = await api.scanLocalSkills()
      scanRoots = await api.getScanRoots()
    } catch (error) {
      localError = String(error)
    } finally {
      localLoading = false
    }
  }

  async function addRoot() {
    if (!newScanRoot.trim()) return
    try {
      scanRoots = await api.addScanRoot(newScanRoot.trim())
      newScanRoot = ''
      await refreshLocal()
    } catch (error) {
      localError = String(error)
    }
  }

  async function removeRoot(path) {
    try {
      scanRoots = await api.removeScanRoot(path)
      await refreshLocal()
    } catch (error) {
      localError = String(error)
    }
  }

  async function handleDelete(path) {
    const confirmed = window.confirm('确认删除该 skill？此操作不可恢复。')
    if (!confirmed) return
    try {
      await api.deleteSkill(path)
      await refreshLocal()
    } catch (error) {
      localError = String(error)
    }
  }

  async function handleCopy(path) {
    const target = window.prompt('请输入复制目标路径', path)
    if (!target) return
    try {
      await api.copySkill(path, target)
      await refreshLocal()
    } catch (error) {
      localError = String(error)
    }
  }

  async function handleMove(path) {
    const target = window.prompt('请输入移动目标路径', path)
    if (!target) return
    try {
      await api.moveSkill(path, target)
      await refreshLocal()
    } catch (error) {
      localError = String(error)
    }
  }

  async function loadRemote(reset = false) {
    remoteLoading = true
    remoteError = ''
    try {
      if (reset) {
        remotePage = 1
      }
      const response = await api.fetchRemoteSkills(remotePage, remotePageSize, remoteQuery)
      remoteHasMore = response.has_more
      if (reset) {
        remoteSkills = response.skills
      } else {
        remoteSkills = [...remoteSkills, ...response.skills]
      }
    } catch (error) {
      remoteError = String(error)
    } finally {
      remoteLoading = false
    }
  }

  async function handleSearchRemote() {
    await loadRemote(true)
  }

  async function loadMoreRemote() {
    if (!remoteHasMore) return
    remotePage += 1
    await loadRemote(false)
  }

  async function handleInstall(skill) {
    installLog = ''
    installingSkill = skill.id
    try {
      const result = await api.installSkill({
        source: skill.source,
        skill_id: skill.skill_id,
        agent: installAgent,
        global: installGlobal,
        project_dir: null
      })
      installLog = result.message
      if (!result.success) {
        installLog = `${result.message}\n${result.stderr || result.stdout}`
      } else {
        await refreshLocal()
      }
    } catch (error) {
      installLog = String(error)
    } finally {
      installingSkill = ''
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
      <div class="flex items-center gap-2 rounded-full bg-slate-100 p-1 text-sm">
        <button
          class={`rounded-full px-4 py-2 ${activeTab === 'local' ? 'bg-white shadow-sm' : 'text-slate-500'}`}
          on:click={() => (activeTab = 'local')}
        >
          本地技能
        </button>
        <button
          class={`rounded-full px-4 py-2 ${activeTab === 'remote' ? 'bg-white shadow-sm' : 'text-slate-500'}`}
          on:click={() => (activeTab = 'remote')}
        >
          远程技能库
        </button>
      </div>
    </div>
  </header>

  <main class="mx-auto max-w-6xl px-6 py-6">
    {#if activeTab === 'local'}
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
              class="flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-sm"
              on:click={refreshLocal}
            >
              <RefreshCw size={16} /> 刷新
            </button>
          </div>
          {#if localError}
            <p class="mt-3 text-sm text-rose-500">{localError}</p>
          {/if}
        </div>

        <div class="rounded-2xl border border-slate-200 bg-white p-4">
          <p class="mb-3 text-sm font-semibold text-slate-700">扫描路径</p>
          <div class="flex flex-wrap items-center gap-3">
            <input
              class="flex-1 rounded-xl border border-slate-200 bg-slate-50 px-4 py-2 text-sm"
              placeholder="添加自定义扫描路径（项目根目录）"
              bind:value={newScanRoot}
            />
            <button
              class="rounded-xl bg-slate-900 px-4 py-2 text-sm text-white"
              on:click={addRoot}
            >
              添加
            </button>
          </div>
          {#if scanRoots.length > 0}
            <div class="mt-3 space-y-2 text-sm text-slate-600">
              {#each scanRoots as root}
                <div class="flex items-center justify-between rounded-lg bg-slate-50 px-3 py-2">
                  <span>{root}</span>
                  <button class="text-rose-500" on:click={() => removeRoot(root)}>移除</button>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <div class="space-y-3">
          {#if localLoading}
            <div class="rounded-2xl border border-dashed border-slate-200 bg-white p-6 text-center text-sm text-slate-500">
              正在扫描本地 skill...
            </div>
          {:else if filteredLocalSkills.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-200 bg-white p-6 text-center text-sm text-slate-500">
              未找到本地技能
            </div>
          {:else}
            {#each filteredLocalSkills as skill}
              <div class="rounded-2xl border border-slate-200 bg-white p-4">
                <div class="flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <p class="text-base font-semibold">{skill.name}</p>
                    <p class="text-xs text-slate-500">{skill.description || '暂无描述'}</p>
                    <p class="mt-1 text-xs text-slate-400">
                      {skill.agent} · {skill.install_mode}
                    </p>
                  </div>
                  <div class="flex items-center gap-2">
                    <button
                      class="flex items-center gap-1 rounded-lg border border-slate-200 px-3 py-1.5 text-xs"
                      on:click={() => handleCopy(skill.path)}
                    >
                      <Copy size={14} /> 复制
                    </button>
                    <button
                      class="flex items-center gap-1 rounded-lg border border-slate-200 px-3 py-1.5 text-xs"
                      on:click={() => handleMove(skill.path)}
                    >
                      <MoveRight size={14} /> 移动
                    </button>
                    <button
                      class="flex items-center gap-1 rounded-lg border border-rose-200 px-3 py-1.5 text-xs text-rose-500"
                      on:click={() => handleDelete(skill.path)}
                    >
                      <Trash2 size={14} /> 删除
                    </button>
                  </div>
                </div>
                <div class="mt-3 rounded-lg bg-slate-50 px-3 py-2 text-xs text-slate-500">
                  {skill.path}
                </div>
                {#if skill.real_path}
                  <div class="mt-2 rounded-lg bg-slate-50 px-3 py-2 text-xs text-slate-500">
                    指向：{skill.real_path}
                  </div>
                {/if}
              </div>
            {/each}
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
                on:keydown={(event) => event.key === 'Enter' && handleSearchRemote()}
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
              class="flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-sm"
              on:click={handleSearchRemote}
            >
              <Search size={16} /> 搜索
            </button>
          </div>
          {#if remoteError}
            <p class="mt-3 text-sm text-rose-500">{remoteError}</p>
          {/if}
          {#if installLog}
            <div class="mt-3 whitespace-pre-wrap rounded-lg bg-slate-50 px-3 py-2 text-xs text-slate-600">
              {installLog}
            </div>
          {/if}
        </div>

        <div class="space-y-3">
          {#if remoteLoading && remoteSkills.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-200 bg-white p-6 text-center text-sm text-slate-500">
              正在加载技能库...
            </div>
          {:else if remoteSkills.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-200 bg-white p-6 text-center text-sm text-slate-500">
              暂无技能数据
            </div>
          {:else}
            {#each remoteSkills as skill}
              <div class="rounded-2xl border border-slate-200 bg-white p-4">
                <div class="flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <p class="text-base font-semibold">{skill.name}</p>
                    <p class="text-xs text-slate-500">{skill.source}</p>
                    <p class="mt-1 text-xs text-slate-400">{skill.installs} installs</p>
                  </div>
                  <button
                    class="flex items-center gap-2 rounded-xl bg-slate-900 px-3 py-2 text-xs text-white"
                    on:click={() => handleInstall(skill)}
                    disabled={installingSkill === skill.id}
                  >
                    <CloudDownload size={14} />
                    {installingSkill === skill.id ? '安装中...' : '一键安装'}
                  </button>
                </div>
              </div>
            {/each}
          {/if}
        </div>

        <div class="flex items-center justify-center">
          <button
            class="rounded-xl border border-slate-200 px-4 py-2 text-sm"
            on:click={loadMoreRemote}
            disabled={!remoteHasMore || remoteLoading}
          >
            {remoteLoading ? '加载中...' : remoteHasMore ? '加载更多' : '没有更多了'}
          </button>
        </div>
      </section>
    {/if}
  </main>
</div>
