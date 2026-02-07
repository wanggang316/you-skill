<script lang="ts">
  import { onMount } from 'svelte'
  import { confirm } from '@tauri-apps/plugin-dialog'
  import { open } from '@tauri-apps/plugin-shell'
  import AddSkillModal from '../lib/components/AddSkillModal.svelte'
  import InstallConfirmModal from '../lib/components/InstallConfirmModal.svelte'
  import LocalSkillsSection from '../lib/components/LocalSkillsSection.svelte'
  import PageHeader from '../lib/components/PageHeader.svelte'
  import RemoteSkillsSection from '../lib/components/RemoteSkillsSection.svelte'
  import SettingsPanel from '../lib/components/SettingsPanel.svelte'
  import SkillDetail from '../lib/components/SkillDetail.svelte'
  import { api } from '../lib/api/skills'
  import { t } from '../lib/i18n'
  import { loadSettings } from '../lib/stores/settings'

  let currentView = $state('list')
  let activeTab = $state('local')
  let selectedSkill = $state(null)
  let addSkillModalOpen = $state(false)

  let localSkills = $state([])
  let localSearch = $state('')
  let localAgent = $state('all')
  let localLoading = $state(false)
  let localError = $state('')

  let remoteSkills = $state([])
  let remoteQuery = $state('')
  let remoteSkip = $state(0)
  let remoteLimit = $state(20)
  let remoteHasMore = $state(false)
  let remoteLoading = $state(false)
  let remoteError = $state('')
  let remoteTotal = $state(0)
  let remoteSortBy = $state('star_count')
  let remoteSortOrder = $state('desc')

  let agents = $state([])
  let installAgent = $state('cursor')
  let installGlobal = $state(true)
  let installLog = $state('')
  let installingSkill = $state('')
  let linkBusy = $state(false)
  let editingSkillKey = $state('')
  let editSelection = $state([])

  // Install confirm modal state
  let installConfirmModalOpen = $state(false)
  let pendingInstallSkill = $state(null)
  let pendingInstallAgents = $state([])
  let isDownloading = $state(false)
  let downloadError = $state('')

  const allSelected = $derived(
    agents.length > 0 && editSelection.length === agents.length
  )

  const agentMap = $derived.by(
    () => new Map(agents.map((agent) => [agent.id, agent.display_name]))
  )

  const filteredLocalSkills = $derived.by(() => {
    const source = Array.isArray(localSkills) ? localSkills : []
    const needle = localSearch.trim().toLowerCase()
    return source.filter((skill) => {
      const matchesSearch =
        !needle ||
        skill.name.toLowerCase().includes(needle) ||
        (skill.description || '').toLowerCase().includes(needle)
      const matchesAgent =
        localAgent === 'all' || (skill.agents || []).includes(localAgent)
      return matchesSearch && matchesAgent
    })
  })

  const managedSkills = $derived.by(() =>
    filteredLocalSkills
      .filter((skill) => skill.managed_status === 'managed')
      .map((skill) => ({
        ...skill,
        key: `${skill.name}::${skill.scope}::${skill.canonical_path}`
      }))
      .sort((a, b) => (b.created_at || 0) - (a.created_at || 0))
  )

  const unmanagedSkills = $derived.by(() =>
    filteredLocalSkills
      .filter((skill) =>
        ['unmanaged', 'mixed', 'unknown'].includes(skill.managed_status)
      )
      .sort((a, b) => (b.created_at || 0) - (a.created_at || 0))
  )

  onMount(async () => {
    await loadSettings()
    await loadAgents()
    await refreshLocal()
    await loadRemote(true)
  })

  const loadAgents = async () => {
    try {
      agents = await api.listAgents()
      if (agents.length > 0) {
        installAgent = agents[0].id
      }
    } catch (error) {
      console.error(error)
    }
  }

  const refreshLocal = async () => {
    localLoading = true
    localError = ''
    try {
      localSkills = await api.scanLocalSkills()
    } catch (error) {
      localError = String(error)
    } finally {
      localLoading = false
    }
  }

  const loadRemote = async (reset = false) => {
    remoteLoading = true
    remoteError = ''
    try {
      if (reset) {
        remoteSkip = 0
        remoteSkills = []
      }
      const response = await api.fetchRemoteSkills({
        skip: remoteSkip,
        limit: remoteLimit,
        search: remoteQuery,
        sort_by: remoteSortBy,
        sort_order: remoteSortOrder
      })
      remoteHasMore = response.has_more
      remoteTotal = response.total
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

  const handleSearchRemote = async () => {
    await loadRemote(true)
  }

  const loadMoreRemote = async () => {
    if (!remoteHasMore) return
    remoteSkip += remoteLimit
    await loadRemote(false)
  }

  const handleSortChange = async (sortBy: string, sortOrder: string) => {
    remoteSortBy = sortBy
    remoteSortOrder = sortOrder
    await loadRemote(true)
  }

  const handleInstall = async (skill) => {
    // First download/detect the skill
    isDownloading = true
    downloadError = ''
    installingSkill = skill.id
    try {
      // Detect skills from the GitHub URL
      const detectedSkills = await api.detectGithubSkills(skill.url)
      // Find the matching skill by name or path
      const matchingSkill = detectedSkills.find(
        s => s.name === skill.name || skill.path?.includes(s.name) || s.path === skill.path
      )
      if (!matchingSkill && detectedSkills.length > 0) {
        // Use first detected skill if no exact match
        pendingInstallSkill = { ...skill, detectedPath: detectedSkills[0].path }
      } else if (matchingSkill) {
        pendingInstallSkill = { ...skill, detectedPath: matchingSkill.path }
      } else {
        downloadError = 'No skills found in repository'
        return
      }
      // Open confirm modal after successful download
      installConfirmModalOpen = true
    } catch (error) {
      downloadError = String(error)
      installLog = String(error)
    } finally {
      isDownloading = false
      installingSkill = ''
    }
  }

  const handleConfirmInstall = async (selectedAgents) => {
    if (!pendingInstallSkill) return
    installLog = ''
    installingSkill = pendingInstallSkill.id
    pendingInstallAgents = selectedAgents
    try {
      // Use installGithubSkill API for remote skills
      const skillPath = pendingInstallSkill.detectedPath || pendingInstallSkill.path || pendingInstallSkill.name
      const result = await api.installGithubSkill({
        url: pendingInstallSkill.url,
        skill_path: skillPath,
        agents: selectedAgents
      })
      if (!result.success) {
        installLog = `${result.message}\n${result.stderr || result.stdout}`
      } else {
        installLog = ''
        await refreshLocal()
      }
    } catch (error) {
      installLog = String(error)
    } finally {
      installingSkill = ''
      pendingInstallSkill = null
      pendingInstallAgents = []
    }
  }

  const handleOpenUrl = async (url) => {
    try {
      await open(url)
    } catch (error) {
      console.error('Failed to open URL:', error)
    }
  }

  const handleUnify = async (skill) => {
    if (!skill || !skill.agents || skill.agents.length === 0) {
      localError = $t('error.noSkillAgent')
      return
    }
    const agent = skill.agents[0]
    try {
      const check = await api.checkCanonicalSkill(skill.name, skill.scope)
      let prefer = 'current'
      if (check.exists) {
        const keepCanonical = await confirm(
          $t('confirm.duplicateSkill'),
          { title: $t('confirm.duplicateTitle') }
        )
        prefer = keepCanonical ? 'canonical' : 'current'
      }
      const result = await api.unifySkill({
        name: skill.name,
        agent,
        scope: skill.scope,
        current_path: skill.canonical_path,
        prefer
      })
      if (!result.success) {
        localError = result.message
      } else {
        await refreshLocal()
      }
    } catch (error) {
      localError = String(error)
    }
  }

  const handleBulkUnify = async () => {
    if (!unmanagedSkills.length) return
    for (const skill of unmanagedSkills) {
      if (!skill || !skill.agents || skill.agents.length === 0) continue
      try {
        const check = await api.checkCanonicalSkill(skill.name, skill.scope)
        let prefer = 'current'
        if (check.exists) {
          const keepCanonical = await confirm(
            $t('confirm.duplicateSkillWithName', { name: skill.name }),
            { title: $t('confirm.duplicateTitle') }
          )
          prefer = keepCanonical ? 'canonical' : 'current'
        }
        const result = await api.unifySkill({
          name: skill.name,
          agent: skill.agents[0],
          scope: skill.scope,
          current_path: skill.canonical_path,
          prefer
        })
        if (!result.success) {
          localError = result.message
        }
      } catch (error) {
        localError = String(error)
      }
    }
    await refreshLocal()
  }

  const handleDeleteSkill = async (skill) => {
    try {
      const confirmed = await confirm(
        $t('confirm.deleteSkill', { name: skill.name }),
        { title: $t('confirm.deleteTitle') }
      )
      if (!confirmed) return
      await api.deleteSkill(skill.canonical_path)
      await refreshLocal()
    } catch (error) {
      localError = String(error)
    }
  }

  const openLinkDialog = (skill) => {
    if (editingSkillKey === skill.key) {
      editingSkillKey = ''
      editSelection = []
      return
    }
    editingSkillKey = skill.key
    editSelection = [...(skill.agents || [])]
  }

  const toggleAgentSelection = (agentId, enabled) => {
    if (enabled) {
      if (!editSelection.includes(agentId)) {
        editSelection = [...editSelection, agentId]
      }
    } else {
      editSelection = editSelection.filter((id) => id !== agentId)
    }
  }

  const toggleSelectAll = (enabled) => {
    if (enabled) {
      editSelection = agents.map((agent) => agent.id)
    } else {
      editSelection = []
    }
  }

  const confirmAgentLinks = async () => {
    const skill = managedSkills.find((item) => item.key === editingSkillKey)
    if (!skill || linkBusy) return
    linkBusy = true
    try {
      const currentSet = new Set(skill.agents || [])
      const targetSet = new Set(editSelection)
      for (const agent of agents) {
        const shouldLink = targetSet.has(agent.id)
        const isLinked = currentSet.has(agent.id)
        if (shouldLink !== isLinked) {
          await api.setAgentLink(skill.name, agent.id, skill.scope, shouldLink)
        }
      }
      await refreshLocal()
      editingSkillKey = ''
      editSelection = []
    } catch (error) {
      localError = String(error)
    } finally {
      linkBusy = false
    }
  }

  const handleViewSkill = (skill) => {
    selectedSkill = skill
    currentView = 'detail'
  }

  const handleBackToList = () => {
    currentView = 'list'
    selectedSkill = null
  }
</script>

<div class="flex h-screen flex-col overflow-hidden bg-[var(--base-100)] text-[var(--base-content)]">
  <PageHeader
    {currentView}
    {activeTab}
    skillName={selectedSkill?.name}
    onChangeView={(view) => (currentView = view)}
    onChangeTab={(tab) => (activeTab = tab)}
    onAddSkill={() => (addSkillModalOpen = true)}
    onBack={handleBackToList}
  />

  <AddSkillModal
    bind:open={addSkillModalOpen}
    {agents}
    onSuccess={refreshLocal}
  />

  <main class="flex-1 overflow-y-auto">
    <div class="mx-auto max-w-6xl px-6 py-6">
      {#if currentView === 'detail' && selectedSkill}
        <SkillDetail
          skill={selectedSkill}
          type={selectedSkill.canonical_path ? 'local' : 'remote'}
          {agents}
        />
      {:else if currentView === 'settings'}
        <SettingsPanel />
      {:else if activeTab === 'local'}
        <LocalSkillsSection
        bind:localSearch
        bind:localAgent
        {agents}
        {localLoading}
        {localError}
        {filteredLocalSkills}
        {managedSkills}
        {unmanagedSkills}
        {editingSkillKey}
        {editSelection}
        {allSelected}
        {agentMap}
        {linkBusy}
        onRefresh={refreshLocal}
        onOpenLinkDialog={openLinkDialog}
        onDeleteSkill={handleDeleteSkill}
        onToggleSelectAll={toggleSelectAll}
        onToggleAgentSelection={toggleAgentSelection}
        onConfirmAgentLinks={confirmAgentLinks}
        onBulkUnify={handleBulkUnify}
        onUnifySkill={handleUnify}
        onViewSkill={handleViewSkill}
      />
    {:else}
      <RemoteSkillsSection
        bind:remoteQuery
        bind:remoteSortBy
        bind:remoteSortOrder
        {localSkills}
        {remoteLoading}
        {remoteSkills}
        {remoteError}
        {installLog}
        {installingSkill}
        {isDownloading}
        {remoteHasMore}
        {remoteTotal}
        onSearch={handleSearchRemote}
        onLoadMore={loadMoreRemote}
        onInstall={handleInstall}
        onViewSkill={handleViewSkill}
        onSortChange={handleSortChange}
      />
    {/if}
    </div>
  </main>

  <InstallConfirmModal
    bind:open={installConfirmModalOpen}
    skillName={pendingInstallSkill?.name || ''}
    {agents}
    onConfirm={handleConfirmInstall}
    onCancel={() => {
      pendingInstallSkill = null
      pendingInstallAgents = []
    }}
  />
</div>
