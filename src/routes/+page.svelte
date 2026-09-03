<script lang="ts">
  import { tick } from "svelte";
  import { get } from "svelte/store";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { t } from "$lib/i18n";
  import LocalSkillsSection from "$lib/components/LocalSkillsSection.svelte";
  import RemoteSkillsSection from "$lib/components/RemoteSkillsSection.svelte";
  import { getSkillsLocation, projectScopeKey } from "$lib/navigation/app-shell";
  import { settings, updateSettings as updateAppSettings } from "$lib/stores/settings";
  import { userProjects } from "$lib/stores/user-projects";
  import {
    detectGithubAuto,
    checkSkillVersion,
    installFromUnknown,
    installFromGithub,
    recordInstall,
    manageSkillAgentApps,
    deleteSkill,
  } from "$lib/api/skills";
  import {
    agents as agentsStore,
    checkForSkillUpdates,
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
  } from "$lib/stores/skills";
  import type {
    DetectedSkill,
    InstallScope,
    LocalSkill,
    RemoteSkill,
    SourceVersionGroup,
  } from "$lib/api/skills";

  let mainScrollContainer = $state<HTMLElement | null>(null);
  let initialScrollTop = $state<number | null>(null);
  let restoredScrollKey = $state("");

  let localSearch = $state("");
  let localAgent = $state("all");

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
  let selectAgentModalAllowScopeChange = $state(true);
  let selectAgentModalInitialScope = $state<InstallScope>("global");
  let selectAgentModalInitialProjectPath = $state<string | null>(null);
  let selectAgentModalCallback = $state<
    | ((
        selectedAgents: string[],
        method: "symlink" | "copy",
        scope: InstallScope,
        projectPath: string | null
      ) => Promise<boolean>)
    | null
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
  let batchUpdating = $state(false);
  let batchUpdateCompleted = $state(0);
  let batchUpdateTotal = $state(0);

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

  const routeLocation = $derived(getSkillsLocation($page.url));
  const activeTab = $derived(routeLocation.tab);
  const localScopeKey = $derived(
    routeLocation.projectPath ? projectScopeKey(routeLocation.projectPath) : "global"
  );
  const localScope = $derived.by(() =>
    localScopeKey.startsWith("project:") ? "project" : "global"
  );
  const localProjectPath = $derived.by(() =>
    localScopeKey.startsWith("project:")
      ? decodeURIComponent(localScopeKey.slice("project:".length))
      : null
  );
  const selectedProject = $derived.by(() =>
    $userProjects.find((project) => project.path === localProjectPath)
  );
  const contentTitle = $derived.by(() => {
    if (activeTab === "remote") return $t("sidebar.library");
    if (localScope === "global") return $t("sidebar.global");
    return selectedProject?.name ?? $t("header.localTab");
  });
  const updatableLocalSkills = $derived.by(() => {
    if (localScope !== "global") return [];
    const localNames = new Set($localSkillsStore.map((skill) => skill.name));
    return $skillsWithUpdateStore.filter((skill) => localNames.has(skill.name));
  });

  const restoreInitialScroll = async () => {
    if (initialScrollTop === null || !mainScrollContainer) return;
    await tick();
    mainScrollContainer.scrollTop = initialScrollTop;
    initialScrollTop = null;
  };

  $effect(() => {
    const scrollParam = $page.url.searchParams.get("scroll");
    const scrollKey = `${$page.url.pathname}${$page.url.search}`;
    if (scrollParam && restoredScrollKey !== scrollKey) {
      const parsed = Number(scrollParam);
      if (Number.isFinite(parsed) && parsed >= 0) {
        initialScrollTop = parsed;
        restoredScrollKey = scrollKey;
      }
    }

    if (activeTab === "remote" && !get(remoteLoadedStore)) {
      remoteLoadedStore.set(true);
      loadRemote(true).catch(console.error);
    } else {
      restoreInitialScroll().catch(console.error);
    }
  });

  const refreshLocal = async (options?: { awaitUpdateCheck?: boolean }) => {
    await refreshLocalState({
      scope: localScope,
      project_path: localProjectPath,
    });
    if (localScope === "global") {
      if (options?.awaitUpdateCheck) {
        await checkForSkillUpdates();
      } else {
        checkForSkillUpdates().catch(console.error);
      }
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
    const shouldCheckUpdates = localScope === "global";
    await loadRemoteState({
      reset,
      skip: remoteSkip,
      limit: remoteLimit,
      search: remoteQuery,
      sortBy: remoteSortBy,
      sortOrder: remoteSortOrder,
      checkUpdates: shouldCheckUpdates,
    });
    if (activeTab === "remote") {
      await restoreInitialScroll();
    }
  };

  const toGitRepoUrl = (url: string) => (url.endsWith(".git") ? url : `${url}.git`);

  const resolveUpdateContext = (skillName: string) => {
    const localSkill = get(localSkillsStore).find((ls) => ls.name === skillName);
    const selectedAgents = localSkill
      ? Array.from(new Set(localSkill.installed_agent_apps.map((app) => app.id)))
      : get(agentsStore).map((a) => a.id);
    const method =
      localSkill?.installed_agent_apps[0]?.method ?? get(settings).sync_mode ?? "symlink";
    const scope: InstallScope = localScope as InstallScope;
    const projectPath = scope === "project" ? localProjectPath : null;
    return { selectedAgents, method, scope, projectPath };
  };

  const runSkillUpdate = async (
    skill: RemoteSkill,
    options?: { refreshAfterSuccess?: boolean }
  ): Promise<{ success: boolean; error?: string }> => {
    const refreshAfterSuccess = options?.refreshAfterSuccess ?? true;
    if (get(updatingSkillsStore).includes(skill.name)) {
      return { success: false, error: `Skill ${skill.name} is already updating` };
    }

    updatingSkillsStore.update((names) => [...names, skill.name]);
    try {
      if (!skill.url) {
        const error = `Skill ${skill.name} has no source URL`;
        localErrorStore.set(error);
        return { success: false, error };
      }

      isDownloading = true;
      installingSkill = skill.id;

      const detectedSkill = await detectGithubAuto(skill.url, skill.name);
      const updateContext = resolveUpdateContext(skill.name);
      installLog = "";
      const result = await installFromGithub({
        name: detectedSkill.name,
        tmp_path: detectedSkill.tmp_path,
        skill_path: detectedSkill.skill_path,
        source_url: toGitRepoUrl(skill.url),
        skill_folder_hash: skill.skill_path_sha ?? null,
        agent_apps: updateContext.selectedAgents,
        method: updateContext.method,
        scope: updateContext.scope,
        project_path: updateContext.projectPath,
      });
      if (!result.success) {
        const error = `${result.message}\n${result.stderr || result.stdout}`;
        installLog = error;
        return { success: false, error };
      } else {
        installLog = "";
        if (refreshAfterSuccess) {
          await refreshLocal({ awaitUpdateCheck: true });
        }
        return { success: true };
      }
    } catch (error) {
      const message = String(error);
      installLog = message;
      return { success: false, error: message };
    } finally {
      isDownloading = false;
      installingSkill = "";
      updatingSkillsStore.update((names) => names.filter((name) => name !== skill.name));
    }
  };

  const handleUpdateSkill = async (skill: RemoteSkill) => {
    await runSkillUpdate(skill);
  };

  const handleUpdateAllSkills = async () => {
    if (batchUpdating) return;

    const skillsToUpdate = [...updatableLocalSkills];
    if (skillsToUpdate.length === 0) return;

    batchUpdating = true;
    batchUpdateCompleted = 0;
    batchUpdateTotal = skillsToUpdate.length;
    localErrorStore.set("");
    installLog = "";

    const failedSkills: string[] = [];
    const failureLogs: string[] = [];

    try {
      for (const skill of skillsToUpdate) {
        const latestSkill =
          get(skillsWithUpdateStore).find((item) => item.name === skill.name) ?? skill;
        const result = await runSkillUpdate(latestSkill, { refreshAfterSuccess: false });
        if (!result.success) {
          failedSkills.push(skill.name);
          if (result.error) {
            failureLogs.push(`[${skill.name}] ${result.error}`);
          }
        }
        batchUpdateCompleted += 1;
      }

      await refreshLocal({ awaitUpdateCheck: true });

      if (failedSkills.length > 0) {
        localErrorStore.set(
          $t("local.updateAllFailed", {
            count: failedSkills.length,
            names: failedSkills.join(", "),
          })
        );
        installLog = failureLogs.join("\n\n");
      } else {
        localErrorStore.set("");
        installLog = "";
      }
    } catch (error) {
      localErrorStore.set(String(error));
    } finally {
      batchUpdating = false;
      batchUpdateCompleted = 0;
      batchUpdateTotal = 0;
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
      selectAgentModalAllowScopeChange = true;
      selectAgentModalInitialScope = localScope;
      selectAgentModalInitialProjectPath = localScope === "project" ? localProjectPath : null;
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
    selectAgentModalAllowScopeChange = false;
    selectAgentModalInitialScope = localScope;
    selectAgentModalInitialProjectPath = localScope === "project" ? localProjectPath : null;
    selectAgentModalCallback = async (selectedAgents, method, scope, projectPath) => {
      return manageSkillAgentAppsFlow(
        skill,
        selectedAgents,
        method,
        sourcePath,
        scope,
        projectPath
      );
    };
    selectAgentModalOpen = true;
  };

  const openUnknownSelectAgentModal = (skill: LocalSkill, sourcePath: string) => {
    selectAgentModalTitle = skill.name;
    selectAgentModalConfirmText = $t("remote.update");
    selectAgentModalInitialSelection = getSkillAgentIds(skill);
    selectAgentModalAllowScopeChange = false;
    selectAgentModalInitialScope = localScope;
    selectAgentModalInitialProjectPath = localScope === "project" ? localProjectPath : null;
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
      skill.root_folder,
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
      skill.root_folder,
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
    params.set("scope", localScope);
    if (localScope === "project" && localProjectPath) {
      params.set("projectPath", localProjectPath);
    }
    if (mainScrollContainer) {
      params.set("scroll", String(mainScrollContainer.scrollTop));
    }
    return `/?${params.toString()}`;
  };

  const handleViewSkill = (skill: LocalSkill | RemoteSkill) => {
    const type = "source_type" in skill ? "local" : "remote";
    const query = new URLSearchParams();
    query.set("returnTo", getHomeReturnTo());
    if (type === "local") {
      query.set("scope", localScope);
      if (localScope === "project" && localProjectPath) {
        query.set("projectPath", localProjectPath);
      }
    }
    goto(`/skills/${type}/${encodeURIComponent(skill.name)}?${query.toString()}`);
  };

  $effect(() => {
    const _scope = localScope;
    const _project = localProjectPath;
    refreshLocal().catch(console.error);
  });
</script>

<section class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
  <header class="border-base-300 flex min-h-12 items-center border-b px-7" data-window-drag-region>
    <h1 class="text-base-content truncate text-[1.05rem] font-semibold tracking-[-0.02em]">
      {contentTitle}
    </h1>
  </header>

  <main bind:this={mainScrollContainer} class="flex-1 overflow-y-auto">
    <div class="mx-auto max-w-6xl px-7 py-6">
      {#if activeTab === "local"}
        <LocalSkillsSection
          bind:localSearch
          bind:localAgent
          agents={$agentsStore}
          localLoading={$localLoadingStore}
          localError={$localErrorStore}
          {filteredLocalSkills}
          {agentMap}
          skillsWithUpdate={$skillsWithUpdateStore}
          updatingSkills={$updatingSkillsStore}
          updateAllCount={updatableLocalSkills.length}
          {batchUpdating}
          {batchUpdateCompleted}
          {batchUpdateTotal}
          onRefresh={refreshLocal}
          onDeleteSkill={handleDeleteSkill}
          onViewSkill={handleViewSkill}
          onOpenSelectAgentModal={openSelectAgentModal}
          onUpdateSkill={handleLocalUpdateSkill}
          onUpdateAllSkills={handleUpdateAllSkills}
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
</section>

<!-- Select Agent Modal -->
{#await import("$lib/components/SelectAgentModal.svelte") then { default: SelectAgentModal }}
  <SelectAgentModal
    bind:open={selectAgentModalOpen}
    title={selectAgentModalTitle}
    confirmText={selectAgentModalConfirmText}
    agents={$agentsStore}
    initialSelection={selectAgentModalInitialSelection}
    allowScopeChange={selectAgentModalAllowScopeChange}
    initialScope={selectAgentModalInitialScope}
    initialProjectPath={selectAgentModalInitialProjectPath}
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
      selectAgentModalAllowScopeChange = true;
      selectAgentModalInitialScope = "global";
      selectAgentModalInitialProjectPath = null;
    }}
  />
{/await}

{#await import("$lib/components/UnknownPermissionModal.svelte") then { default: UnknownPermissionModal }}
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

{#await import("$lib/components/CheckSkillVersionModal.svelte") then { default: CheckSkillVersionModal }}
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
