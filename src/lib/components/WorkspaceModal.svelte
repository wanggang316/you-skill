<script lang="ts">
  import { AlertCircle, AlertTriangle, FileText, Folder, Loader2 } from "@lucide/svelte";
  import AgentBadge from "./AgentBadge.svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { groupAgents } from "../agents";
  import { t } from "../i18n";
  import {
    addWorkspace,
    registerProjects,
    scanWorkspace,
    type ProjectCandidate,
  } from "../api/user-projects";
  import { agentsById, refreshHub } from "../stores/hub";
  import { refreshInstructions } from "../stores/instructions";
  import type { AgentInfo } from "../api/skills";

  /** Agent badges shown per candidate before the rest collapse into a counter. */
  const MAX_AGENT_BADGES = 4;

  /** One badge per project skills directory, since agents sharing one count as one. */
  const agentGroups = (ids: string[], agents: Map<string, AgentInfo>) =>
    groupAgents(
      ids.flatMap((id) => agents.get(id) ?? []),
      "project"
    );
  import { closeWorkspaceModal, workspaceModal } from "../stores/modals";
  import {
    forgetProject,
    refreshUserProjects,
    refreshWorkspaces,
    userProjects,
  } from "../stores/user-projects";

  let open = $state(false);
  let name = $state("");
  let path = $state("");
  let candidates = $state<ProjectCandidate[]>([]);
  let selected = $state<string[]>([]);
  /** Missing registered projects picked for removal (rescan only). */
  let staleSelected = $state<string[]>([]);
  let scanning = $state(false);
  let applying = $state(false);
  let error = $state("");
  let scanned = $state(false);

  const modalState = $derived($workspaceModal);
  const isRescan = $derived(modalState.mode === "rescan");
  const fresh = $derived(candidates.filter((item) => !item.registered));
  /** Projects added from this workspace whose folder is gone (deleted or renamed). */
  const stale = $derived(
    isRescan
      ? $userProjects.filter((project) => project.workspacePath === path && project.missing)
      : []
  );
  const canApply = $derived(
    !applying &&
      !scanning &&
      Boolean(path.trim()) &&
      (isRescan ? selected.length > 0 || staleSelected.length > 0 : true)
  );

  $effect(() => {
    const next = $workspaceModal;
    if (next.open && !open) {
      name = next.name;
      path = next.path;
      candidates = [];
      selected = [];
      staleSelected = [];
      error = "";
      scanned = false;
      applying = false;
      if (next.path) void scan(next.path);
    }
    open = next.open;
  });

  const baseName = (value: string) => value.split(/[/\\]/).filter(Boolean).pop() || value;

  const relativePath = (value: string) => {
    const root = path.replace(/[/\\]+$/, "");
    return value.startsWith(root) ? value.slice(root.length).replace(/^[/\\]+/, "") || "." : value;
  };

  async function scan(target: string) {
    scanning = true;
    error = "";
    try {
      const found = await scanWorkspace(target);
      candidates = found;
      selected = found.filter((item) => !item.registered).map((item) => item.path);
      if (isRescan) {
        await refreshUserProjects();
        staleSelected = stale.map((project) => project.path);
      }
      scanned = true;
    } catch (err) {
      error = String(err);
      candidates = [];
      selected = [];
    } finally {
      scanning = false;
    }
  }

  async function handlePickFolder() {
    try {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const result = await openDialog({ multiple: false, directory: true });
      if (typeof result !== "string") return;
      path = result;
      if (!name.trim()) name = baseName(result);
      await scan(result);
    } catch (err) {
      error = String(err);
    }
  }

  function toggle(candidatePath: string) {
    selected = selected.includes(candidatePath)
      ? selected.filter((item) => item !== candidatePath)
      : [...selected, candidatePath];
  }

  function toggleStale(projectPath: string) {
    staleSelected = staleSelected.includes(projectPath)
      ? staleSelected.filter((item) => item !== projectPath)
      : [...staleSelected, projectPath];
  }

  function handleClose() {
    open = false;
    closeWorkspaceModal();
  }

  async function handleApply() {
    if (!canApply) return;
    applying = true;
    error = "";
    try {
      if (!isRescan) {
        await addWorkspace(name.trim() || baseName(path), path.trim());
      }
      const picked = candidates.filter((item) => !item.registered && selected.includes(item.path));
      if (picked.length > 0) {
        await registerProjects(
          picked.map((item) => ({ name: item.name, path: item.path, workspacePath: path.trim() }))
        );
      }
      const failures: string[] = [];
      for (const project of stale.filter((item) => staleSelected.includes(item.path))) {
        failures.push(...(await forgetProject(project.path, project.name)));
      }
      if (failures.length > 0) {
        error = failures.join("\n");
        return;
      }
      await refreshWorkspaces();
      await refreshUserProjects();
      await Promise.all([refreshHub(), refreshInstructions()]);
      handleClose();
    } catch (err) {
      error = String(err);
    } finally {
      applying = false;
    }
  }
</script>

<Modal
  bind:open
  title={isRescan ? $t("workspace.rescanTitle") : $t("workspace.addTitle")}
  onClose={handleClose}
  containerClass="max-w-2xl"
>
  <div class="flex h-[70vh] min-h-0 flex-col">
    <div class="border-base-200 flex-none space-y-3 border-b px-6 py-4">
      <div class="flex items-end gap-2">
        <div class="min-w-0 flex-1">
          <label for="workspace-path" class="text-base-content-muted mb-1.5 block text-xs">
            {$t("workspace.folder")}
          </label>
          <input
            id="workspace-path"
            class="border-base-300 bg-base-200 text-base-content h-9 w-full rounded-xl border px-3 text-[13px] focus:outline-none disabled:opacity-60"
            bind:value={path}
            disabled={isRescan || applying}
            placeholder={$t("workspace.folderPlaceholder")}
          />
        </div>
        {#if !isRescan}
          <button
            class="border-base-300 text-base-content hover:bg-base-200 flex h-9 items-center gap-1.5 rounded-xl border px-3 text-[13px] transition"
            type="button"
            onclick={handlePickFolder}
            disabled={applying}
          >
            <Folder size={15} />
            {$t("workspace.selectFolder")}
          </button>
        {/if}
      </div>
      {#if !isRescan}
        <div>
          <label for="workspace-name" class="text-base-content-muted mb-1.5 block text-xs">
            {$t("workspace.name")}
          </label>
          <input
            id="workspace-name"
            class="border-base-300 bg-base-200 text-base-content h-9 w-full rounded-xl border px-3 text-[13px] focus:outline-none"
            bind:value={name}
            disabled={applying}
            placeholder={$t("workspace.namePlaceholder")}
          />
        </div>
      {/if}
      <p class="text-base-content-faint text-[11px]">{$t("workspace.rule")}</p>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto px-3 py-2">
      {#if scanning}
        <div class="text-base-content-muted flex items-center justify-center gap-2 py-10 text-xs">
          <Loader2 size={16} class="animate-spin" />
          {$t("workspace.scanning")}
        </div>
      {:else if !scanned}
        <p class="text-base-content-faint px-3 py-10 text-center text-xs">
          {$t("workspace.pickHint")}
        </p>
      {:else if candidates.length === 0}
        <p class="text-base-content-muted px-3 py-10 text-center text-xs">
          {$t("workspace.none")}
        </p>
      {:else}
        <p class="text-base-content-subtle px-3 pb-1.5 text-[11px]">
          {$t("workspace.found", { count: fresh.length, total: candidates.length })}
        </p>
        {#each candidates as candidate (candidate.path)}
          {@const checked = selected.includes(candidate.path)}
          {@const groups = agentGroups(candidate.agentIds, $agentsById)}
          <label
            class={`flex items-start gap-2.5 rounded-lg px-2.5 py-2 transition ${
              candidate.registered ? "opacity-55" : "hover:bg-base-200 cursor-pointer"
            } ${checked ? "bg-base-200" : ""}`}
          >
            <input
              class="accent-primary mt-1"
              type="checkbox"
              {checked}
              disabled={candidate.registered || applying}
              onchange={() => toggle(candidate.path)}
            />
            <span class="min-w-0 flex-1">
              <span class="flex min-w-0 items-center gap-1.5">
                <span class="text-base-content truncate text-[13px] font-medium">
                  {candidate.name}
                </span>
                {#if candidate.registered}
                  <span class="tag tag-neutral shrink-0">{$t("workspace.registered")}</span>
                {/if}
              </span>
              <span
                class="text-base-content-faint block truncate text-[11px]"
                title={candidate.path}
              >
                {relativePath(candidate.path)}
              </span>
            </span>
            <span class="flex shrink-0 items-center gap-1.5">
              {#if candidate.skillCount > 0}
                <span class="text-base-content-faint text-[11px]">
                  {$t("workspace.skillCount", { count: candidate.skillCount })}
                </span>
              {/if}
              {#each candidate.profiles as profile (profile)}
                <span
                  class="text-base-content-subtle inline-flex items-center gap-0.5 text-[11px]"
                  title={profile}
                >
                  <FileText size={11} />
                  {profile.replace(/\.md$/i, "")}
                </span>
              {/each}
              {#each groups.slice(0, MAX_AGENT_BADGES) as group (group.key)}
                <AgentBadge agentIds={group.agents.map((agent) => agent.id)} agents={$agentsById} />
              {/each}
              {#if groups.length > MAX_AGENT_BADGES}
                <span
                  class="text-base-content-subtle text-[11px]"
                  title={groups
                    .slice(MAX_AGENT_BADGES)
                    .map((group) => group.agents[0].display_name)
                    .join(", ")}
                >
                  +{groups.length - MAX_AGENT_BADGES}
                </span>
              {/if}
            </span>
          </label>
        {/each}
      {/if}

      {#if scanned && stale.length > 0}
        <div class="border-base-200 mt-3 border-t pt-2">
          <p class="text-base-content-subtle px-3 pb-0.5 text-[11px] font-medium">
            {$t("workspace.missingProjects")}
          </p>
          <p class="text-base-content-faint px-3 pb-1.5 text-[11px]">
            {$t("workspace.missingHint")}
          </p>
          {#each stale as project (project.path)}
            {@const checked = staleSelected.includes(project.path)}
            <label
              class={`hover:bg-base-200 flex cursor-pointer items-start gap-2.5 rounded-lg px-2.5 py-2 transition ${
                checked ? "bg-base-200" : ""
              }`}
            >
              <input
                class="accent-primary mt-1"
                type="checkbox"
                {checked}
                disabled={applying}
                onchange={() => toggleStale(project.path)}
              />
              <span class="min-w-0 flex-1">
                <span class="flex min-w-0 items-center gap-1.5">
                  <span class="text-base-content truncate text-[13px] font-medium">
                    {project.name}
                  </span>
                  <span class="text-error shrink-0" title={$t("projects.missing")}>
                    <AlertTriangle size={12} />
                  </span>
                </span>
                <span
                  class="text-base-content-faint block truncate text-[11px]"
                  title={project.path}
                >
                  {relativePath(project.path)}
                </span>
              </span>
            </label>
          {/each}
        </div>
      {/if}

      {#if error}
        <div class="text-error flex items-start gap-2 px-3 py-2 text-sm whitespace-pre-wrap">
          <AlertCircle size={16} class="mt-0.5 shrink-0" />
          <span>{error}</span>
        </div>
      {/if}
    </div>
  </div>
  {#snippet footer()}
    <span class="text-base-content-muted mr-auto text-xs">
      {$t("picker.selected", { count: selected.length })}
    </span>
    <button
      class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
      type="button"
      onclick={handleClose}
      disabled={applying}
    >
      {$t("common.cancel")}
    </button>
    <PrimaryActionButton
      onclick={handleApply}
      disabled={!canApply}
      loading={applying}
      loadingText={$t("install.applying")}
    >
      {#if !isRescan}
        {$t("workspace.addConfirm")}
      {:else if selected.length > 0}
        {$t("workspace.registerConfirm")}
      {:else}
        {$t("install.confirm")}
      {/if}
    </PrimaryActionButton>
  {/snippet}
</Modal>
