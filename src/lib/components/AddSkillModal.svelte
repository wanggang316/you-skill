<script>
  import {
    FileArchive,
    Github,
    Check,
    AlertCircle,
    Loader2,
    Folder,
  } from "@lucide/svelte";
  import { t } from "../i18n";
  import {
    detectZipSkills,
    detectGithubSkills,
    installZipSkill,
    installGithubSkill,
    detectFolderSkills,
    installFolderSkill,
  } from "../api/skills";
  import Modal from "./ui/Modal.svelte";

  let { open = $bindable(false), agents = [], onSuccess = () => {} } = $props();

  // Tab state: 'zip' | 'folder' | 'github'
  let activeTab = $state("zip");

  // ZIP file state
  let selectedZipPath = $state("");
  let zipFileName = $state("");
  let isDetectingZip = $state(false);
  let detectedZipSkills = $state([]);
  let selectedZipSkill = $state(null);
  let zipError = $state("");

  // Folder state
  let selectedFolderPath = $state("");
  let folderName = $state("");
  let isDetectingFolder = $state(false);
  let detectedFolderSkills = $state([]);
  let selectedFolderSkill = $state(null);
  let folderError = $state("");

  // Github state
  let githubUrl = $state("");
  let isDetecting = $state(false);
  let detectedSkills = $state([]);
  let selectedSkill = $state(null);
  let githubError = $state("");

  // Agent selection state
  let selectedAgents = $state([]);

  // Global loading state
  let isInstalling = $state(false);
  let installError = $state("");

  // Reset state when modal opens
  $effect(() => {
    if (open) {
      resetState();
    }
  });

  function resetState() {
    activeTab = "zip";
    selectedZipPath = "";
    zipFileName = "";
    isDetectingZip = false;
    detectedZipSkills = [];
    selectedZipSkill = null;
    zipError = "";
    selectedFolderPath = "";
    folderName = "";
    isDetectingFolder = false;
    detectedFolderSkills = [];
    selectedFolderSkill = null;
    folderError = "";
    githubUrl = "";
    isDetecting = false;
    detectedSkills = [];
    selectedSkill = null;
    githubError = "";
    selectedAgents = agents.map((a) => a.id);
    isInstalling = false;
    installError = "";
  }

  function closeModal() {
    open = false;
  }

  async function handleSelectZipFile() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "Skill Files", extensions: ["skill", "zip"] }],
      });
      if (result) {
        selectedZipPath = result;
        zipFileName = result.split(/[/\\]/).pop() || "";
      }
    } catch (error) {
      console.error("Failed to select zip file:", error);
    }
  }

  async function handleDetectZip() {
    if (!selectedZipPath) return;

    isDetectingZip = true;
    zipError = "";
    detectedZipSkills = [];
    selectedZipSkill = null;

    try {
      const skills = await detectZipSkills(selectedZipPath);
      detectedZipSkills = skills;
      if (skills.length === 0) {
        zipError = $t("addSkill.noSkillsFound");
      } else if (skills.length === 1) {
        selectedZipSkill = skills[0];
      }
    } catch (error) {
      zipError = String(error);
    } finally {
      isDetectingZip = false;
    }
  }

  async function handleSelectFolder() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({
        multiple: false,
        directory: true,
      });
      if (result) {
        selectedFolderPath = result;
        folderName = result.split(/[/\\]/).pop() || "";
      }
    } catch (error) {
      console.error("Failed to select folder:", error);
    }
  }

  async function handleDetectFolder() {
    if (!selectedFolderPath) return;

    isDetectingFolder = true;
    folderError = "";
    detectedFolderSkills = [];
    selectedFolderSkill = null;

    try {
      const skills = await detectFolderSkills(selectedFolderPath);
      detectedFolderSkills = skills;
      if (skills.length === 0) {
        folderError = $t("addSkill.noSkillsFound");
      } else if (skills.length === 1) {
        selectedFolderSkill = skills[0];
      }
    } catch (error) {
      folderError = String(error);
    } finally {
      isDetectingFolder = false;
    }
  }

  async function handleDetectGithub() {
    if (!githubUrl.trim()) return;

    isDetecting = true;
    githubError = "";
    detectedSkills = [];
    selectedSkill = null;

    try {
      const skills = await detectGithubSkills(githubUrl.trim());
      detectedSkills = skills;
      if (skills.length === 0) {
        githubError = $t("addSkill.noSkillsFound");
      } else if (skills.length === 1) {
        selectedSkill = skills[0];
      }
    } catch (error) {
      githubError = String(error);
    } finally {
      isDetecting = false;
    }
  }

  function toggleAgent(agentId) {
    if (selectedAgents.includes(agentId)) {
      selectedAgents = selectedAgents.filter((id) => id !== agentId);
    } else {
      selectedAgents = [...selectedAgents, agentId];
    }
  }

  function selectAllAgents() {
    selectedAgents = agents.map((a) => a.id);
  }

  function deselectAllAgents() {
    selectedAgents = [];
  }

  async function handleConfirm() {
    if (selectedAgents.length === 0) {
      installError = $t("addSkill.noAgentsSelected");
      return;
    }

    isInstalling = true;
    installError = "";

    try {
      if (activeTab === "zip") {
        if (!selectedZipPath) {
          installError = $t("addSkill.noZipSelected");
          isInstalling = false;
          return;
        }
        if (!selectedZipSkill) {
          installError = $t("addSkill.noSkillSelected");
          isInstalling = false;
          return;
        }
        await installZipSkill({
          zip_path: selectedZipPath,
          skill_path: selectedZipSkill.path,
          agents: selectedAgents,
        });
      } else if (activeTab === "folder") {
        if (!selectedFolderPath) {
          installError = $t("addSkill.noFolderSelected");
          isInstalling = false;
          return;
        }
        if (!selectedFolderSkill) {
          installError = $t("addSkill.noSkillSelected");
          isInstalling = false;
          return;
        }
        await installFolderSkill({
          folder_path: selectedFolderPath,
          skill_path: selectedFolderSkill.path,
          agents: selectedAgents,
        });
      } else {
        if (!selectedSkill) {
          installError = $t("addSkill.noSkillSelected");
          return;
        }
        await installGithubSkill({
          url: githubUrl,
          skill_path: selectedSkill.path,
          agents: selectedAgents,
        });
      }

      onSuccess();
      closeModal();
    } catch (error) {
      installError = String(error);
    } finally {
      isInstalling = false;
    }
  }

  function canConfirm() {
    if (selectedAgents.length === 0) return false;
    if (activeTab === "zip") {
      return !!selectedZipPath && !!selectedZipSkill;
    } else if (activeTab === "folder") {
      return !!selectedFolderPath && !!selectedFolderSkill;
    } else {
      return !!selectedSkill;
    }
  }

  const allSelected = $derived(selectedAgents.length === agents.length);
</script>

<Modal bind:open title={$t("addSkill.title")} onClose={closeModal}>
  <div class="flex h-full max-h-[90vh] w-full max-w-lg flex-col">
    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6 pt-16">
      <!-- Tabs -->
      <div class="mb-6 flex gap-2 rounded-full bg-[var(--base-200)] p-1">
        <button
          class={`flex flex-1 items-center justify-center gap-2 rounded-full px-3 py-2 text-sm transition ${activeTab === "zip" ? "bg-[var(--base-100)] text-[var(--base-content)] shadow-sm" : "text-[var(--base-content-muted)] hover:text-[var(--base-content)]"}`}
          onclick={() => (activeTab = "zip")}
          type="button"
        >
          <FileArchive size={16} />
          {$t("addSkill.tab.zip")}
        </button>
        <button
          class={`flex flex-1 items-center justify-center gap-2 rounded-full px-3 py-2 text-sm transition ${activeTab === "folder" ? "bg-[var(--base-100)] text-[var(--base-content)] shadow-sm" : "text-[var(--base-content-muted)] hover:text-[var(--base-content)]"}`}
          onclick={() => (activeTab = "folder")}
          type="button"
        >
          <Folder size={16} />
          {$t("addSkill.tab.folder")}
        </button>
        <button
          class={`flex flex-1 items-center justify-center gap-2 rounded-full px-3 py-2 text-sm transition ${activeTab === "github" ? "bg-[var(--base-100)] text-[var(--base-content)] shadow-sm" : "text-[var(--base-content-muted)] hover:text-[var(--base-content)]"}`}
          onclick={() => (activeTab = "github")}
          type="button"
        >
          <Github size={16} />
          {$t("addSkill.tab.github")}
        </button>
      </div>

      <!-- ZIP Mode -->
      {#if activeTab === "zip"}
        <div class="space-y-4">
          <p class="text-sm text-[var(--base-content-muted)]">
            {$t("addSkill.zip.description")}
          </p>
          <button
            class="w-full rounded-xl border-2 border-dashed border-[var(--base-300)] p-8 transition hover:border-[var(--primary)] hover:bg-[var(--base-200)]"
            onclick={handleSelectZipFile}
            type="button"
          >
            {#if selectedZipPath}
              <div
                class="flex items-center justify-center gap-2 text-[var(--base-content)]"
              >
                <FileArchive size={20} class="text-[var(--primary)]" />
                <span class="font-medium">{zipFileName}</span>
              </div>
              <p class="mt-2 text-xs text-[var(--base-content-muted)]">
                {$t("addSkill.zip.clickToChange")}
              </p>
            {:else}
              <div
                class="flex flex-col items-center gap-2 text-[var(--base-content-muted)]"
              >
                <FileArchive size={32} />
                <span>{$t("addSkill.zip.selectFile")}</span>
              </div>
            {/if}
          </button>

          {#if selectedZipPath}
            <div class="flex gap-2">
              <button
                class="flex-1 rounded-xl bg-[var(--primary)] px-4 py-2 text-sm text-[var(--primary-content)] transition hover:bg-[var(--primary-hover)] disabled:opacity-50"
                onclick={handleDetectZip}
                disabled={isDetectingZip}
                type="button"
              >
                {#if isDetectingZip}
                  <Loader2 size={16} class="animate-spin inline mr-1" />
                {/if}
                {$t("addSkill.zip.detect")}
              </button>
            </div>
          {/if}

          {#if zipError}
            <div class="flex items-center gap-2 text-sm text-[var(--error)]">
              <AlertCircle size={16} />
              <span>{zipError}</span>
            </div>
          {/if}

          {#if detectedZipSkills.length > 0}
            <div class="space-y-2">
              <p class="text-sm font-medium text-[var(--base-content)]">
                {$t("addSkill.zip.selectSkill")}
              </p>
              <div
                class="max-h-48 space-y-2 overflow-y-auto rounded-xl border border-[var(--base-300)] bg-[var(--base-200)] p-2"
              >
                {#each detectedZipSkills as skill}
                  <button
                    class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${selectedZipSkill?.path === skill.path ? "bg-[var(--primary)] text-[var(--primary-content)]" : "bg-[var(--base-100)] text-[var(--base-content)] hover:bg-[var(--base-300)]"}`}
                    onclick={() => (selectedZipSkill = skill)}
                    type="button"
                  >
                    <div>
                      <p class="font-medium">{skill.name}</p>
                      <p
                        class={`text-xs ${selectedZipSkill?.path === skill.path ? "text-[var(--primary-content)] opacity-80" : "text-[var(--base-content-muted)]"}`}
                      >
                        {skill.path}
                      </p>
                    </div>
                    {#if selectedZipSkill?.path === skill.path}
                      <Check size={16} />
                    {/if}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {:else if activeTab === "folder"}
        <!-- Folder Mode -->
        <div class="space-y-4">
          <p class="text-sm text-[var(--base-content-muted)]">
            {$t("addSkill.folder.description")}
          </p>
          <button
            class="w-full rounded-xl border-2 border-dashed border-[var(--base-300)] p-8 transition hover:border-[var(--primary)] hover:bg-[var(--base-200)]"
            onclick={handleSelectFolder}
            type="button"
          >
            {#if selectedFolderPath}
              <div
                class="flex items-center justify-center gap-2 text-[var(--base-content)]"
              >
                <Folder size={20} class="text-[var(--primary)]" />
                <span class="font-medium">{folderName}</span>
              </div>
              <p class="mt-2 text-xs text-[var(--base-content-muted)]">
                {$t("addSkill.folder.clickToChange")}
              </p>
            {:else}
              <div
                class="flex flex-col items-center gap-2 text-[var(--base-content-muted)]"
              >
                <Folder size={32} />
                <span>{$t("addSkill.folder.selectFolder")}</span>
              </div>
            {/if}
          </button>

          {#if selectedFolderPath}
            <div class="flex gap-2">
              <button
                class="flex-1 rounded-xl bg-[var(--primary)] px-4 py-2 text-sm text-[var(--primary-content)] transition hover:bg-[var(--primary-hover)] disabled:opacity-50"
                onclick={handleDetectFolder}
                disabled={isDetectingFolder}
                type="button"
              >
                {#if isDetectingFolder}
                  <Loader2 size={16} class="animate-spin inline mr-1" />
                {/if}
                {$t("addSkill.folder.detect")}
              </button>
            </div>
          {/if}

          {#if folderError}
            <div class="flex items-center gap-2 text-sm text-[var(--error)]">
              <AlertCircle size={16} />
              <span>{folderError}</span>
            </div>
          {/if}

          {#if detectedFolderSkills.length > 0}
            <div class="space-y-2">
              <p class="text-sm font-medium text-[var(--base-content)]">
                {$t("addSkill.folder.selectSkill")}
              </p>
              <div
                class="max-h-48 space-y-2 overflow-y-auto rounded-xl border border-[var(--base-300)] bg-[var(--base-200)] p-2"
              >
                {#each detectedFolderSkills as skill}
                  <button
                    class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${selectedFolderSkill?.path === skill.path ? "bg-[var(--primary)] text-[var(--primary-content)]" : "bg-[var(--base-100)] text-[var(--base-content)] hover:bg-[var(--base-300)]"}`}
                    onclick={() => (selectedFolderSkill = skill)}
                    type="button"
                  >
                    <div>
                      <p class="font-medium">{skill.name}</p>
                      <p
                        class={`text-xs ${selectedFolderSkill?.path === skill.path ? "text-[var(--primary-content)] opacity-80" : "text-[var(--base-content-muted)]"}`}
                      >
                        {skill.path}
                      </p>
                    </div>
                    {#if selectedFolderSkill?.path === skill.path}
                      <Check size={16} />
                    {/if}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {:else}
        <!-- GitHub Mode -->
        <div class="space-y-4">
          <p class="text-sm text-[var(--base-content-muted)]">
            {$t("addSkill.github.description")}
          </p>
          <div class="flex gap-2">
            <input
              type="text"
              class="flex-1 rounded-xl border border-[var(--base-300)] bg-[var(--base-200)] px-4 py-2 text-sm text-[var(--base-content)] placeholder:text-[var(--base-content-subtle)] focus:border-[var(--primary)] focus:outline-none"
              placeholder={$t("addSkill.github.urlPlaceholder")}
              bind:value={githubUrl}
              onkeydown={(e) => e.key === "Enter" && handleDetectGithub()}
            />
            <button
              class="rounded-xl bg-[var(--primary)] px-4 py-2 text-sm text-[var(--primary-content)] transition hover:bg-[var(--primary-hover)] disabled:opacity-50"
              onclick={handleDetectGithub}
              disabled={!githubUrl.trim() || isDetecting}
              type="button"
            >
              {#if isDetecting}
                <Loader2 size={16} class="animate-spin" />
              {:else}
                {$t("addSkill.github.detect")}
              {/if}
            </button>
          </div>

          {#if githubError}
            <div class="flex items-center gap-2 text-sm text-[var(--error)]">
              <AlertCircle size={16} />
              <span>{githubError}</span>
            </div>
          {/if}

          {#if detectedSkills.length > 0}
            <div class="space-y-2">
              <p class="text-sm font-medium text-[var(--base-content)]">
                {$t("addSkill.github.selectSkill")}
              </p>
              <div
                class="max-h-48 space-y-2 overflow-y-auto rounded-xl border border-[var(--base-300)] bg-[var(--base-200)] p-2"
              >
                {#each detectedSkills as skill}
                  <button
                    class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${selectedSkill?.path === skill.path ? "bg-[var(--primary)] text-[var(--primary-content)]" : "bg-[var(--base-100)] text-[var(--base-content)] hover:bg-[var(--base-300)]"}`}
                    onclick={() => (selectedSkill = skill)}
                    type="button"
                  >
                    <div>
                      <p class="font-medium">{skill.name}</p>
                      <p
                        class={`text-xs ${selectedSkill?.path === skill.path ? "text-[var(--primary-content)] opacity-80" : "text-[var(--base-content-muted)]"}`}
                      >
                        {skill.path}
                      </p>
                    </div>
                    {#if selectedSkill?.path === skill.path}
                      <Check size={16} />
                    {/if}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <!-- Agent Selection -->
      <div class="mt-6 space-y-3">
        <div class="flex items-center justify-between">
          <p class="text-sm font-medium text-[var(--base-content)]">
            {$t("addSkill.selectAgents")}
          </p>
          <div class="flex gap-2">
            {#if allSelected}
              <button
                class="text-xs text-[var(--base-content-muted)] transition hover:text-[var(--base-content)]"
                onclick={deselectAllAgents}
                type="button"
              >
                {$t("addSkill.deselectAll")}
              </button>
            {:else}
              <button
                class="text-xs text-[var(--base-content-muted)] transition hover:text-[var(--base-content)]"
                onclick={selectAllAgents}
                type="button"
              >
                {$t("addSkill.selectAll")}
              </button>
            {/if}
          </div>
        </div>
        <div class="flex flex-wrap gap-2">
          {#each agents as agent}
            <label
              class={`flex cursor-pointer items-center gap-2 rounded-lg border px-3 py-2 text-sm transition ${selectedAgents.includes(agent.id) ? "border-[var(--primary)] bg-[var(--primary)] text-[var(--primary-content)]" : "border-[var(--base-300)] bg-[var(--base-100)] text-[var(--base-content)] hover:bg-[var(--base-200)]"}`}
            >
              <input
                type="checkbox"
                class="hidden"
                checked={selectedAgents.includes(agent.id)}
                onchange={() => toggleAgent(agent.id)}
              />
              <span>{agent.display_name}</span>
            </label>
          {/each}
        </div>
      </div>

      {#if installError}
        <div class="mt-4 flex items-center gap-2 text-sm text-[var(--error)]">
          <AlertCircle size={16} />
          <span>{installError}</span>
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div
      class="flex justify-end gap-3 border-t border-[var(--base-300)] px-6 py-4 bg-[var(--base-100)] rounded-b-2xl"
    >
      <button
        class="rounded-xl border border-[var(--base-300)] px-4 py-2 text-sm text-[var(--base-content)] transition hover:bg-[var(--base-200)]"
        onclick={closeModal}
        disabled={isInstalling}
        type="button"
      >
        {$t("addSkill.cancel")}
      </button>
      <button
        class="rounded-xl bg-[var(--primary)] px-4 py-2 text-sm text-[var(--primary-content)] transition hover:bg-[var(--primary-hover)] disabled:opacity-50"
        onclick={handleConfirm}
        disabled={!canConfirm() || isInstalling}
        type="button"
      >
        {#if isInstalling}
          <Loader2 size={16} class="animate-spin" />
        {:else}
          {$t("addSkill.confirm")}
        {/if}
      </button>
    </div>
  </div>
</Modal>
