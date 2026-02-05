<script>
  import { onMount } from 'svelte'
  import { confirm } from '@tauri-apps/plugin-dialog'
  import AddSkillModal from '../lib/components/AddSkillModal.svelte'
  import LocalSkillsSection from '../lib/components/LocalSkillsSection.svelte'
  import PageHeader from '../lib/components/PageHeader.svelte'
  import RemoteSkillsSection from '../lib/components/RemoteSkillsSection.svelte'
  import SettingsPanel from '../lib/components/SettingsPanel.svelte'
  import { api } from '../lib/api/skills'
  import { t } from '../lib/i18n'
  import { loadSettings } from '../lib/stores/settings'

  let currentView = $state('list')
  let activeTab = $state('local')
  let addSkillModalOpen = $state(false)

  let localSkills = $state([])
  let localSearch = $state('')
  let localAgent = $state('all')
  let localLoading = $state(false)
  let localError = $state('')

  let remoteSkills = $state([])
  let remoteQuery = $state('')
  let remotePage = $state(1)
  let remotePageSize = $state(50)
  let remoteHasMore = $state(false)
  let remoteLoading = $state(false)
  let remoteError = $state('')

  let agents = $state([])
  let installAgent = $state('cursor')
  let installGlobal = $state(true)
  let installLog = $state('')
  let installingSkill = $state('')
  let linkBusy = $state(false)
  let editingSkillKey = $state('')
  let editSelection = $state([])

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
        remotePage = 1
      }
      const response = await api.fetchRemoteSkills(
        remotePage,
        remotePageSize,
        remoteQuery
      )
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

  const handleSearchRemote = async () => {
    await loadRemote(true)
  }

  const loadMoreRemote = async () => {
    if (!remoteHasMore) return
    remotePage += 1
    await loadRemote(false)
  }

  const handleInstall = async (skill) => {
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
</script>

<div class="min-h-screen bg-[var(--base-100)] text-[var(--base-content)]">
  <PageHeader
    {currentView}
    {activeTab}
    onChangeView={(view) => (currentView = view)}
    onChangeTab={(tab) => (activeTab = tab)}
    onAddSkill={() => (addSkillModalOpen = true)}
  />

  <AddSkillModal
    bind:open={addSkillModalOpen}
    {agents}
    onSuccess={refreshLocal}
  />

  <main class="mx-auto max-w-6xl px-6 py-6">
    {#if currentView === 'settings'}
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
      />
    {:else}
      <RemoteSkillsSection
        bind:remoteQuery
        bind:installAgent
        bind:installGlobal
        {agents}
        {remoteLoading}
        {remoteSkills}
        {remoteError}
        {installLog}
        {installingSkill}
        {remoteHasMore}
        onSearch={handleSearchRemote}
        onLoadMore={loadMoreRemote}
        onInstall={handleInstall}
      />
    {/if}
  </main>
</div>
