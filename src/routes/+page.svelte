<script lang="ts">
  import { onMount, tick } from "svelte";
  import { get } from "svelte/store";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import { t } from "../lib/i18n";
  import LocalSkillsSection from "../lib/components/LocalSkillsSection.svelte";
  import RemoteSkillsSection from "../lib/components/RemoteSkillsSection.svelte";
  import AddSkillModal from "../lib/components/AddSkillModal.svelte";
  import UserProjectFormModal from "../lib/components/UserProjectFormModal.svelte";
  import { listUserProjects, type UserProject } from "../lib/api/user-projects";
  import { settings, updateSettings as updateAppSettings } from "../lib/stores/settings";
  import { ensureUpdateChecked, installAvailableUpdate, updaterState } from "../lib/stores/updater";
  import {
    detectGithubAuto,
    checkSkillVersion,
    installFromUnknown,
    installFromGithub,
    recordInstall,
    manageSkillAgentApps,
    deleteSkill,
  } from "../lib/api/skills";
  import {
    agents as agentsStore,
    checkForSkillUpdates,
    loadAgents,
    loadRemote as loadRemoteState,
    localError as localErrorStore,
    localLoading as localLoadingStore,
    localSkills as localSkillsStore,
    refreshLocal as refreshLocalState,
    remoteError as remoteErrorStore,
    remoteHasMore as remoteHasMoreStore,
    remoteLoaded as remoteLoadedStore,
    remoteLoading as remoteLoadingStore,
    remoteSkills as remoteSkillsStore,
    remoteTotal as remoteTotalStore,
    skillsWithUpdate as skillsWithUpdateStore,
    updatingSkills as updatingSkillsStore,
  } from "../lib/stores/skills";
  import type {
    DetectedSkill,
    InstallScope,
    LocalSkill,
    RemoteSkill,
    SourceVersionGroup,
  } from "../lib/api/skills";

  // Shared state for modals
  let addSkillModalOpen = $state(false);
  let userProjectsModalOpen = $state(false);
  let hasUpdate = $state(false);
  let updatingApp = $state(false);
  let mainScrollContainer = $state<HTMLElement | null>(null);
  let initialScrollTop = $state<number | null>(null);

  // Active tab
  let activeTab = $state("local");

  let localSearch = $state("");
  let localAgent = $state("all");
  let localScopeKey = $state("global");
  let userProjects = $state<UserProject[]>([]);
  let userProjectsModalWasOpen = $state(false);

  // Remote skills state
  let remoteQuery = $state("");
  let remoteSkip = $state(0);
  let remoteLimit = $state(20);
  let remoteSortBy = $state("heat_score");
  let remoteSortOrder = $state("desc");

  // Pending install state
  let pendingInstallSkill = $state<(RemoteSkill & { detectedSkill?: DetectedSkill }) | null>(null);
  let isDownloading = $state(false);

  // Select agent modal state
  let selectAgentModalOpen = $state(false);
  let selectAgentModalTitle = $state("");
  let selectAgentModalConfirmText = $state("");
  let selectAgentModalInitialSelection = $state<string[]>([]);
  let selectAgentModalCallback = $state<
    ((
      selectedAgents: string[],
      method: "symlink" | "copy",
      scope: InstallScope,
      projectPath: string | null
    ) => Promise<boolean>) | null
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
    () => new Map($agentsStore.map((agent) => [agent.id, agent.display_name]))
  );

  const filteredLocalSkills = $derived.by(() => {
    const source = Array.isArray($localSkillsStore) ? $localSkillsStore : [];
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

  const localScope = $derived.by(() => (localScopeKey.startsWith("project:") ? "project" : "global"));
  const localProjectPath = $derived.by(() =>
    localScopeKey.startsWith("project:")
      ? decodeURIComponent(localScopeKey.slice("project:".length))
      : null
  );

  // Initialize and load shared data on mount - 只加载本地数据
  onMount(() => {
    let unlistenOpenInstallModal: UnlistenFn | null = null;
    const unsubscribeUpdater = updaterState.subscribe((state) => {
      hasUpdate = state.hasUpdate;
      updatingApp = state.installing;
    });

    const searchParams = get(page).url.searchParams;
    const tabParam = searchParams.get("tab");
    if (tabParam === "remote" || tabParam === "local") {
      activeTab = tabParam;
    }
    const scrollParam = searchParams.get("scroll");
    if (scrollParam) {
      const parsed = Number(scrollParam);
      if (Number.isFinite(parsed) && parsed >= 0) {
        initialScrollTop = parsed;
      }
    }

    // 只加载首屏必需的本地数据
    loadAgents().catch(console.error);
    loadUserProjectOptions().catch(console.error);
    refreshLocal().catch(console.error);

    if (activeTab === "remote" && !get(remoteLoadedStore)) {
      remoteLoadedStore.set(true);
      loadRemote(true).catch(console.error);
    }

    // 如果数据已在 store 中（不触发加载），也要尝试恢复一次滚动位置
    restoreInitialScroll().catch(console.error);

    // Listen for tray menu events
    listen("open-install-modal", () => {
      addSkillModalOpen = true;
    })
      .then((unlisten) => {
        unlistenOpenInstallModal = unlisten;
      })
      .catch(console.error);
    ensureUpdateChecked().catch(console.error);

    return () => {
      unsubscribeUpdater();
      if (unlistenOpenInstallModal) {
        unlistenOpenInstallModal();
      }
    };
  });

  const restoreInitialScroll = async () => {
    if (initialScrollTop === null || !mainScrollContainer) return;
    await tick();
    mainScrollContainer.scrollTop = initialScrollTop;
    initialScrollTop = null;
  };

  const loadUserProjectOptions = async () => {
    userProjects = await listUserProjects();
    if (localScope === "project") {
      const exists = userProjects.some((item) => item.path === localProjectPath);
      if (!exists) {
        localScopeKey = "global";
      }
    }
  };

  $effect(() => {
    if (userProjectsModalOpen) {
      userProjectsModalWasOpen = true;
      return;
    }
    if (userProjectsModalWasOpen) {
      userProjectsModalWasOpen = false;
      loadUserProjectOptions().catch(console.error);
      refreshLocal().catch(console.error);
    }
  });

  const refreshLocal = async () => {
    await refreshLocalState({
      scope: localScope,
      project_path: localProjectPath,
    });
    if (localScope === "global") {
      checkForSkillUpdates().catch(console.error);
    } else {
      skillsWithUpdateStore.set([]);
    }
    if (activeTab === "local") {
      await restoreInitialScroll();
    }
  };

  const loadRemote = async (reset = false) => {
    if (reset) {
      remoteSkip = 0;
    }
    await loadRemoteState({
      reset,
      skip: remoteSkip,
      limit: remoteLimit,
      search: remoteQuery,
      sortBy: remoteSortBy,
      sortOrder: remoteSortOrder,
    });
    if (activeTab === "remote") {
      await restoreInitialScroll();
    }
  };

  const toGitRepoUrl = (url: string) => (url.endsWith(".git") ? url : `${url}.git`);

  const handleUpdateSkill = async (skill: RemoteSkill) => {
    if (get(updatingSkillsStore).includes(skill.name)) return;

    updatingSkillsStore.update((names) => [...names, skill.name]);
    try {
      if (!skill.url) {
        localErrorStore.set(`Skill ${skill.name} has no source URL`);
        return;
      }

      isDownloading = true;
      installingSkill = skill.id;

      const detectedSkill = await detectGithubAuto(skill.url, skill.name);
      pendingInstallSkill = { ...skill, detectedSkill };

      const localSkill = get(localSkillsStore).find((ls) => ls.name === skill.name);
      const currentAgents = localSkill
        ? Array.from(new Set(localSkill.installed_agent_apps.map((app) => app.id)))
        : get(agentsStore).map((a) => a.id);

      selectAgentModalTitle = `${$t("remote.update")} ${skill.name}`;
      selectAgentModalConfirmText = $t("selectAgent.confirm");
      selectAgentModalInitialSelection = currentAgents;
      selectAgentModalCallback = async (selectedAgents, method, scope, projectPath) => {
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
            scope,
            project_path: projectPath,
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
      updatingSkillsStore.update((names) => names.filter((name) => name !== skill.name));
    }
  };

  const handleSearchRemote = async () => {
    await loadRemote(true);
  };

  const loadMoreRemote = async () => {
    if (!get(remoteHasMoreStore)) return;
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

      selectAgentModalTitle = `${$t("remote.install")} ${skill.name}`;
      selectAgentModalConfirmText = $t("selectAgent.confirm");
      selectAgentModalInitialSelection = get(agentsStore).map((a) => a.id);
      selectAgentModalCallback = async (selectedAgents, method, scope, projectPath) => {
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
            scope,
            project_path: projectPath,
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
        localErrorStore.set(String(error));
      });
      return;
    }

    startSourceTypeFlow(skill).catch((error) => {
      localErrorStore.set(String(error));
    });
  };

  const handleLocalUpdateSkill = async (skill: LocalSkill) => {
    const remoteSkill = get(skillsWithUpdateStore).find((item) => item.name === skill.name);
    if (!remoteSkill) {
      localErrorStore.set(`No update metadata found for skill ${skill.name}`);
      return;
    }
    if (!remoteSkill.url) {
      localErrorStore.set(`Skill ${skill.name} has no source URL`);
      return;
    }
    await handleUpdateSkill(remoteSkill);
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
    selectAgentModalConfirmText = $t("remote.update");
    selectAgentModalInitialSelection = getSkillAgentIds(skill);
    selectAgentModalCallback = async (selectedAgents, method, scope, projectPath) => {
      return manageSkillAgentAppsFlow(skill, selectedAgents, method, sourcePath, scope, projectPath);
    };
    selectAgentModalOpen = true;
  };

  const openUnknownSelectAgentModal = (skill: LocalSkill, sourcePath: string) => {
    selectAgentModalTitle = skill.name;
    selectAgentModalConfirmText = $t("remote.update");
    selectAgentModalInitialSelection = getSkillAgentIds(skill);
    selectAgentModalCallback = async (selectedAgents, method, scope, projectPath) => {
      try {
        const result = await installFromUnknown({
          name: skill.name,
          source_path: sourcePath,
          agent_apps: selectedAgents,
          method,
          scope,
          project_path: projectPath,
        });
        if (!result.success) {
          throw new Error(`${result.message}\n${result.stderr || result.stdout}`);
        }
        await refreshLocal();
        return true;
      } catch (error) {
        localErrorStore.set(String(error));
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
    sourcePath: string,
    scope: InstallScope,
    projectPath: string | null
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
          scope,
          project_path: projectPath,
        });
        if (!result.success) {
          throw new Error(`${result.message}\n${result.stderr || result.stdout}`);
        }
        await refreshLocal();
      };

      await executeManage(sourcePath);
      return true;
    } catch (error) {
      localErrorStore.set(String(error));
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
      await deleteSkill(skill.name, localScope, localProjectPath);
      await refreshLocal();
    } catch (error) {
      localErrorStore.set(String(error));
    }
  };

  const getHomeReturnTo = () => {
    const params = new URLSearchParams();
    params.set("tab", activeTab);
    if (mainScrollContainer) {
      params.set("scroll", String(mainScrollContainer.scrollTop));
    }
    return `/?${params.toString()}`;
  };

  const handleViewSkill = (skill: LocalSkill | RemoteSkill) => {
    const type = "source_type" in skill ? "local" : "remote";
    const returnTo = encodeURIComponent(getHomeReturnTo());
    goto(`/skills/${type}/${encodeURIComponent(skill.name)}?returnTo=${returnTo}`);
  };

  // Handle tab change - 延迟加载非关键数据
  const handleTabChange = (tab: string) => {
    activeTab = tab;

    // 切换到远程标签时，首次加载远程数据
    if (tab === "remote" && !get(remoteLoadedStore)) {
      remoteLoadedStore.set(true);
      loadRemote(true).catch(console.error);
    } else {
      restoreInitialScroll().catch(console.error);
    }

    // 切换到本地标签时，延迟检查更新（非阻塞）
    if (tab === "local" && get(localSkillsStore).length > 0) {
      checkForSkillUpdates().catch(console.error);
    }
  };

  $effect(() => {
    const _scope = localScope;
    const _project = localProjectPath;
    refreshLocal().catch(console.error);
  });

  // Navigation handlers
  const handleAppUpdate = async () => {
    if (updatingApp) return;
    try {
      await installAvailableUpdate();
    } catch (error) {
      console.error("Failed to install update:", error);
    }
  };

  const navigateToSettings = () => {
    const returnTo = encodeURIComponent(getHomeReturnTo());
    goto(`/settings?returnTo=${returnTo}`);
  };
</script>

<div class="bg-base-100 text-base-content flex h-screen flex-col overflow-hidden">
  <PageHeader
    currentView="list"
    {activeTab}
    skillName=""
    {hasUpdate}
    updateLoading={updatingApp}
    agentAppsLoading={false}
    onChangeTab={handleTabChange}
    onAddSkill={() => (addSkillModalOpen = true)}
    onOpenUpdate={handleAppUpdate}
    onOpenProjectManage={() => (userProjectsModalOpen = true)}
    onOpenSettings={navigateToSettings}
    onBack={() => {}}
    onDetailAction={undefined}
    onRefreshAgentApps={() => {}}
  />

  <main bind:this={mainScrollContainer} class="flex-1 overflow-y-auto">
    <div class="mx-auto max-w-6xl px-6 py-6">
      {#if activeTab === "local"}
        <LocalSkillsSection
          bind:localSearch
          bind:localAgent
          bind:localScopeKey
          agents={$agentsStore}
          projects={userProjects}
          localLoading={$localLoadingStore}
          localError={$localErrorStore}
          {filteredLocalSkills}
          {agentMap}
          skillsWithUpdate={$skillsWithUpdateStore}
          updatingSkills={$updatingSkillsStore}
          onRefresh={refreshLocal}
          onDeleteSkill={handleDeleteSkill}
          onViewSkill={handleViewSkill}
          onOpenSelectAgentModal={openSelectAgentModal}
          onUpdateSkill={handleLocalUpdateSkill}
        />
      {:else}
        <RemoteSkillsSection
          bind:remoteQuery
          bind:remoteSortBy
          bind:remoteSortOrder
          localSkills={$localSkillsStore}
          remoteLoading={$remoteLoadingStore}
          remoteSkills={$remoteSkillsStore}
          remoteError={$remoteErrorStore}
          {installLog}
          {installingSkill}
          {isDownloading}
          remoteHasMore={$remoteHasMoreStore}
          remoteTotal={$remoteTotalStore}
          skillsWithUpdate={$skillsWithUpdateStore}
          updatingSkills={$updatingSkillsStore}
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

<AddSkillModal bind:open={addSkillModalOpen} agents={$agentsStore} onSuccess={refreshLocal} />
<UserProjectFormModal bind:open={userProjectsModalOpen} />

<!-- Select Agent Modal -->
{#await import("../lib/components/SelectAgentModal.svelte") then { default: SelectAgentModal }}
  <SelectAgentModal
    bind:open={selectAgentModalOpen}
    title={selectAgentModalTitle}
    confirmText={selectAgentModalConfirmText}
    agents={$agentsStore}
    initialSelection={selectAgentModalInitialSelection}
    onConfirm={async (
      selectedAgents: string[],
      method: "symlink" | "copy",
      scope: InstallScope,
      projectPath: string | null
    ) => {
      if (selectAgentModalCallback) {
        return await selectAgentModalCallback(selectedAgents, method, scope, projectPath);
      }
      return true;
    }}
    onCancel={() => {
      selectAgentModalCallback = null;
      selectAgentModalConfirmText = "";
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
