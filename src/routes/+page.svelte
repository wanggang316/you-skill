<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { goto } from "$app/navigation";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import { t } from "../lib/i18n";
  import { check } from "@tauri-apps/plugin-updater";
  import LocalSkillsSection from "../lib/components/LocalSkillsSection.svelte";
  import RemoteSkillsSection from "../lib/components/RemoteSkillsSection.svelte";
  import {
    scanLocalSkills,
    fetchRemoteSkills,
    fetchSkillsByNames,
    detectGithubAuto,
    install,
    recordInstall,
    checkCanonicalSkill,
    unifySkill,
    deleteSkillComplete,
    setAgentLink,
    checkSkillUpdate,
  } from "../lib/api/skills";
  import { listLocalAgentApps } from "../lib/api/agent-apps";
  import type { DetectedSkill, LocalSkill, RemoteSkill, AgentInfo } from "../lib/api/skills";

  // Shared state for modals
  let addSkillModalOpen = $state(false);
  let hasUpdate = $state(false);

  // Active tab
  let activeTab = $state("local");

  // Local skills state
  let localSkills = $state<LocalSkill[]>([]);
  let localSearch = $state("");
  let localAgent = $state("all");
  let localLoading = $state(false);
  let localError = $state("");

  // Remote skills state
  let remoteSkills = $state<RemoteSkill[]>([]);
  let remoteQuery = $state("");
  let remoteSkip = $state(0);
  let remoteLimit = $state(20);
  let remoteHasMore = $state(false);
  let remoteLoading = $state(false);
  let remoteError = $state("");
  let remoteTotal = $state(0);
  let remoteSortBy = $state("heat_score");
  let remoteSortOrder = $state("desc");

  // Agents state
  let agents = $state<AgentInfo[]>([]);

  // Update skills state
  let skillsWithUpdate = $state<RemoteSkill[]>([]);
  let updatingSkills = $state<string[]>([]);
  let isCheckingUpdates = $state(false);
  let updateCheckPromise = $state<Promise<void> | null>(null);

  // Pending install state
  let pendingInstallSkill = $state<RemoteSkill & { detectedSkill?: DetectedSkill } | null>(null);
  let pendingInstallAgents = $state<string[]>([]);
  let isDownloading = $state(false);
  let downloadError = $state("");

  // Select agent modal state
  let selectAgentModalOpen = $state(false);
  let selectAgentModalTitle = $state("");
  let selectAgentModalInitialSelection = $state<string[]>([]);
  let selectAgentModalCallback = $state<((selectedAgents: string[]) => Promise<void>) | null>(null);

  // Install state
  let installLog = $state("");
  let installingSkill = $state("");
  let linkBusy = $state(false);

  // Pending import modal state
  let pendingImportModalOpenState = $state(false);

  const agentMap = $derived.by(
    () => new Map(agents.map((agent) => [agent.id, agent.display_name]))
  );

  const filteredLocalSkills = $derived.by(() => {
    const source = Array.isArray(localSkills) ? localSkills : [];
    const needle = localSearch.trim().toLowerCase();
    return source.filter((skill) => {
      const matchesSearch =
        !needle ||
        skill.name.toLowerCase().includes(needle) ||
        (skill.description || "").toLowerCase().includes(needle);
      const matchesAgent = localAgent === "all" || (skill.agents || []).includes(localAgent);
      return matchesSearch && matchesAgent;
    });
  });

  const managedSkills = $derived.by(() =>
    filteredLocalSkills
      .filter((skill) => skill.managed_status === "managed")
      .map((skill) => ({
        ...skill,
        key: `${skill.name}::${skill.scope}::${skill.canonical_path}`,
      }))
      .sort((a, b) => (b.created_at || 0) - (a.created_at || 0))
  );

  const unmanagedSkills = $derived.by(() =>
    filteredLocalSkills
      .filter((skill) => ["unmanaged", "mixed", "unknown"].includes(skill.managed_status))
      .sort((a, b) => (b.created_at || 0) - (a.created_at || 0))
  );

  const unmanagedCount = $derived(unmanagedSkills.length);

  // Track if remote skills have been loaded
  let remoteLoaded = $state(false);

  // Initialize and load shared data on mount - 只加载本地数据
  onMount(() => {
    // 只加载首屏必需的本地数据
    loadAgents().catch(console.error);
    refreshLocal().catch(console.error);

    // Listen for tray menu events
    listen("open-install-modal", () => {
      addSkillModalOpen = true;
    });

    return () => {};
  });

  const checkForUpdate = async () => {
    try {
      const update = await check();
      if (update) {
        hasUpdate = true;
      }
    } catch (error) {
      console.error("Failed to check for update:", error);
    }
  };

  const loadAgents = async () => {
    try {
      agents = await listLocalAgentApps();
    } catch (error) {
      console.error(error);
    }
  };

  const refreshLocal = async () => {
    localLoading = true;
    localError = "";
    try {
      localSkills = await scanLocalSkills();

      // 延迟检查更新，不阻塞首屏
      setTimeout(() => checkForSkillUpdates().catch(console.error), 100);
    } catch (error) {
      localError = String(error);
    } finally {
      localLoading = false;
    }
  };

  const loadRemote = async (reset = false) => {
    remoteLoading = true;
    remoteError = "";
    try {
      if (reset) {
        remoteSkip = 0;
        remoteSkills = [];
      }
      const response = await fetchRemoteSkills({
        skip: remoteSkip,
        limit: remoteLimit,
        search: remoteQuery,
        sortBy: remoteSortBy,
        sortOrder: remoteSortOrder,
      });
      remoteHasMore = response.has_more;
      remoteTotal = response.total;
      if (reset) {
        remoteSkills = response.skills;
      } else {
        remoteSkills = [...remoteSkills, ...response.skills];
      }
      await checkUpdatesFromRemoteList();
    } catch (error) {
      remoteError = String(error);
    } finally {
      remoteLoading = false;
    }
  };

  const checkForSkillUpdates = async () => {
    if (isCheckingUpdates) {
      if (updateCheckPromise) {
        await updateCheckPromise;
      }
      return;
    }

    if (localSkills.length === 0) {
      skillsWithUpdate = [];
      return;
    }

    isCheckingUpdates = true;
    skillsWithUpdate = [];

    const checkPromise = (async () => {
      try {
        const skillNames = localSkills.map((s) => s.name);
        const remoteSkillsMap = await fetchSkillsByNames(skillNames);

        for (const remoteSkill of remoteSkillsMap) {
          if (remoteSkill.skill_path_sha) {
            const hasUpdate = await checkSkillUpdate(remoteSkill.name, remoteSkill.skill_path_sha);
            if (hasUpdate) {
              skillsWithUpdate.push(remoteSkill);
            }
          }
        }
      } catch (error) {
        console.error("Failed to check for skill updates:", error);
      } finally {
        isCheckingUpdates = false;
        updateCheckPromise = null;
      }
    })();

    updateCheckPromise = checkPromise;
    await checkPromise;
  };

  const checkUpdatesFromRemoteList = async () => {
    if (localSkills.length === 0 || remoteSkills.length === 0) {
      return;
    }

    const remoteSkillsMap = new Map(remoteSkills.map((rs) => [rs.name, rs]));

    for (const localSkill of localSkills) {
      const remoteSkill = remoteSkillsMap.get(localSkill.name);
      if (remoteSkill && remoteSkill.skill_path_sha) {
        const hasUpdate = await checkSkillUpdate(localSkill.name, remoteSkill.skill_path_sha);
        if (hasUpdate) {
          if (!skillsWithUpdate.some((s) => s.name === remoteSkill.name)) {
            skillsWithUpdate = [...skillsWithUpdate, remoteSkill];
          }
        } else {
          skillsWithUpdate = skillsWithUpdate.filter((s) => s.name !== remoteSkill.name);
        }
      }
    }
  };

  const handleUpdateSkill = async (skill: RemoteSkill) => {
    if (updatingSkills.includes(skill.name)) return;

    updatingSkills = [...updatingSkills, skill.name];
    try {
      if (!skill.url) {
        localError = `Skill ${skill.name} has no source URL`;
        return;
      }

      isDownloading = true;
      downloadError = "";
      installingSkill = skill.id;

      const detectedSkill = await detectGithubAuto(skill.url);
      pendingInstallSkill = { ...skill, detectedSkill };

      const localSkill = localSkills.find((ls) => ls.name === skill.name);
      const currentAgents = localSkill?.agents || agents.map((a) => a.id);

      selectAgentModalTitle = $t("installConfirm.title", { name: skill.name });
      selectAgentModalInitialSelection = currentAgents;
      selectAgentModalCallback = async (selectedAgents) => {
        if (!pendingInstallSkill) return;
        installLog = "";
        installingSkill = pendingInstallSkill.id;
        pendingInstallAgents = selectedAgents;
        try {
          if (!pendingInstallSkill.detectedSkill) return;
          const result = await install({
            detected_skill: pendingInstallSkill.detectedSkill,
            agent_apps: selectedAgents,
            method: "symlink",
          });
          if (!result.success) {
            installLog = `${result.message}\n${result.stderr || result.stdout}`;
          } else {
            installLog = "";
            await refreshLocal();
            await checkForSkillUpdates();
          }
        } catch (error) {
          installLog = String(error);
        } finally {
          installingSkill = "";
          pendingInstallSkill = null;
          pendingInstallAgents = [];
        }
      };
      selectAgentModalOpen = true;
    } catch (error) {
      downloadError = String(error);
      installLog = String(error);
    } finally {
      isDownloading = false;
      installingSkill = "";
      updatingSkills = updatingSkills.filter((name) => name !== skill.name);
    }
  };

  const handleSearchRemote = async () => {
    await loadRemote(true);
  };

  const loadMoreRemote = async () => {
    if (!remoteHasMore) return;
    remoteSkip += remoteLimit;
    await loadRemote(false);
  };

  const handleSortChange = async (sortBy: string, sortOrder: string) => {
    remoteSortBy = sortBy;
    remoteSortOrder = sortOrder;
    await loadRemote(true);
  };

  const handleInstall = async (skill: RemoteSkill) => {
    isDownloading = true;
    downloadError = "";
    installingSkill = skill.id;
    try {
      if (!skill.url) return;
      const detectedSkill = await detectGithubAuto(skill.url);
      pendingInstallSkill = { ...skill, detectedSkill };

      selectAgentModalTitle = $t("installConfirm.title", { name: skill.name });
      selectAgentModalInitialSelection = agents.map((a) => a.id);
      selectAgentModalCallback = async (selectedAgents) => {
        if (!pendingInstallSkill) return;
        installLog = "";
        installingSkill = pendingInstallSkill.id;
        pendingInstallAgents = selectedAgents;
        try {
          if (!pendingInstallSkill.detectedSkill) return;
          const result = await install({
            detected_skill: pendingInstallSkill.detectedSkill,
            agent_apps: selectedAgents,
            method: "symlink",
          });
          if (!result.success) {
            installLog = `${result.message}\n${result.stderr || result.stdout}`;
          } else {
            installLog = "";
            if (pendingInstallSkill?.id) {
              try {
                await recordInstall(pendingInstallSkill.id);
              } catch (e) {
                console.error("Failed to record install:", e);
              }
            }
            await refreshLocal();
          }
        } catch (error) {
          installLog = String(error);
        } finally {
          installingSkill = "";
          pendingInstallSkill = null;
          pendingInstallAgents = [];
        }
      };
      selectAgentModalOpen = true;
    } catch (error) {
      downloadError = String(error);
      installLog = String(error);
    } finally {
      isDownloading = false;
      installingSkill = "";
    }
  };

  const openSelectAgentModal = (skill: LocalSkill) => {
    selectAgentModalTitle = skill.name;
    selectAgentModalInitialSelection = skill.agents || [];
    selectAgentModalCallback = async (selectedAgents) => {
      if (!skill || linkBusy) return;
      linkBusy = true;
      try {
        // First: unlink all currently linked agents
        for (const agentId of skill.agents || []) {
          await setAgentLink(skill.name, agentId, skill.scope, false);
        }
        // Then: re-link selected agents
        for (const agentId of selectedAgents) {
          await setAgentLink(skill.name, agentId, skill.scope, true);
        }
        await refreshLocal();
      } catch (error) {
        localError = String(error);
      } finally {
        linkBusy = false;
      }
    };
    selectAgentModalOpen = true;
  };

  const handleUnify = async (skill: LocalSkill) => {
    const { confirm } = await import("@tauri-apps/plugin-dialog");
    if (!skill || !skill.agents || skill.agents.length === 0) {
      localError = $t("error.noSkillAgent");
      return;
    }
    const agent = skill.agents[0];
    try {
      const check = await checkCanonicalSkill(skill.name, skill.scope);
      let prefer = "current";
      if (check.exists) {
        const keepCanonical = await confirm($t("confirm.duplicateSkill"), {
          title: $t("confirm.duplicateTitle"),
        });
        prefer = keepCanonical ? "canonical" : "current";
      }
      const result = await unifySkill({
        name: skill.name,
        agent,
        scope: skill.scope,
        current_path: skill.canonical_path,
        prefer: prefer as "canonical" | "current",
      });
      if (!result.success) {
        localError = result.message;
      } else {
        await refreshLocal();
      }
    } catch (error) {
      localError = String(error);
    }
  };

  const handleBulkUnify = async () => {
    const { confirm } = await import("@tauri-apps/plugin-dialog");
    if (!unmanagedSkills.length) return;
    for (const skill of unmanagedSkills) {
      if (!skill || !skill.agents || skill.agents.length === 0) continue;
      try {
        const check = await checkCanonicalSkill(skill.name, skill.scope);
        let prefer = "current";
        if (check.exists) {
          const keepCanonical = await confirm(
            $t("confirm.duplicateSkillWithName", { name: skill.name }),
            { title: $t("confirm.duplicateTitle") }
          );
          prefer = keepCanonical ? "canonical" : "current";
        }
        const result = await unifySkill({
          name: skill.name,
          agent: skill.agents[0],
          scope: skill.scope,
          current_path: skill.canonical_path,
          prefer: prefer as "canonical" | "current",
        });
        if (!result.success) {
          localError = result.message;
        }
      } catch (error) {
        localError = String(error);
      }
    }
    await refreshLocal();
  };

  const handleDeleteSkill = async (skill: LocalSkill) => {
    const { confirm } = await import("@tauri-apps/plugin-dialog");
    try {
      const confirmed = await confirm($t("confirm.deleteSkill", { name: skill.name }), {
        title: $t("confirm.deleteTitle"),
      });
      if (!confirmed) return;
      await deleteSkillComplete(skill.name, skill.canonical_path, skill.scope, skill.agents || []);
      await refreshLocal();
    } catch (error) {
      localError = String(error);
    }
  };

  const handleViewSkill = (skill: LocalSkill | RemoteSkill) => {
    const type = "canonical_path" in skill ? "local" : "remote";
    goto(`/skills/${type}/${encodeURIComponent(skill.name)}`);
  };

  // Handle tab change - 延迟加载非关键数据
  const handleTabChange = (tab: string) => {
    activeTab = tab;

    // 切换到远程标签时，首次加载远程数据
    if (tab === "remote" && !remoteLoaded) {
      remoteLoaded = true;
      loadRemote(true).catch(console.error);
    }

    // 切换到本地标签时，延迟检查更新（非阻塞）
    if (tab === "local" && localSkills.length > 0) {
      setTimeout(() => checkForSkillUpdates().catch(console.error), 50);
    }
  };

  // Navigation handlers
  const navigateToSettings = () => {
    goto("/settings");
  };
</script>

<div class="bg-base-100 text-base-content flex h-screen flex-col overflow-hidden">
  <PageHeader
      currentView="list"
      activeTab={activeTab}
      skillName=""
      {unmanagedCount}
      {hasUpdate}
      agentAppsLoading={false}
      onChangeTab={handleTabChange}
      onAddSkill={() => addSkillModalOpen = true}
      onOpenPendingImport={() => pendingImportModalOpenState = true}
      onOpenUpdate={navigateToSettings}
      onBack={() => {}}
      onDetailAction={undefined}
      onRefreshAgentApps={() => {}}
    />

    <main class="flex-1 overflow-y-auto">
      <div class="mx-auto max-w-6xl px-6 py-6">
        {#if activeTab === "local"}
          <LocalSkillsSection
            bind:localSearch
            bind:localAgent
            {agents}
            {localLoading}
            {localError}
            {filteredLocalSkills}
            {managedSkills}
            {unmanagedSkills}
            {agentMap}
            {skillsWithUpdate}
            {updatingSkills}
            onRefresh={refreshLocal}
            onDeleteSkill={handleDeleteSkill}
            onBulkUnify={handleBulkUnify}
            onUnifySkill={handleUnify}
            onViewSkill={handleViewSkill}
            onOpenPendingImport={() => (pendingImportModalOpenState = true)}
            onOpenSelectAgentModal={openSelectAgentModal}
            onUpdateSkill={handleUpdateSkill}
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
            {skillsWithUpdate}
            {updatingSkills}
            onSearch={handleSearchRemote}
            onLoadMore={loadMoreRemote}
            onInstall={handleInstall}
            onUpdateSkill={handleUpdateSkill}
            onViewSkill={handleViewSkill}
            onSortChange={handleSortChange}
            onRefresh={handleSearchRemote}
          />
        {/if}
      </div>
    </main>
</div>

<!-- Select Agent Modal -->
{#await import("../lib/components/SelectAgentModal.svelte") then { default: SelectAgentModal }}
  <SelectAgentModal
    bind:open={selectAgentModalOpen}
    title={selectAgentModalTitle}
    {agents}
    initialSelection={selectAgentModalInitialSelection}
    onConfirm={async (selectedAgents: string[]) => {
      if (selectAgentModalCallback) {
        await selectAgentModalCallback(selectedAgents);
      }
    }}
    onCancel={() => {
      selectAgentModalCallback = null;
    }}
  />
{/await}

<!-- Pending Import Modal -->
{#await import("../lib/components/PendingImportModal.svelte") then { default: PendingImportModal }}
  <PendingImportModal
    bind:open={pendingImportModalOpenState}
    {unmanagedSkills}
    {agentMap}
    onImport={handleUnify}
    onImportAll={handleBulkUnify}
    onDelete={handleDeleteSkill}
  />
{/await}
