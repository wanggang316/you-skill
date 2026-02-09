<script lang="ts">
  import { onMount } from "svelte";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { open } from "@tauri-apps/plugin-shell";
  import { listen } from "@tauri-apps/api/event";
  import AddSkillModal from "../lib/components/AddSkillModal.svelte";
  import SelectAgentModal from "../lib/components/SelectAgentModal.svelte";
  import PendingImportModal from "../lib/components/PendingImportModal.svelte";
  import LocalSkillsSection from "../lib/components/LocalSkillsSection.svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import RemoteSkillsSection from "../lib/components/RemoteSkillsSection.svelte";
  import SettingsPanel from "../lib/components/SettingsPanel.svelte";
  import SkillDetail from "../lib/components/SkillDetail.svelte";
  import {
    scanLocalSkills,
    fetchRemoteSkills,
    listAgents,
    detectGithubSkills,
    installGithubSkill,
    recordInstall,
    checkCanonicalSkill,
    unifySkill,
    deleteSkill,
    setAgentLink,
    updateTraySkills,
  } from "../lib/api/skills";
  import { t } from "../lib/i18n";
  import { loadSettings } from "../lib/stores/settings";
  import { check } from "@tauri-apps/plugin-updater";

  let currentView = $state("list");
  let activeTab = $state("local");
  let selectedSkill = $state(null);
  let addSkillModalOpen = $state(false);

  let localSkills = $state([]);
  let localSearch = $state("");
  let localAgent = $state("all");
  let localLoading = $state(false);
  let localError = $state("");

  let remoteSkills = $state([]);
  let remoteQuery = $state("");
  let remoteSkip = $state(0);
  let remoteLimit = $state(20);
  let remoteHasMore = $state(false);
  let remoteLoading = $state(false);
  let remoteError = $state("");
  let remoteTotal = $state(0);
  let remoteSortBy = $state("heat_score");
  let remoteSortOrder = $state("desc");

  let agents = $state([]);
  let installAgent = $state("cursor");
  let installGlobal = $state(true);
  let installLog = $state("");
  let installingSkill = $state("");
  let linkBusy = $state(false);

  // Pending import modal state
  let pendingImportModalOpen = $state(false);

  // Update state
  let hasUpdate = $state(false);
  let latestVersion = $state("");

  // Pending install state for remote skills
  let pendingInstallSkill = $state(null);
  let pendingInstallAgents = $state([]);
  let isDownloading = $state(false);
  let downloadError = $state("");

  // Select agent modal state (shared for remote and local skills)
  let selectAgentModalOpen = $state(false);
  let selectAgentModalTitle = $state("");
  let selectAgentModalInitialSelection = $state([]);
  let selectAgentModalCallback = $state(null);

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

  onMount(() => {
    let unlisten: (() => void) | undefined;

    const init = async () => {
      await loadSettings();
      await loadAgents();
      await refreshLocal();
      await loadRemote(true);
      await checkForUpdate();

      // Listen for tray menu events
      unlisten = await listen("open-install-modal", () => {
        activeTab = "remote";
        addSkillModalOpen = true;
      });
    };

    init();

    return () => {
      unlisten?.();
    };
  });

  const checkForUpdate = async () => {
    try {
      const update = await check();
      if (update) {
        hasUpdate = true;
        latestVersion = update.version;
      }
    } catch (error) {
      console.error("Failed to check for update:", error);
    }
  };

  const handleOpenUpdate = () => {
    currentView = "settings";
  };

  const loadAgents = async () => {
    try {
      agents = await listAgents();
      if (agents.length > 0) {
        installAgent = agents[0].id;
      }
    } catch (error) {
      console.error(error);
    }
  };

  const refreshLocal = async () => {
    localLoading = true;
    localError = "";
    try {
      localSkills = await scanLocalSkills();
      // Sync skills to tray menu
      try {
        await updateTraySkills(localSkills);
      } catch (e) {
        console.error("Failed to update tray skills:", e);
      }
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
        sort_by: remoteSortBy,
        sort_order: remoteSortOrder,
      });
      remoteHasMore = response.has_more;
      remoteTotal = response.total;
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

  const handleInstall = async (skill) => {
    // First download/detect the skill
    isDownloading = true;
    downloadError = "";
    installingSkill = skill.id;
    try {
      // Detect skills from the GitHub URL
      const detectedSkills = await detectGithubSkills(skill.url);
      // Find the matching skill by name or path
      const matchingSkill = detectedSkills.find(
        (s) => s.name === skill.name || skill.path?.includes(s.name) || s.path === skill.path
      );
      if (!matchingSkill && detectedSkills.length > 0) {
        // Use first detected skill if no exact match
        pendingInstallSkill = { ...skill, detectedPath: detectedSkills[0].path };
      } else if (matchingSkill) {
        pendingInstallSkill = { ...skill, detectedPath: matchingSkill.path };
      } else {
        downloadError = "No skills found in repository";
        return;
      }
      // Open select agent modal after successful download
      selectAgentModalTitle = $t("installConfirm.title", { name: skill.name });
      selectAgentModalInitialSelection = agents.map((a) => a.id);
      selectAgentModalCallback = async (selectedAgents) => {
        if (!pendingInstallSkill) return;
        installLog = "";
        installingSkill = pendingInstallSkill.id;
        pendingInstallAgents = selectedAgents;
        try {
          // Use installGithubSkill API for remote skills
          const skillPath =
            pendingInstallSkill.detectedPath ||
            pendingInstallSkill.path ||
            pendingInstallSkill.name;
          const result = await installGithubSkill({
            url: pendingInstallSkill.url,
            skill_path: skillPath,
            agents: selectedAgents,
          });
          if (!result.success) {
            installLog = `${result.message}\n${result.stderr || result.stdout}`;
          } else {
            installLog = "";
            // Record install count on backend
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

  // Open SelectAgentModal for local skills
  const openSelectAgentModal = (skill) => {
    selectAgentModalTitle = skill.name;
    selectAgentModalInitialSelection = skill.agents || [];
    selectAgentModalCallback = async (selectedAgents) => {
      if (!skill || linkBusy) return;
      linkBusy = true;
      try {
        const currentSet = new Set(skill.agents || []);
        const targetSet = new Set(selectedAgents);
        for (const agent of agents) {
          const shouldLink = targetSet.has(agent.id);
          const isLinked = currentSet.has(agent.id);
          if (shouldLink !== isLinked) {
            await setAgentLink(skill.name, agent.id, skill.scope, shouldLink);
          }
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

  const handleOpenUrl = async (url) => {
    try {
      await open(url);
    } catch (error) {
      console.error("Failed to open URL:", error);
    }
  };

  const handleUnify = async (skill) => {
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
  };

  const handleBulkUnify = async () => {
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
  };

  const handleDeleteSkill = async (skill) => {
    try {
      const confirmed = await confirm($t("confirm.deleteSkill", { name: skill.name }), {
        title: $t("confirm.deleteTitle"),
      });
      if (!confirmed) return;
      await deleteSkill(skill.canonical_path);
      await refreshLocal();
    } catch (error) {
      localError = String(error);
    }
  };

  const handleViewSkill = (skill) => {
    selectedSkill = skill;
    currentView = "detail";
  };

  const handleBackToList = () => {
    currentView = "list";
    selectedSkill = null;
  };

  // Build GitHub web URL for a specific path
  function buildGitHubUrl(url, path) {
    if (!url) return null;

    // Handle github.com URLs
    if (url.includes("github.com")) {
      // Extract owner and repo from URL like https://github.com/owner/repo
      const match = url.match(/github\.com\/([^\/]+)\/([^\/]+)/);
      if (match) {
        const [, owner, repo] = match;
        // Remove .git suffix if present
        const cleanRepo = repo.replace(/\.git$/, "");
        // Construct web URL: https://github.com/owner/repo/tree/main/path
        return `https://github.com/${owner}/${cleanRepo}/tree/main/${path || ""}`;
      }
    }
    return null;
  }

  const handleDetailAction = async () => {
    if (!selectedSkill) return;

    // For remote skills, open GitHub URL with path
    if (selectedSkill.url) {
      try {
        const fullUrl =
          buildGitHubUrl(selectedSkill.url, selectedSkill.path || "") || selectedSkill.url;
        await open(fullUrl);
      } catch (error) {
        console.error("Failed to open URL:", error);
      }
    } else if (selectedSkill.canonical_path) {
      // For local skills, open in file manager
      const { openInFileManager } = await import("../lib/api/skills");
      try {
        await openInFileManager(selectedSkill.canonical_path);
      } catch (error) {
        console.error("Failed to open in file manager:", error);
      }
    }
  };
</script>

<div class="bg-base-100 text-base-content flex h-screen flex-col overflow-hidden">
  <PageHeader
    {currentView}
    {activeTab}
    skillName={selectedSkill?.name}
    {unmanagedCount}
    {hasUpdate}
    onChangeView={(view) => (currentView = view)}
    onChangeTab={(tab) => (activeTab = tab)}
    onAddSkill={() => (addSkillModalOpen = true)}
    onOpenPendingImport={() => (pendingImportModalOpen = true)}
    onOpenUpdate={handleOpenUpdate}
    onBack={handleBackToList}
    onDetailAction={selectedSkill ? handleDetailAction : null}
  />

  <AddSkillModal bind:open={addSkillModalOpen} {agents} onSuccess={refreshLocal} />

  <main class="flex-1 overflow-y-auto">
    <div class="mx-auto max-w-6xl px-6 py-6">
      {#if currentView === "detail" && selectedSkill}
        <SkillDetail
          skill={selectedSkill}
          type={selectedSkill.canonical_path ? "local" : "remote"}
          {agents}
        />
      {:else if currentView === "settings"}
        <SettingsPanel />
      {:else if activeTab === "local"}
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
          onRefresh={refreshLocal}
          onDeleteSkill={handleDeleteSkill}
          onBulkUnify={handleBulkUnify}
          onUnifySkill={handleUnify}
          onViewSkill={handleViewSkill}
          onOpenPendingImport={() => (pendingImportModalOpen = true)}
          onOpenSelectAgentModal={openSelectAgentModal}
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
          onRefresh={handleSearchRemote}
        />
      {/if}
    </div>
  </main>

  <SelectAgentModal
    bind:open={selectAgentModalOpen}
    title={selectAgentModalTitle}
    {agents}
    initialSelection={selectAgentModalInitialSelection}
    onConfirm={async (selectedAgents) => {
      if (selectAgentModalCallback) {
        await selectAgentModalCallback(selectedAgents);
      }
    }}
    onCancel={() => {
      selectAgentModalCallback = null;
    }}
  />

  <PendingImportModal
    bind:open={pendingImportModalOpen}
    {unmanagedSkills}
    {agentMap}
    onImport={handleUnify}
    onImportAll={handleBulkUnify}
    onDelete={handleDeleteSkill}
  />
</div>
