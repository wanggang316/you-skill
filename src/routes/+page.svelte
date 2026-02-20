<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { listen } from "@tauri-apps/api/event";
  import { goto } from "$app/navigation";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import { t } from "../lib/i18n";
  import { check } from "@tauri-apps/plugin-updater";
  import LocalSkillsSection from "../lib/components/LocalSkillsSection.svelte";
  import RemoteSkillsSection from "../lib/components/RemoteSkillsSection.svelte";
  import AddSkillModal from "../lib/components/AddSkillModal.svelte";
  import { settings, updateSettings as updateAppSettings } from "../lib/stores/settings";
  import {
    listSkills,
    fetchRemoteSkills,
    fetchSkillsByNames,
    detectGithubAuto,
    checkSkillVersion,
    installFromUnknown,
    installFromGithub,
    recordInstall,
    manageSkillAgentApps,
    deleteSkill,
    checkSkillUpdate,
  } from "../lib/api/skills";
  import { listLocalAgentApps } from "../lib/api/agent-apps";
  import type {
    DetectedSkill,
    LocalSkill,
    RemoteSkill,
    AgentInfo,
    SourceVersionGroup,
  } from "../lib/api/skills";

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
  let pendingInstallSkill = $state<(RemoteSkill & { detectedSkill?: DetectedSkill }) | null>(null);
  let isDownloading = $state(false);

  // Select agent modal state
  let selectAgentModalOpen = $state(false);
  let selectAgentModalTitle = $state("");
  let selectAgentModalInitialSelection = $state<string[]>([]);
  let selectAgentModalCallback = $state<
    ((selectedAgents: string[], method: "symlink" | "copy") => Promise<boolean>) | null
  >(null);

  // Unknown permission modal state
  let unknownPermissionModalOpen = $state(false);
  let unknownPermissionModalSkillName = $state("");
  let unknownPermissionModalConfirm = $state<((rememberChoice: boolean) => Promise<void>) | null>(
    null
  );

  // Skill version modal state
  let checkSkillVersionModalOpen = $state(false);
  let checkSkillVersionModalTitle = $state("");
  let checkSkillVersionModalSkillName = $state("");
  let checkSkillVersionModalVersionGroups = $state<SourceVersionGroup[]>([]);
  let checkSkillVersionModalConfirm = $state<((sourcePath: string) => Promise<void>) | null>(null);

  // Install state
  let installLog = $state("");
  let installingSkill = $state("");
  let linkBusy = $state(false);

  const agentMap = $derived.by(
    () => new Map(agents.map((agent) => [agent.id, agent.display_name]))
  );

  const filteredLocalSkills = $derived.by(() => {
    const source = Array.isArray(localSkills) ? localSkills : [];
    const needle = localSearch.trim().toLowerCase();
    return source.filter((skill) => {
      const matchesSearch = !needle || skill.name.toLowerCase().includes(needle);
      const agentIds = skill.installed_agent_apps.map((app) => app.id);
      const matchesAgent = localAgent === "all" || agentIds.includes(localAgent);
      return matchesSearch && matchesAgent;
    });
  });

  const getSkillAgentIds = (skill: LocalSkill): string[] =>
    Array.from(new Set(skill.installed_agent_apps.map((app) => app.id)));

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
      localSkills = await listSkills();

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

  const toGitRepoUrl = (url: string) => (url.endsWith(".git") ? url : `${url}.git`);

  const handleUpdateSkill = async (skill: RemoteSkill) => {
    if (updatingSkills.includes(skill.name)) return;

    updatingSkills = [...updatingSkills, skill.name];
    try {
      if (!skill.url) {
        localError = `Skill ${skill.name} has no source URL`;
        return;
      }

      isDownloading = true;
      installingSkill = skill.id;

      const detectedSkill = await detectGithubAuto(skill.url, skill.name);
      pendingInstallSkill = { ...skill, detectedSkill };

      const localSkill = localSkills.find((ls) => ls.name === skill.name);
      const currentAgents = localSkill
        ? Array.from(new Set(localSkill.installed_agent_apps.map((app) => app.id)))
        : agents.map((a) => a.id);

      selectAgentModalTitle = $t("installConfirm.title", { name: skill.name });
      selectAgentModalInitialSelection = currentAgents;
      selectAgentModalCallback = async (selectedAgents, method) => {
        if (!pendingInstallSkill) return false;
        installLog = "";
        installingSkill = pendingInstallSkill.id;
        try {
          if (!pendingInstallSkill.detectedSkill) return false;
          const result = await installFromGithub({
            name: pendingInstallSkill.detectedSkill.name,
            tmp_path: pendingInstallSkill.detectedSkill.tmp_path,
            skill_path: pendingInstallSkill.detectedSkill.skill_path,
            source_url: toGitRepoUrl(skill.url!),
            skill_folder_hash: skill.skill_path_sha ?? null,
            agent_apps: selectedAgents,
            method,
          });
          if (!result.success) {
            installLog = `${result.message}\n${result.stderr || result.stdout}`;
          } else {
            installLog = "";
            await refreshLocal();
            await checkForSkillUpdates();
          }
          return true;
        } catch (error) {
          installLog = String(error);
          return false;
        } finally {
          installingSkill = "";
          pendingInstallSkill = null;
        }
      };
      selectAgentModalOpen = true;
    } catch (error) {
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
    installingSkill = skill.id;
    try {
      if (!skill.url) return;
      const detectedSkill = await detectGithubAuto(skill.url, skill.name);
      pendingInstallSkill = { ...skill, detectedSkill };

      selectAgentModalTitle = $t("installConfirm.title", { name: skill.name });
      selectAgentModalInitialSelection = agents.map((a) => a.id);
      selectAgentModalCallback = async (selectedAgents, method) => {
        if (!pendingInstallSkill) return false;
        installLog = "";
        installingSkill = pendingInstallSkill.id;
        try {
          if (!pendingInstallSkill.detectedSkill) return false;
          const result = await installFromGithub({
            name: pendingInstallSkill.detectedSkill.name,
            tmp_path: pendingInstallSkill.detectedSkill.tmp_path,
            skill_path: pendingInstallSkill.detectedSkill.skill_path,
            source_url: toGitRepoUrl(pendingInstallSkill.url!),
            skill_folder_hash: pendingInstallSkill.skill_path_sha ?? null,
            agent_apps: selectedAgents,
            method,
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
          return true;
        } catch (error) {
          installLog = String(error);
          return false;
        } finally {
          installingSkill = "";
          pendingInstallSkill = null;
        }
      };
      selectAgentModalOpen = true;
    } catch (error) {
      installLog = String(error);
    } finally {
      isDownloading = false;
      installingSkill = "";
    }
  };

  const openSelectAgentModal = (skill: LocalSkill) => {
    if (skill.source_type === "unknown") {
      startUnknownFlow(skill).catch((error) => {
        localError = String(error);
      });
      return;
    }

    startSourceTypeFlow(skill).catch((error) => {
      localError = String(error);
    });
  };

  const openUnknownPermissionModal = (
    skillName: string,
    onConfirm: (rememberChoice: boolean) => Promise<void>
  ) => {
    unknownPermissionModalSkillName = skillName;
    unknownPermissionModalConfirm = onConfirm;
    unknownPermissionModalOpen = true;
  };

  const openCheckSkillVersionModal = (
    title: string,
    skillName: string,
    versionGroups: SourceVersionGroup[],
    onConfirm: (sourcePath: string) => Promise<void>
  ) => {
    checkSkillVersionModalTitle = title;
    checkSkillVersionModalSkillName = skillName;
    checkSkillVersionModalVersionGroups = versionGroups;
    checkSkillVersionModalConfirm = onConfirm;
    checkSkillVersionModalOpen = true;
  };

  const openSourceTypeSelectAgentModal = (skill: LocalSkill, sourcePath: string) => {
    selectAgentModalTitle = skill.name;
    selectAgentModalInitialSelection = getSkillAgentIds(skill);
    selectAgentModalCallback = async (selectedAgents, method) => {
      return manageSkillAgentAppsFlow(skill, selectedAgents, method, sourcePath);
    };
    selectAgentModalOpen = true;
  };

  const openUnknownSelectAgentModal = (skill: LocalSkill, sourcePath: string) => {
    selectAgentModalTitle = skill.name;
    selectAgentModalInitialSelection = getSkillAgentIds(skill);
    selectAgentModalCallback = async (selectedAgents, method) => {
      try {
        const result = await installFromUnknown({
          name: skill.name,
          source_path: sourcePath,
          agent_apps: selectedAgents,
          method,
        });
        if (!result.success) {
          throw new Error(`${result.message}\n${result.stderr || result.stdout}`);
        }
        await refreshLocal();
        return true;
      } catch (error) {
        localError = String(error);
        return false;
      }
    };
    selectAgentModalOpen = true;
  };

  const startUnknownFlow = async (
    skill: LocalSkill,
    skipPermissionPrompt = false
  ): Promise<void> => {
    if (!skipPermissionPrompt && !get(settings).unknown_skill_install_permission) {
      openUnknownPermissionModal(skill.name, async (rememberChoice) => {
        if (rememberChoice) {
          await updateAppSettings({ unknown_skill_install_permission: true });
        }
        await startUnknownFlow(skill, true);
      });
      return;
    }

    const unknownCheck = await checkSkillVersion(
      skill.name,
      skill.global_folder,
      skill.installed_agent_apps.map((item) => item.skill_folder)
    );

    const resolvedSourcePath = unknownCheck.source_path ?? null;
    if (!resolvedSourcePath) {
      if (unknownCheck.version_groups.length > 0) {
        openCheckSkillVersionModal(
          $t("local.unknown.pathSelectTitle", { name: skill.name }),
          skill.name,
          unknownCheck.version_groups,
          async (chosenPath) => {
            openUnknownSelectAgentModal(skill, chosenPath);
          }
        );
        return;
      }
      throw new Error("No source path available for unknown skill");
    }
    openUnknownSelectAgentModal(skill, resolvedSourcePath);
  };

  const startSourceTypeFlow = async (skill: LocalSkill): Promise<void> => {
    const copyCheck = await checkSkillVersion(
      skill.name,
      skill.global_folder,
      skill.installed_agent_apps.map((item) => item.skill_folder)
    );
    const resolvedSourcePath = copyCheck.source_path ?? null;
    if (!resolvedSourcePath && copyCheck.version_groups.length > 0) {
      openCheckSkillVersionModal(
        $t("local.sourceSelect.title", { name: skill.name }),
        skill.name,
        copyCheck.version_groups,
        async (chosenPath) => {
          openSourceTypeSelectAgentModal(skill, chosenPath);
        }
      );
      return;
    }
    if (!resolvedSourcePath) {
      throw new Error("No source path available for this skill");
    }
    openSourceTypeSelectAgentModal(skill, resolvedSourcePath);
  };

  const manageSkillAgentAppsFlow = async (
    skill: LocalSkill,
    selectedAgents: string[],
    method: "symlink" | "copy",
    sourcePath: string
  ): Promise<boolean> => {
    if (!skill || linkBusy) return false;
    linkBusy = true;
    try {
      const executeManage = async (sourcePath: string) => {
        const result = await manageSkillAgentApps({
          name: skill.name,
          source_type: skill.source_type,
          agent_apps: selectedAgents,
          method,
          source_path: sourcePath,
        });
        if (!result.success) {
          throw new Error(`${result.message}\n${result.stderr || result.stdout}`);
        }
        await refreshLocal();
      };

      await executeManage(sourcePath);
      return true;
    } catch (error) {
      localError = String(error);
      return false;
    } finally {
      linkBusy = false;
    }
  };

  const handleDeleteSkill = async (skill: LocalSkill) => {
    const { confirm } = await import("@tauri-apps/plugin-dialog");
    try {
      const confirmed = await confirm($t("confirm.deleteSkill", { name: skill.name }), {
        title: $t("confirm.deleteTitle"),
      });
      if (!confirmed) return;
      await deleteSkill(skill.name);
      await refreshLocal();
    } catch (error) {
      localError = String(error);
    }
  };

  const handleViewSkill = (skill: LocalSkill | RemoteSkill) => {
    const type = "source_type" in skill ? "local" : "remote";
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
    {activeTab}
    skillName=""
    {hasUpdate}
    agentAppsLoading={false}
    onChangeTab={handleTabChange}
    onAddSkill={() => (addSkillModalOpen = true)}
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
          {agentMap}
          {skillsWithUpdate}
          {updatingSkills}
          onRefresh={refreshLocal}
          onDeleteSkill={handleDeleteSkill}
          onViewSkill={handleViewSkill}
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

<AddSkillModal bind:open={addSkillModalOpen} {agents} onSuccess={refreshLocal} />

<!-- Select Agent Modal -->
{#await import("../lib/components/SelectAgentModal.svelte") then { default: SelectAgentModal }}
  <SelectAgentModal
    bind:open={selectAgentModalOpen}
    title={selectAgentModalTitle}
    {agents}
    initialSelection={selectAgentModalInitialSelection}
    onConfirm={async (selectedAgents: string[], method: "symlink" | "copy") => {
      if (selectAgentModalCallback) {
        return await selectAgentModalCallback(selectedAgents, method);
      }
      return true;
    }}
    onCancel={() => {
      selectAgentModalCallback = null;
    }}
  />
{/await}

{#await import("../lib/components/UnknownPermissionModal.svelte") then { default: UnknownPermissionModal }}
  <UnknownPermissionModal
    bind:open={unknownPermissionModalOpen}
    skillName={unknownPermissionModalSkillName}
    onConfirm={async (rememberChoice: boolean) => {
      if (unknownPermissionModalConfirm) {
        await unknownPermissionModalConfirm(rememberChoice);
      }
      unknownPermissionModalConfirm = null;
    }}
    onCancel={() => {
      unknownPermissionModalConfirm = null;
    }}
  />
{/await}

{#await import("../lib/components/CheckSkillVersionModal.svelte") then { default: CheckSkillVersionModal }}
  <CheckSkillVersionModal
    bind:open={checkSkillVersionModalOpen}
    title={checkSkillVersionModalTitle}
    skillName={checkSkillVersionModalSkillName}
    versionGroups={checkSkillVersionModalVersionGroups}
    onConfirm={async (sourcePath: string) => {
      if (checkSkillVersionModalConfirm) {
        await checkSkillVersionModalConfirm(sourcePath);
      }
      checkSkillVersionModalConfirm = null;
    }}
    onCancel={() => {
      checkSkillVersionModalConfirm = null;
    }}
  />
{/await}
