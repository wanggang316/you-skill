<script>
  import { FileArchive, Github, Check, AlertCircle, Loader2, Folder } from "@lucide/svelte";
  import { t } from "../i18n";
  import {
    detectZip,
    detectGithubManual,
    detectFolder,
    installFromNative,
    installFromGithub,
  } from "../api/skills";
  import Modal from "./ui/Modal.svelte";
  import AgentSelector from "./AgentSelector.svelte";

  /** @type {{ open?: boolean; agents?: import('../api/skills').AgentInfo[]; onSuccess?: () => void }} */
  let { open = $bindable(false), agents = [], onSuccess = () => {} } = $props();

  // Tab state: 'github' | 'zip' | 'folder'
  let activeTab = $state("github");

  // ZIP file state
  let selectedZipPath = $state("");
  let zipFileName = $state("");
  let isDetectingZip = $state(false);
  /** @type {import('../api/skills').DetectedSkill[]} */
  let detectedZipSkills = $state([]);
  /** @type {import('../api/skills').DetectedSkill | null} */
  let selectedZipSkill = $state(null);
  let zipError = $state("");

  // Folder state
  let selectedFolderPath = $state("");
  let folderName = $state("");
  let isDetectingFolder = $state(false);
  /** @type {import('../api/skills').DetectedSkill[]} */
  let detectedFolderSkills = $state([]);
  /** @type {import('../api/skills').DetectedSkill | null} */
  let selectedFolderSkill = $state(null);
  let folderError = $state("");

  // Github state
  let githubUrl = $state("");
  let isDetecting = $state(false);
  /** @type {import('../api/skills').DetectedSkill[]} */
  let detectedSkills = $state([]);
  /** @type {import('../api/skills').DetectedSkill | null} */
  let selectedSkill = $state(null);
  let githubError = $state("");

  // Agent selection state
  /** @type {string[]} */
  let selectedAgents = $state([]);
  /** @type {"symlink" | "copy"} */
  let selectedMethod = $state("symlink");
  let isZipDragOver = $state(false);
  let isFolderDragOver = $state(false);

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
    activeTab = "github";
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
    selectedMethod = "symlink";
    isInstalling = false;
    installError = "";
  }

  function closeModal() {
    open = false;
  }

  /** @param {string} url */
  function toGitRepoUrl(url) {
    const trimmed = url.trim();
    return trimmed.endsWith(".git") ? trimmed : `${trimmed}.git`;
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
        await handleDetectZip();
      }
    } catch (error) {
      console.error("Failed to select zip file:", error);
    }
  }

  /**
   * @param {DragEvent} event
   * @returns {string | null}
   */
  function extractPathFromDrop(event) {
    const dt = event.dataTransfer;
    if (!dt) return null;

    const firstFile = dt.files && dt.files.length > 0 ? dt.files[0] : null;
    if (firstFile) {
      const fileWithPath = /** @type {{ path?: string }} */ (firstFile);
      if (typeof fileWithPath.path === "string" && fileWithPath.path) {
        return fileWithPath.path;
      }
    }

    for (const item of Array.from(dt.items || [])) {
      const file = item.getAsFile?.();
      if (file) {
        const fileWithPath = /** @type {{ path?: string }} */ (file);
        if (typeof fileWithPath.path === "string" && fileWithPath.path) {
          return fileWithPath.path;
        }
      }
    }

    const rawUri = dt.getData("text/uri-list") || dt.getData("text/plain");
    if (rawUri) {
      const firstLine = rawUri
        .split("\n")
        .map((line) => line.trim())
        .find((line) => line && !line.startsWith("#"));
      if (firstLine && firstLine.startsWith("file://")) {
        try {
          return decodeURIComponent(new URL(firstLine).pathname);
        } catch {
          // no-op
        }
      }
    }

    return null;
  }

  /** @param {DragEvent} event */
  function handleDragOver(event) {
    event.preventDefault();
  }

  /** @param {DragEvent} event */
  async function handleZipDrop(event) {
    event.preventDefault();
    isZipDragOver = false;
    const path = extractPathFromDrop(event);
    if (!path) {
      zipError = "Failed to read dropped file path.";
      return;
    }
    if (!/\.(zip|skill)$/i.test(path)) {
      zipError = "Please drop a .zip or .skill file.";
      return;
    }
    zipError = "";
    selectedZipPath = path;
    zipFileName = path.split(/[/\\]/).pop() || "";
    await handleDetectZip();
  }

  /** @param {DragEvent} event */
  async function handleFolderDrop(event) {
    event.preventDefault();
    isFolderDragOver = false;
    const path = extractPathFromDrop(event);
    if (!path) {
      folderError = "Failed to read dropped folder path.";
      return;
    }
    folderError = "";
    selectedFolderPath = path;
    folderName = path.split(/[/\\]/).pop() || "";
    await handleDetectFolder();
  }

  async function handleDetectZip() {
    if (!selectedZipPath) return;

    isDetectingZip = true;
    zipError = "";
    detectedZipSkills = [];
    selectedZipSkill = null;

    try {
      const skill = await detectZip(selectedZipPath);
      detectedZipSkills = [skill];
      selectedZipSkill = skill;
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
        await handleDetectFolder();
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
      const skill = await detectFolder(selectedFolderPath);
      detectedFolderSkills = [skill];
      selectedFolderSkill = skill;
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
      const skills = await detectGithubManual(githubUrl.trim());
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
        await installFromNative({
          name: selectedZipSkill.name,
          tmp_path: selectedZipSkill.tmp_path,
          skill_path: selectedZipSkill.skill_path,
          agent_apps: selectedAgents,
          method: selectedMethod,
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
        await installFromNative({
          name: selectedFolderSkill.name,
          tmp_path: selectedFolderSkill.tmp_path,
          skill_path: selectedFolderSkill.skill_path,
          agent_apps: selectedAgents,
          method: selectedMethod,
        });
      } else {
        if (!selectedSkill) {
          installError = $t("addSkill.noSkillSelected");
          return;
        }
        await installFromGithub({
          name: selectedSkill.name,
          tmp_path: selectedSkill.tmp_path,
          skill_path: selectedSkill.skill_path,
          source_url: toGitRepoUrl(githubUrl),
          skill_folder_hash: null,
          agent_apps: selectedAgents,
          method: selectedMethod,
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
</script>

<Modal bind:open title={$t("addSkill.title")} onClose={closeModal}>
  <div class="flex h-full max-h-[90vh] w-full max-w-lg flex-col">
    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6 pt-16">
      <!-- Tabs -->
      <div class="bg-base-200 mb-6 flex gap-2 rounded-full p-1">
        <button
          class={`flex flex-1 items-center justify-center gap-2 rounded-full px-3 py-2 text-sm transition ${activeTab === "github" ? "bg-base-100 text-base-content shadow-sm" : "text-base-content-muted hover:text-base-content"}`}
          onclick={() => (activeTab = "github")}
          type="button"
        >
          <Github size={16} />
          {$t("addSkill.tab.github")}
        </button>
        <button
          class={`flex flex-1 items-center justify-center gap-2 rounded-full px-3 py-2 text-sm transition ${activeTab === "zip" ? "bg-base-100 text-base-content shadow-sm" : "text-base-content-muted hover:text-base-content"}`}
          onclick={() => (activeTab = "zip")}
          type="button"
        >
          <FileArchive size={16} />
          {$t("addSkill.tab.zip")}
        </button>
        <button
          class={`flex flex-1 items-center justify-center gap-2 rounded-full px-3 py-2 text-sm transition ${activeTab === "folder" ? "bg-base-100 text-base-content shadow-sm" : "text-base-content-muted hover:text-base-content"}`}
          onclick={() => (activeTab = "folder")}
          type="button"
        >
          <Folder size={16} />
          {$t("addSkill.tab.folder")}
        </button>
      </div>

      {#if activeTab === "github"}
        <!-- GitHub Mode -->
        <div class="space-y-4">
          <p class="text-base-content-muted text-sm">
            {$t("addSkill.github.description")}
          </p>
          <div class="flex gap-2">
            <input
              type="text"
              class="border-base-300 bg-base-200 text-base-content placeholder:text-base-content-subtle focus:border-primary flex-1 rounded-xl border px-4 py-2 text-sm focus:outline-none"
              placeholder={$t("addSkill.github.urlPlaceholder")}
              bind:value={githubUrl}
              onkeydown={(e) => e.key === "Enter" && handleDetectGithub()}
            />
            <button
              class="bg-primary text-primary-content hover:bg-primary-hover rounded-xl px-4 py-2 text-sm transition disabled:opacity-50"
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
            <div class="text-error flex items-center gap-2 text-sm">
              <AlertCircle size={16} />
              <span>{githubError}</span>
            </div>
          {/if}

          {#if detectedSkills.length > 0}
            <div class="space-y-2">
              <p class="text-base-content text-sm font-medium">
                {$t("addSkill.github.selectSkill")}
              </p>
              <div
                class="border-base-300 bg-base-200 max-h-48 space-y-2 overflow-y-auto rounded-xl border p-2"
              >
                {#each detectedSkills as skill}
                  <button
                    class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${selectedSkill?.skill_path === skill.skill_path ? "bg-primary text-primary-content" : "bg-base-100 text-base-content hover:bg-base-300"}`}
                    onclick={() => (selectedSkill = skill)}
                    type="button"
                  >
                    <div>
                      <p class="font-medium">{skill.name}</p>
                      <p
                        class={`text-xs ${selectedSkill?.skill_path === skill.skill_path ? "text-primary-content opacity-80" : "text-base-content-muted"}`}
                      >
                        {skill.skill_path}
                      </p>
                    </div>
                    {#if selectedSkill?.skill_path === skill.skill_path}
                      <Check size={16} />
                    {/if}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {:else if activeTab === "zip"}
        <!-- ZIP Mode -->
        <div class="space-y-3">
          <p class="text-base-content-muted text-xs">
            {$t("addSkill.zip.description")}
          </p>
          <button
            class={`w-full rounded-xl border-2 border-dashed p-3 transition ${
              isZipDragOver
                ? "border-primary bg-base-200"
                : "border-base-300 hover:border-primary hover:bg-base-200"
            }`}
            onclick={handleSelectZipFile}
            ondragover={handleDragOver}
            ondragenter={() => (isZipDragOver = true)}
            ondragleave={() => (isZipDragOver = false)}
            ondrop={handleZipDrop}
            type="button"
          >
            {#if selectedZipPath}
              <div class="text-base-content flex items-center justify-center gap-2 text-sm">
                <FileArchive size={16} class="text-primary" />
                <span class="font-medium">{zipFileName}</span>
              </div>
              <p class="text-base-content-muted mt-1 text-[11px]">
                {$t("addSkill.zip.clickToChange")}
              </p>
            {:else}
              <div class="text-base-content-muted flex flex-col items-center gap-1 text-sm">
                <FileArchive size={24} />
                <span>{$t("addSkill.zip.selectFile")}</span>
              </div>
            {/if}
          </button>

          {#if zipError}
            <div class="text-error flex items-center gap-2 text-sm">
              <AlertCircle size={16} />
              <span>{zipError}</span>
            </div>
          {/if}

          {#if isDetectingZip}
            <div class="text-base-content-muted flex items-center gap-2 text-sm">
              <Loader2 size={16} class="animate-spin" />
              <span>{$t("addSkill.zip.detect")}</span>
            </div>
          {/if}

          {#if detectedZipSkills.length > 0}
            <div class="space-y-2">
              <p class="text-base-content text-sm font-medium">
                {$t("addSkill.zip.selectSkill")}
              </p>
              <div
                class="border-base-300 bg-base-200 max-h-48 space-y-2 overflow-y-auto rounded-xl border p-2"
              >
                {#each detectedZipSkills as skill}
                  <button
                    class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${selectedZipSkill?.skill_path === skill.skill_path ? "bg-primary text-primary-content" : "bg-base-100 text-base-content hover:bg-base-300"}`}
                    onclick={() => (selectedZipSkill = skill)}
                    type="button"
                  >
                    <div>
                      <p class="font-medium">{skill.name}</p>
                      <p
                        class={`text-xs ${selectedZipSkill?.skill_path === skill.skill_path ? "text-primary-content opacity-80" : "text-base-content-muted"}`}
                      >
                        {skill.skill_path}
                      </p>
                    </div>
                    {#if selectedZipSkill?.skill_path === skill.skill_path}
                      <Check size={16} />
                    {/if}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {:else}
        <!-- Folder Mode -->
        <div class="space-y-3">
          <p class="text-base-content-muted text-xs">
            {$t("addSkill.folder.description")}
          </p>
          <button
            class={`w-full rounded-xl border-2 border-dashed p-3 transition ${
              isFolderDragOver
                ? "border-primary bg-base-200"
                : "border-base-300 hover:border-primary hover:bg-base-200"
            }`}
            onclick={handleSelectFolder}
            ondragover={handleDragOver}
            ondragenter={() => (isFolderDragOver = true)}
            ondragleave={() => (isFolderDragOver = false)}
            ondrop={handleFolderDrop}
            type="button"
          >
            {#if selectedFolderPath}
              <div class="text-base-content flex items-center justify-center gap-2 text-sm">
                <Folder size={16} class="text-primary" />
                <span class="font-medium">{folderName}</span>
              </div>
              <p class="text-base-content-muted mt-1 text-[11px]">
                {$t("addSkill.folder.clickToChange")}
              </p>
            {:else}
              <div class="text-base-content-muted flex flex-col items-center gap-1 text-sm">
                <Folder size={24} />
                <span>{$t("addSkill.folder.selectFolder")}</span>
              </div>
            {/if}
          </button>

          {#if folderError}
            <div class="text-error flex items-center gap-2 text-sm">
              <AlertCircle size={16} />
              <span>{folderError}</span>
            </div>
          {/if}

          {#if isDetectingFolder}
            <div class="text-base-content-muted flex items-center gap-2 text-sm">
              <Loader2 size={16} class="animate-spin" />
              <span>{$t("addSkill.folder.detect")}</span>
            </div>
          {/if}

          {#if detectedFolderSkills.length > 0}
            <div class="space-y-2">
              <p class="text-base-content text-sm font-medium">
                {$t("addSkill.folder.selectSkill")}
              </p>
              <div
                class="border-base-300 bg-base-200 max-h-48 space-y-2 overflow-y-auto rounded-xl border p-2"
              >
                {#each detectedFolderSkills as skill}
                  <button
                    class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${selectedFolderSkill?.skill_path === skill.skill_path ? "bg-primary text-primary-content" : "bg-base-100 text-base-content hover:bg-base-300"}`}
                    onclick={() => (selectedFolderSkill = skill)}
                    type="button"
                  >
                    <div>
                      <p class="font-medium">{skill.name}</p>
                      <p
                        class={`text-xs ${selectedFolderSkill?.skill_path === skill.skill_path ? "text-primary-content opacity-80" : "text-base-content-muted"}`}
                      >
                        {skill.skill_path}
                      </p>
                    </div>
                    {#if selectedFolderSkill?.skill_path === skill.skill_path}
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
        <p class="text-base-content text-sm font-medium">
          {$t("addSkill.selectAgents")}
        </p>
        <AgentSelector {agents} selectedIds={selectedAgents} />
      </div>

      {#if installError}
        <div class="text-error mt-4 flex items-center gap-2 text-sm">
          <AlertCircle size={16} />
          <span>{installError}</span>
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div
      class="border-base-300 bg-base-100 flex items-center justify-end gap-3 rounded-b-2xl border-t px-6 py-3"
    >
      <select
        bind:value={selectedMethod}
        class="bg-base-100 text-base-content rounded-lg px-3 py-2 text-sm"
        disabled={isInstalling}
      >
        <option value="symlink">{$t("settings.syncMode.symlink")}</option>
        <option value="copy">{$t("settings.syncMode.copy")}</option>
      </select>
      <button
        class="bg-primary text-primary-content hover:bg-primary-hover rounded-xl px-4 py-2 text-sm transition disabled:opacity-50"
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
