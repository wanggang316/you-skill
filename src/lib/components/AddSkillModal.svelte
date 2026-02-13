<script>
  import { FileArchive, Github, Check, AlertCircle, Loader2, Folder } from "@lucide/svelte";
  import { t } from "../i18n";
  import {
    detectZipSkills,
    detectGithubSkills,
    installPackage,
    installGithubManual,
    detectFolderSkills,
    installFolder,
  } from "../api/skills";
  import Modal from "./ui/Modal.svelte";
  import AgentSelector from "./AgentSelector.svelte";

  /** @type {{ open?: boolean; agents?: import('../api/skills').AgentInfo[]; onSuccess?: () => void }} */
  let { open = $bindable(false), agents = [], onSuccess = () => {} } = $props();

  // Tab state: 'zip' | 'folder' | 'github'
  let activeTab = $state("zip");

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
        await installPackage({
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
        await installFolder({
          folder_path: selectedFolderPath,
          skill_path: selectedFolderSkill.path,
          agents: selectedAgents,
        });
      } else {
        if (!selectedSkill) {
          installError = $t("addSkill.noSkillSelected");
          return;
        }
        await installGithubManual({
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
</script>

<Modal bind:open title={$t("addSkill.title")} onClose={closeModal}>
  <div class="flex h-full max-h-[90vh] w-full max-w-lg flex-col">
    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6 pt-16">
      <!-- Tabs -->
      <div class="bg-base-200 mb-6 flex gap-2 rounded-full p-1">
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
        <button
          class={`flex flex-1 items-center justify-center gap-2 rounded-full px-3 py-2 text-sm transition ${activeTab === "github" ? "bg-base-100 text-base-content shadow-sm" : "text-base-content-muted hover:text-base-content"}`}
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
          <p class="text-base-content-muted text-sm">
            {$t("addSkill.zip.description")}
          </p>
          <button
            class="border-base-300 hover:border-primary hover:bg-base-200 w-full rounded-xl border-2 border-dashed p-8 transition"
            onclick={handleSelectZipFile}
            type="button"
          >
            {#if selectedZipPath}
              <div class="text-base-content flex items-center justify-center gap-2">
                <FileArchive size={20} class="text-primary" />
                <span class="font-medium">{zipFileName}</span>
              </div>
              <p class="text-base-content-muted mt-2 text-xs">
                {$t("addSkill.zip.clickToChange")}
              </p>
            {:else}
              <div class="text-base-content-muted flex flex-col items-center gap-2">
                <FileArchive size={32} />
                <span>{$t("addSkill.zip.selectFile")}</span>
              </div>
            {/if}
          </button>

          {#if selectedZipPath}
            <div class="flex gap-2">
              <button
                class="bg-primary text-primary-content hover:bg-primary-hover flex-1 rounded-xl px-4 py-2 text-sm transition disabled:opacity-50"
                onclick={handleDetectZip}
                disabled={isDetectingZip}
                type="button"
              >
                {#if isDetectingZip}
                  <Loader2 size={16} class="mr-1 inline animate-spin" />
                {/if}
                {$t("addSkill.zip.detect")}
              </button>
            </div>
          {/if}

          {#if zipError}
            <div class="text-error flex items-center gap-2 text-sm">
              <AlertCircle size={16} />
              <span>{zipError}</span>
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
                    class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${selectedZipSkill?.path === skill.path ? "bg-primary text-primary-content" : "bg-base-100 text-base-content hover:bg-base-300"}`}
                    onclick={() => (selectedZipSkill = skill)}
                    type="button"
                  >
                    <div>
                      <p class="font-medium">{skill.name}</p>
                      <p
                        class={`text-xs ${selectedZipSkill?.path === skill.path ? "text-primary-content opacity-80" : "text-base-content-muted"}`}
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
          <p class="text-base-content-muted text-sm">
            {$t("addSkill.folder.description")}
          </p>
          <button
            class="border-base-300 hover:border-primary hover:bg-base-200 w-full rounded-xl border-2 border-dashed p-8 transition"
            onclick={handleSelectFolder}
            type="button"
          >
            {#if selectedFolderPath}
              <div class="text-base-content flex items-center justify-center gap-2">
                <Folder size={20} class="text-primary" />
                <span class="font-medium">{folderName}</span>
              </div>
              <p class="text-base-content-muted mt-2 text-xs">
                {$t("addSkill.folder.clickToChange")}
              </p>
            {:else}
              <div class="text-base-content-muted flex flex-col items-center gap-2">
                <Folder size={32} />
                <span>{$t("addSkill.folder.selectFolder")}</span>
              </div>
            {/if}
          </button>

          {#if selectedFolderPath}
            <div class="flex gap-2">
              <button
                class="bg-primary text-primary-content hover:bg-primary-hover flex-1 rounded-xl px-4 py-2 text-sm transition disabled:opacity-50"
                onclick={handleDetectFolder}
                disabled={isDetectingFolder}
                type="button"
              >
                {#if isDetectingFolder}
                  <Loader2 size={16} class="mr-1 inline animate-spin" />
                {/if}
                {$t("addSkill.folder.detect")}
              </button>
            </div>
          {/if}

          {#if folderError}
            <div class="text-error flex items-center gap-2 text-sm">
              <AlertCircle size={16} />
              <span>{folderError}</span>
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
                    class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${selectedFolderSkill?.path === skill.path ? "bg-primary text-primary-content" : "bg-base-100 text-base-content hover:bg-base-300"}`}
                    onclick={() => (selectedFolderSkill = skill)}
                    type="button"
                  >
                    <div>
                      <p class="font-medium">{skill.name}</p>
                      <p
                        class={`text-xs ${selectedFolderSkill?.path === skill.path ? "text-primary-content opacity-80" : "text-base-content-muted"}`}
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
                    class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${selectedSkill?.path === skill.path ? "bg-primary text-primary-content" : "bg-base-100 text-base-content hover:bg-base-300"}`}
                    onclick={() => (selectedSkill = skill)}
                    type="button"
                  >
                    <div>
                      <p class="font-medium">{skill.name}</p>
                      <p
                        class={`text-xs ${selectedSkill?.path === skill.path ? "text-primary-content opacity-80" : "text-base-content-muted"}`}
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
      class="border-base-300 bg-base-100 flex justify-end gap-3 rounded-b-2xl border-t px-6 py-3"
    >
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
