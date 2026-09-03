<script lang="ts">
  import { AlertCircle, CheckCircle2, FileArchive, Folder, Github, Loader2 } from "@lucide/svelte";
  import { get } from "svelte/store";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import SegmentedTabs from "$lib/components/ui/SegmentedTabs.svelte";
  import DetectedSkillList from "./DetectedSkillList.svelte";
  import ScanResultList, { type ScanChoice } from "./ScanResultList.svelte";
  import { t } from "../i18n";
  import {
    importScanned,
    importSkills,
    scanFolder,
    type ImportItem,
    type ImportOutcome,
    type ScanDecision,
    type ScanItem,
    type SkillSource,
  } from "../api/hub";
  import { detectGithubManual, detectZip, type DetectedSkill } from "../api/skills";
  import { hubSkillsByName, refreshHub } from "../stores/hub";
  import {
    closeImportModal,
    importModal,
    openInstallModal,
    type ImportTab,
  } from "../stores/modals";

  let open = $state(false);
  let activeTab = $state<ImportTab>("github");

  // GitHub
  let githubUrl = $state("");
  let isDetectingGithub = $state(false);
  let detectedGithubSkills = $state<DetectedSkill[]>([]);
  let selectedGithubSkills = $state<DetectedSkill[]>([]);
  let githubError = $state("");

  // Zip
  let selectedZipPath = $state("");
  let zipFileName = $state("");
  let isDetectingZip = $state(false);
  let detectedZipSkills = $state<DetectedSkill[]>([]);
  let selectedZipSkills = $state<DetectedSkill[]>([]);
  let zipError = $state("");
  let isZipDragOver = $state(false);
  let suppressZipClick = $state(false);

  // Folder (scan)
  let selectedFolderPath = $state("");
  let folderName = $state("");
  let isScanning = $state(false);
  let scanItems = $state<ScanItem[]>([]);
  let scanChoices = $state<Record<string, ScanChoice>>({});
  let folderError = $state("");
  let isFolderDragOver = $state(false);
  let suppressFolderClick = $state(false);

  // Import
  let isImporting = $state(false);
  let importError = $state("");
  let overwriteExisting = $state(false);
  let importedOutcomes = $state<ImportOutcome[] | null>(null);
  let unlistenNativeDragDrop: (() => void) | null = null;

  const selectedDetected = $derived(
    activeTab === "github" ? selectedGithubSkills : activeTab === "zip" ? selectedZipSkills : []
  );
  const existingNames = $derived(
    selectedDetected.map((skill) => skill.name).filter((name) => $hubSkillsByName.has(name))
  );
  const selectedScanCount = $derived(
    scanItems.filter((item) => scanChoices[item.path]?.selected).length
  );
  const canConfirm = $derived.by(() => {
    if (isImporting) return false;
    if (activeTab === "folder") return selectedScanCount > 0;
    if (selectedDetected.length === 0) return false;
    if (existingNames.length > 0 && !overwriteExisting) return false;
    return true;
  });

  $effect(() => {
    const state = $importModal;
    if (state.open && !open) {
      resetState();
      activeTab = state.initialTab;
      if (state.initialFolder) {
        void applyFolderPath(state.initialFolder);
      }
    }
    open = state.open;
  });

  function resetState() {
    activeTab = "github";
    githubUrl = "";
    isDetectingGithub = false;
    detectedGithubSkills = [];
    selectedGithubSkills = [];
    githubError = "";
    selectedZipPath = "";
    zipFileName = "";
    isDetectingZip = false;
    detectedZipSkills = [];
    selectedZipSkills = [];
    zipError = "";
    selectedFolderPath = "";
    folderName = "";
    isScanning = false;
    scanItems = [];
    scanChoices = {};
    folderError = "";
    isImporting = false;
    importError = "";
    overwriteExisting = false;
    importedOutcomes = null;
    suppressZipClick = false;
    suppressFolderClick = false;
  }

  function handleClose() {
    open = false;
    closeImportModal();
  }

  // ---------------------------------------------------------------------------
  // Drag & drop (Tauri native events + HTML5 fallback)
  // ---------------------------------------------------------------------------

  $effect(() => {
    if (!open) return;
    let disposed = false;
    const onWindowDragOver = (event: DragEvent) => {
      event.preventDefault();
      if (activeTab === "zip") isZipDragOver = true;
      if (activeTab === "folder") isFolderDragOver = true;
    };
    const onWindowDragLeave = () => {
      isZipDragOver = false;
      isFolderDragOver = false;
    };
    const onWindowDrop = (event: DragEvent) => {
      event.preventDefault();
      isZipDragOver = false;
      isFolderDragOver = false;
    };

    const setup = async () => {
      if (typeof window === "undefined") return;
      window.addEventListener("dragover", onWindowDragOver);
      window.addEventListener("dragleave", onWindowDragLeave);
      window.addEventListener("drop", onWindowDrop);
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        const unlisten = await getCurrentWebview().onDragDropEvent((event) => {
          const payload = event.payload;
          if (payload.type === "enter" || payload.type === "over") {
            if (activeTab === "zip") isZipDragOver = true;
            if (activeTab === "folder") isFolderDragOver = true;
            return;
          }
          if (payload.type === "leave") {
            isZipDragOver = false;
            isFolderDragOver = false;
            return;
          }
          if (payload.type === "drop") {
            isZipDragOver = false;
            isFolderDragOver = false;
            const droppedPath = payload.paths?.[0] ?? "";
            if (!droppedPath) return;
            if (activeTab === "zip") void applyZipPath(droppedPath);
            if (activeTab === "folder") void applyFolderPath(droppedPath);
          }
        });
        if (disposed) {
          unlisten();
          return;
        }
        unlistenNativeDragDrop = unlisten;
      } catch {
        // Browser preview without Tauri.
      }
    };
    void setup();

    return () => {
      disposed = true;
      isZipDragOver = false;
      isFolderDragOver = false;
      if (typeof window !== "undefined") {
        window.removeEventListener("dragover", onWindowDragOver);
        window.removeEventListener("dragleave", onWindowDragLeave);
        window.removeEventListener("drop", onWindowDrop);
      }
      unlistenNativeDragDrop?.();
      unlistenNativeDragDrop = null;
    };
  });

  function pathFromUri(uri: string): string | null {
    if (!uri.startsWith("file://")) return null;
    try {
      return decodeURIComponent(new URL(uri).pathname);
    } catch {
      return null;
    }
  }

  function pathFromDropText(raw: string): string | null {
    const firstLine = raw
      .split("\n")
      .map((line) => line.trim())
      .find((line) => line && !line.startsWith("#"));
    if (!firstLine) return null;
    return pathFromUri(firstLine) ?? firstLine.replace(/^['"]|['"]$/g, "") ?? null;
  }

  function normalizeDroppedPath(path: string): string {
    const normalized = path.trim();
    if (!normalized) return "";
    if (normalized.startsWith("file://")) return pathFromUri(normalized) || "";
    return normalized;
  }

  function extractPathFromTransfer(dt: DataTransfer | null): string | null {
    if (!dt) return null;
    const candidates: Array<File | null> = [
      dt.files?.[0] ?? null,
      ...Array.from(dt.items || []).map((item) => item.getAsFile?.() ?? null),
    ];
    for (const file of candidates) {
      const withPath = file as (File & { path?: string }) | null;
      if (withPath?.path) return withPath.path;
    }
    const rawUri = dt.getData("text/uri-list") || dt.getData("text/plain");
    return rawUri ? pathFromDropText(rawUri) : null;
  }

  function baseName(path: string): string {
    return path.split(/[/\\]/).filter(Boolean).pop() || "";
  }

  async function persistDroppedZipFile(file: File): Promise<string> {
    const { mkdir, writeFile, BaseDirectory } = await import("@tauri-apps/plugin-fs");
    const { appLocalDataDir, join } = await import("@tauri-apps/api/path");
    const folder = "dropped-skills";
    await mkdir(folder, { baseDir: BaseDirectory.AppLocalData, recursive: true });
    const safeName = (file.name || "dropped.skill").replace(/[^\w.-]/g, "_");
    const relativePath = `${folder}/${Date.now()}-${Math.random().toString(36).slice(2, 10)}-${safeName}`;
    await writeFile(relativePath, new Uint8Array(await file.arrayBuffer()), {
      baseDir: BaseDirectory.AppLocalData,
    });
    return await join(await appLocalDataDir(), relativePath);
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
  }

  async function handleZipDrop(event: DragEvent) {
    event.preventDefault();
    isZipDragOver = false;
    suppressZipClick = true;
    setTimeout(() => (suppressZipClick = false), 100);
    const path = extractPathFromTransfer(event.dataTransfer);
    if (path) {
      await applyZipPath(path);
      return;
    }
    const file = event.dataTransfer?.files?.[0];
    if (!file) {
      zipError = $t("import.zip.dropReadError");
      return;
    }
    try {
      await applyZipPath(await persistDroppedZipFile(file), file.name);
    } catch (error) {
      console.error("[ImportSkillModal] zip drop failed:", error);
      zipError = $t("import.zip.dropReadError");
    }
  }

  async function handleFolderDrop(event: DragEvent) {
    event.preventDefault();
    isFolderDragOver = false;
    suppressFolderClick = true;
    setTimeout(() => (suppressFolderClick = false), 100);
    const path = extractPathFromTransfer(event.dataTransfer);
    if (path) {
      await applyFolderPath(path);
      return;
    }
    folderError = $t("import.folder.dropReadError");
  }

  // ---------------------------------------------------------------------------
  // GitHub
  // ---------------------------------------------------------------------------

  function parseGithubInput(value: string): { githubPath: string; sourceUrl: string } | null {
    const trimmed = value.trim();
    if (!trimmed) return null;
    if (/^https?:\/\//i.test(trimmed)) {
      return { githubPath: trimmed, sourceUrl: trimmed };
    }
    if (/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(trimmed)) {
      return { githubPath: trimmed, sourceUrl: `https://github.com/${trimmed}` };
    }
    return null;
  }

  function repoSlug(sourceUrl: string): string {
    const match = sourceUrl.match(/github\.com[/:]([^/]+)\/([^/#?]+)/);
    if (!match) return sourceUrl;
    return `${match[1]}/${match[2].replace(/\.git$/, "")}`;
  }

  async function handleDetectGithub() {
    const parsed = parseGithubInput(githubUrl);
    if (!parsed) {
      githubError = "Unsupported URL format. Use http(s) URL or owner/repo.";
      detectedGithubSkills = [];
      selectedGithubSkills = [];
      return;
    }
    isDetectingGithub = true;
    githubError = "";
    detectedGithubSkills = [];
    selectedGithubSkills = [];
    try {
      const skills = await detectGithubManual(parsed.githubPath);
      detectedGithubSkills = skills;
      selectedGithubSkills = [...skills];
      if (skills.length === 0) githubError = $t("import.noSkillsFound");
    } catch (error) {
      githubError = String(error);
    } finally {
      isDetectingGithub = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Zip
  // ---------------------------------------------------------------------------

  async function handleSelectZipFile() {
    if (suppressZipClick) return;
    try {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const result = await openDialog({
        multiple: false,
        directory: false,
        filters: [{ name: "Skill Files", extensions: ["skill", "zip"] }],
      });
      if (result) await applyZipPath(result);
    } catch (error) {
      console.error("Failed to select zip file:", error);
    }
  }

  async function applyZipPath(path: string, displayName = "") {
    const normalized = normalizeDroppedPath(path);
    if (!normalized) {
      zipError = $t("import.zip.dropReadError");
      return;
    }
    if (!/\.(zip|skill)$/i.test(normalized)) {
      zipError = $t("import.zip.invalidCompressedFile");
      return;
    }
    zipError = "";
    selectedZipPath = normalized;
    zipFileName = displayName || baseName(normalized);
    isDetectingZip = true;
    detectedZipSkills = [];
    selectedZipSkills = [];
    try {
      const skills = await detectZip(normalized);
      detectedZipSkills = skills;
      selectedZipSkills = [...skills];
      if (skills.length === 0) zipError = $t("import.noSkillsFound");
    } catch (error) {
      zipError = String(error);
    } finally {
      isDetectingZip = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Folder scan
  // ---------------------------------------------------------------------------

  async function handleSelectFolder() {
    if (suppressFolderClick) return;
    try {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const result = await openDialog({ multiple: false, directory: true });
      if (result) await applyFolderPath(result);
    } catch (error) {
      console.error("Failed to select folder:", error);
    }
  }

  function defaultChoice(item: ScanItem): ScanChoice {
    const known = get(hubSkillsByName).get(item.name);
    const alreadyRegistered = Boolean(
      known?.installs.some((install) => install.path === item.path)
    );
    switch (item.status) {
      case "new":
        return { selected: true, resolution: "import", registerInstall: Boolean(item.inAgentRoot) };
      case "different":
        return {
          selected: true,
          resolution: "adopt_into_hub",
          registerInstall: Boolean(item.inAgentRoot),
        };
      case "identical":
        return {
          selected: Boolean(item.inAgentRoot) && !alreadyRegistered,
          resolution: "import",
          registerInstall: Boolean(item.inAgentRoot),
        };
      default:
        return { selected: false, resolution: "skip", registerInstall: false };
    }
  }

  async function applyFolderPath(path: string) {
    const normalized = normalizeDroppedPath(path);
    if (!normalized) {
      folderError = $t("import.folder.dropReadError");
      return;
    }
    activeTab = "folder";
    folderError = "";
    selectedFolderPath = normalized;
    folderName = baseName(normalized);
    isScanning = true;
    scanItems = [];
    scanChoices = {};
    try {
      const items = await scanFolder(normalized);
      scanItems = items;
      const choices: Record<string, ScanChoice> = {};
      for (const item of items) choices[item.path] = defaultChoice(item);
      scanChoices = choices;
      if (items.length === 0) folderError = $t("import.folder.empty");
    } catch (error) {
      folderError = String(error);
    } finally {
      isScanning = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Selection helpers
  // ---------------------------------------------------------------------------

  function toggleSkill(list: DetectedSkill[], skill: DetectedSkill): DetectedSkill[] {
    const exists = list.some((s) => s.skill_path === skill.skill_path);
    return exists ? list.filter((s) => s.skill_path !== skill.skill_path) : [...list, skill];
  }

  function toggleAllSkills(current: DetectedSkill[], all: DetectedSkill[]): DetectedSkill[] {
    return current.length === all.length ? [] : [...all];
  }

  function findDuplicateNames(skills: DetectedSkill[]): string[] {
    const seen = new Set<string>();
    const dupes = new Set<string>();
    for (const skill of skills) {
      if (seen.has(skill.name)) dupes.add(skill.name);
      else seen.add(skill.name);
    }
    return [...dupes];
  }

  // ---------------------------------------------------------------------------
  // Import
  // ---------------------------------------------------------------------------

  function buildImportItems(): ImportItem[] {
    if (activeTab === "github") {
      const parsed = parseGithubInput(githubUrl);
      if (!parsed) throw new Error("Unsupported URL format. Use http(s) URL or owner/repo.");
      const repo = repoSlug(parsed.sourceUrl);
      return selectedGithubSkills.map((skill) => ({
        name: skill.name,
        tmpPath: skill.tmp_path,
        source: {
          type: "github",
          repo,
          url: `https://github.com/${repo}.git`,
          skillPath: skill.skill_path,
          branch: skill.branch ?? null,
        } satisfies SkillSource,
      }));
    }
    return selectedZipSkills.map((skill) => ({
      name: skill.name,
      tmpPath: skill.tmp_path,
      source: { type: "zip", path: selectedZipPath } satisfies SkillSource,
    }));
  }

  function buildScanDecisions(): ScanDecision[] {
    return scanItems
      .filter((item) => scanChoices[item.path]?.selected)
      .map((item) => {
        const choice = scanChoices[item.path];
        return {
          name: item.name,
          path: item.path,
          resolution: choice.resolution,
          registerInstall: choice.registerInstall,
          source: item.sourceHint ?? null,
        };
      });
  }

  async function handleConfirm() {
    if (!canConfirm) return;
    importError = "";
    if (activeTab !== "folder") {
      if (selectedDetected.length === 0) {
        importError = $t("import.noSkillSelected");
        return;
      }
      const duplicates = findDuplicateNames(selectedDetected);
      if (duplicates.length > 0) {
        importError = $t("import.duplicateNames", { names: duplicates.join(", ") });
        return;
      }
    }

    isImporting = true;
    try {
      const outcomes =
        activeTab === "folder"
          ? await importScanned(buildScanDecisions())
          : await importSkills(buildImportItems(), overwriteExisting);
      await refreshHub();
      importedOutcomes = outcomes;
    } catch (error) {
      importError = String(error);
      await refreshHub().catch(console.error);
    } finally {
      isImporting = false;
    }
  }

  function handleInstallNow() {
    const names = (importedOutcomes ?? []).map((outcome) => outcome.name);
    handleClose();
    openInstallModal(names);
  }
</script>

<Modal bind:open title={$t("import.title")} onClose={handleClose} containerClass="max-w-xl">
  <div class="flex h-full min-h-0 w-full flex-col">
    <div class="flex-1 p-6 pt-1">
      {#if importedOutcomes}
        <div class="space-y-4 py-4">
          <div class="text-success flex items-center gap-2 text-sm">
            <CheckCircle2 size={18} />
            <span class="text-base-content">
              {$t("import.done", { count: importedOutcomes.length })}
            </span>
          </div>
          <ul
            class="border-base-300 bg-base-200 max-h-48 space-y-1 overflow-y-auto rounded-xl border p-3 text-sm"
          >
            {#each importedOutcomes as outcome (outcome.name)}
              <li class="text-base-content flex items-center justify-between gap-3">
                <span class="truncate">{outcome.name}</span>
                <span class="text-base-content-faint font-mono text-[11px]">
                  {outcome.hash.slice(0, 8)}
                </span>
              </li>
            {/each}
          </ul>
        </div>
      {:else}
        <div class="sticky top-0 z-10 mb-4">
          <SegmentedTabs
            items={[
              { value: "github", label: $t("import.tab.github"), icon: Github },
              { value: "zip", label: $t("import.tab.zip"), icon: FileArchive },
              { value: "folder", label: $t("import.tab.folder"), icon: Folder },
            ]}
            value={activeTab}
            onChange={(tab) => (activeTab = tab as ImportTab)}
            fullWidth={true}
          />
        </div>

        {#if activeTab === "github"}
          <div class="space-y-4">
            <p class="text-base-content-muted text-sm">{$t("import.github.description")}</p>
            <div class="flex gap-2">
              <input
                type="text"
                class="border-base-300 bg-base-200 text-base-content placeholder:text-base-content-subtle focus:border-primary flex-1 rounded-xl border px-4 py-2 text-sm focus:outline-none"
                placeholder={$t("import.github.urlPlaceholder")}
                bind:value={githubUrl}
                onkeydown={(e) => e.key === "Enter" && handleDetectGithub()}
              />
              <PrimaryActionButton
                onclick={handleDetectGithub}
                disabled={!githubUrl.trim() || isDetectingGithub}
                loading={isDetectingGithub}
              >
                {$t("import.detect")}
              </PrimaryActionButton>
            </div>
            {#if githubError}
              <div class="text-error flex items-center gap-2 text-sm">
                <AlertCircle size={16} />
                <span>{githubError}</span>
              </div>
            {/if}
            {#if detectedGithubSkills.length > 0}
              <DetectedSkillList
                skills={detectedGithubSkills}
                selectedSkills={selectedGithubSkills}
                onToggle={(skill) =>
                  (selectedGithubSkills = toggleSkill(selectedGithubSkills, skill))}
                onToggleAll={() =>
                  (selectedGithubSkills = toggleAllSkills(
                    selectedGithubSkills,
                    detectedGithubSkills
                  ))}
                showHeader={true}
                headerLabel={$t("import.selectSkills")}
              />
            {/if}
          </div>
        {:else if activeTab === "zip"}
          <div class="space-y-3">
            <p class="text-base-content-muted text-sm">{$t("import.zip.description")}</p>
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
                  {$t("import.zip.clickToChange")}
                </p>
              {:else}
                <div class="text-base-content-muted flex flex-col items-center gap-1 text-sm">
                  <FileArchive size={24} />
                  <span>{$t("import.zip.selectFile")}</span>
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
                <span>{$t("import.detect")}</span>
              </div>
            {/if}
            {#if detectedZipSkills.length > 0}
              <DetectedSkillList
                skills={detectedZipSkills}
                selectedSkills={selectedZipSkills}
                onToggle={(skill) => (selectedZipSkills = toggleSkill(selectedZipSkills, skill))}
                onToggleAll={() =>
                  (selectedZipSkills = toggleAllSkills(selectedZipSkills, detectedZipSkills))}
                showHeader={true}
                headerLabel={$t("import.selectSkills")}
              />
            {/if}
          </div>
        {:else}
          <div class="space-y-3">
            <p class="text-base-content-muted text-sm">{$t("import.folder.description")}</p>
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
                <p
                  class="text-base-content-muted mt-1 truncate text-[11px]"
                  title={selectedFolderPath}
                >
                  {selectedFolderPath}
                </p>
              {:else}
                <div class="text-base-content-muted flex flex-col items-center gap-1 text-sm">
                  <Folder size={24} />
                  <span>{$t("import.folder.selectFolder")}</span>
                </div>
              {/if}
            </button>
            {#if folderError}
              <div class="text-error flex items-center gap-2 text-sm">
                <AlertCircle size={16} />
                <span>{folderError}</span>
              </div>
            {/if}
            {#if isScanning}
              <div class="text-base-content-muted flex items-center gap-2 text-sm">
                <Loader2 size={16} class="animate-spin" />
                <span>{$t("import.folder.scanning")}</span>
              </div>
            {/if}
            {#if scanItems.length > 0}
              <ScanResultList
                items={scanItems}
                bind:choices={scanChoices}
                rootPath={selectedFolderPath}
                disabled={isImporting}
              />
            {/if}
          </div>
        {/if}

        {#if activeTab !== "folder" && existingNames.length > 0}
          <label class="text-warning-content mt-4 flex cursor-pointer items-start gap-2 text-sm">
            <input type="checkbox" class="mt-0.5" bind:checked={overwriteExisting} />
            <span>{$t("import.overwriteExisting", { names: existingNames.join(", ") })}</span>
          </label>
        {/if}

        {#if importError}
          <div class="text-error mt-4 flex items-start gap-2 text-sm whitespace-pre-wrap">
            <AlertCircle size={16} class="mt-0.5 shrink-0" />
            <span>{importError}</span>
          </div>
        {/if}
      {/if}
    </div>
  </div>
  {#snippet footer()}
    {#if importedOutcomes}
      <button
        class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
        type="button"
        onclick={handleClose}
      >
        {$t("import.finish")}
      </button>
      {#if importedOutcomes.length > 0}
        <PrimaryActionButton onclick={handleInstallNow}>
          {$t("import.installNow")}
        </PrimaryActionButton>
      {/if}
    {:else}
      <PrimaryActionButton
        onclick={handleConfirm}
        disabled={!canConfirm}
        loading={isImporting}
        loadingText={$t("import.importing")}
        className="select-none"
      >
        {$t("import.confirm")}
      </PrimaryActionButton>
    {/if}
  {/snippet}
</Modal>
