<script lang="ts">
  import {
    AlertCircle,
    CheckCircle2,
    FilePlus2,
    FileText,
    Folder,
    Github,
    Loader2,
  } from "@lucide/svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import SegmentedTabs from "$lib/components/ui/SegmentedTabs.svelte";
  import DetectedInstructionList, {
    type DetectedChoice,
  } from "./instructions/DetectedInstructionList.svelte";
  import { t } from "../i18n";
  import {
    createInstruction,
    detectInstructionFiles,
    detectInstructionGithub,
    importInstructions,
    type DetectedInstruction,
    type InstructionImportOutcome,
  } from "../api/instructions";
  import { instructionsByName, refreshInstructions } from "../stores/instructions";
  import {
    closeImportInstructionModal,
    importInstructionModal,
    openInstallModal,
  } from "../stores/modals";

  type Tab = "github" | "local" | "new";

  let open = $state(false);
  let activeTab = $state<Tab>("github");

  // GitHub
  let githubUrl = $state("");
  let isDetectingGithub = $state(false);
  let githubItems = $state<DetectedInstruction[]>([]);
  let githubError = $state("");

  // Local files and folders
  let localItems = $state<DetectedInstruction[]>([]);
  let isDetectingLocal = $state(false);
  let localError = $state("");
  let isDragOver = $state(false);
  let suppressClick = $state(false);

  /** Selection and name per detected file, keyed by path. */
  let choices = $state<Record<string, DetectedChoice>>({});

  // New
  let newName = $state("");
  let newContent = $state("");

  // Import
  let isImporting = $state(false);
  let importError = $state("");
  let overwriteExisting = $state(false);
  let importedOutcomes = $state<InstructionImportOutcome[] | null>(null);
  let unlistenNativeDragDrop: (() => void) | null = null;

  const items = $derived(
    activeTab === "github" ? githubItems : activeTab === "local" ? localItems : []
  );
  const selectedItems = $derived(items.filter((item) => choices[item.path]?.selected));
  const existingNames = $derived(
    new Set([...$instructionsByName.keys()].map((name) => name.toLowerCase()))
  );
  const chosenName = (item: DetectedInstruction) =>
    (choices[item.path]?.name ?? item.name).trim() || item.name;
  const overwriteNames = $derived(
    selectedItems.map(chosenName).filter((name) => existingNames.has(name.toLowerCase()))
  );
  const canConfirm = $derived.by(() => {
    if (isImporting) return false;
    if (activeTab === "new") return Boolean(newName.trim());
    if (selectedItems.length === 0) return false;
    return overwriteNames.length === 0 || overwriteExisting;
  });

  $effect(() => {
    const state = $importInstructionModal;
    if (state.open && !open) resetState();
    open = state.open;
  });

  function resetState() {
    activeTab = "github";
    githubUrl = "";
    isDetectingGithub = false;
    githubItems = [];
    githubError = "";
    localItems = [];
    isDetectingLocal = false;
    localError = "";
    isDragOver = false;
    suppressClick = false;
    choices = {};
    newName = "";
    newContent = "";
    isImporting = false;
    importError = "";
    overwriteExisting = false;
    importedOutcomes = null;
  }

  function handleClose() {
    open = false;
    closeImportInstructionModal();
  }

  function addItems(found: DetectedInstruction[], target: "github" | "local") {
    const next = { ...choices };
    for (const item of found) {
      next[item.path] = next[item.path] ?? { selected: true, name: item.name };
    }
    choices = next;
    if (target === "github") {
      githubItems = found;
      return;
    }
    const known = new Set(localItems.map((item) => item.path));
    localItems = [...localItems, ...found.filter((item) => !known.has(item.path))];
  }

  // ---------------------------------------------------------------------------
  // Drag & drop (Tauri native events + HTML5 fallback)
  // ---------------------------------------------------------------------------

  $effect(() => {
    if (!open) return;
    let disposed = false;
    const onWindowDragOver = (event: DragEvent) => {
      event.preventDefault();
      if (activeTab === "local") isDragOver = true;
    };
    const onWindowDragLeave = () => (isDragOver = false);
    const onWindowDrop = (event: DragEvent) => {
      event.preventDefault();
      isDragOver = false;
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
            if (activeTab === "local") isDragOver = true;
            return;
          }
          if (payload.type === "leave") {
            isDragOver = false;
            return;
          }
          if (payload.type === "drop") {
            isDragOver = false;
            const dropped = payload.paths ?? [];
            if (dropped.length > 0 && activeTab === "local") void addLocalPaths(dropped);
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
      isDragOver = false;
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

  function extractPathsFromTransfer(dt: DataTransfer | null): string[] {
    if (!dt) return [];
    const fromFiles = [
      ...Array.from(dt.files || []),
      ...Array.from(dt.items || []).map((item) => item.getAsFile?.() ?? null),
    ]
      .map((file) => (file as (File & { path?: string }) | null)?.path)
      .filter((path): path is string => Boolean(path));
    if (fromFiles.length > 0) return [...new Set(fromFiles)];
    const raw = dt.getData("text/uri-list") || dt.getData("text/plain");
    return raw
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith("#"))
      .map((line) => pathFromUri(line) ?? line.replace(/^['"]|['"]$/g, ""));
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    isDragOver = false;
    suppressClick = true;
    setTimeout(() => (suppressClick = false), 100);
    const paths = extractPathsFromTransfer(event.dataTransfer);
    if (paths.length > 0) void addLocalPaths(paths);
    else localError = $t("instructions.import.local.dropReadError");
  }

  // ---------------------------------------------------------------------------
  // GitHub
  // ---------------------------------------------------------------------------

  function parseGithubInput(value: string): string | null {
    const trimmed = value.trim();
    if (!trimmed) return null;
    if (/^https?:\/\//i.test(trimmed)) return trimmed;
    if (/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(trimmed)) return trimmed;
    return null;
  }

  async function handleDetectGithub() {
    const githubPath = parseGithubInput(githubUrl);
    if (!githubPath) {
      githubError = "Unsupported URL format. Use http(s) URL or owner/repo.";
      githubItems = [];
      return;
    }
    isDetectingGithub = true;
    githubError = "";
    githubItems = [];
    try {
      const found = await detectInstructionGithub(githubPath);
      addItems(found, "github");
      if (found.length === 0) githubError = $t("instructions.import.noFilesFound");
    } catch (error) {
      githubError = String(error);
    } finally {
      isDetectingGithub = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Local
  // ---------------------------------------------------------------------------

  async function handleSelectFiles() {
    if (suppressClick) return;
    try {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const result = await openDialog({
        multiple: true,
        directory: false,
        filters: [{ name: "Markdown", extensions: ["md", "markdown", "mdc"] }],
      });
      const paths = Array.isArray(result) ? result : result ? [result] : [];
      if (paths.length > 0) await addLocalPaths(paths);
    } catch (error) {
      console.error("Failed to select files:", error);
    }
  }

  async function handleSelectFolder() {
    try {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const result = await openDialog({ multiple: false, directory: true });
      if (typeof result === "string") await addLocalPaths([result]);
    } catch (error) {
      console.error("Failed to select folder:", error);
    }
  }

  async function addLocalPaths(raw: string[]) {
    const paths = raw
      .map((path) => (path.trim().startsWith("file://") ? pathFromUri(path.trim()) : path.trim()))
      .filter((path): path is string => Boolean(path));
    if (paths.length === 0) {
      localError = $t("instructions.import.local.dropReadError");
      return;
    }
    isDetectingLocal = true;
    localError = "";
    try {
      const found = await detectInstructionFiles(paths);
      addItems(found, "local");
      if (found.length === 0) localError = $t("instructions.import.noFilesFound");
    } catch (error) {
      localError = String(error);
    } finally {
      isDetectingLocal = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Confirm
  // ---------------------------------------------------------------------------

  function duplicateNames(): string[] {
    const seen = new Set<string>();
    const dupes = new Set<string>();
    for (const item of selectedItems) {
      const name = chosenName(item).toLowerCase();
      if (seen.has(name)) dupes.add(name);
      else seen.add(name);
    }
    return [...dupes];
  }

  async function handleConfirm() {
    if (!canConfirm) return;
    importError = "";
    if (activeTab !== "new") {
      const dupes = duplicateNames();
      if (dupes.length > 0) {
        importError = $t("instructions.import.duplicateNames", { names: dupes.join(", ") });
        return;
      }
    }
    isImporting = true;
    try {
      let outcomes: InstructionImportOutcome[];
      if (activeTab === "new") {
        outcomes = [await createInstruction(newName.trim(), newContent)];
      } else {
        outcomes = await importInstructions(
          selectedItems.map((item) => ({
            name: chosenName(item),
            path: item.path,
            source: item.source ?? null,
          })),
          overwriteExisting
        );
      }
      await refreshInstructions();
      importedOutcomes = outcomes;
    } catch (error) {
      importError = String(error);
      await refreshInstructions().catch(console.error);
    } finally {
      isImporting = false;
    }
  }

  function handleInstallNow() {
    const names = (importedOutcomes ?? []).map((outcome) => outcome.name);
    handleClose();
    openInstallModal(names, { kind: "instruction" });
  }

  const inputClass =
    "border-base-300 bg-base-200 text-base-content placeholder:text-base-content-subtle focus:border-primary w-full rounded-xl border px-4 py-2 text-sm focus:outline-none";
</script>

<Modal
  bind:open
  title={$t("instructions.import.title")}
  onClose={handleClose}
  containerClass="max-w-xl"
>
  <div class="flex h-full min-h-0 w-full flex-col">
    <div class="flex-1 p-6 pt-1">
      {#if importedOutcomes}
        <div class="space-y-4 py-4">
          <div class="text-success flex items-center gap-2 text-sm">
            <CheckCircle2 size={18} />
            <span class="text-base-content">
              {$t("instructions.import.done", { count: importedOutcomes.length })}
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
              { value: "github", label: $t("instructions.import.tab.github"), icon: Github },
              { value: "local", label: $t("instructions.import.tab.local"), icon: Folder },
              { value: "new", label: $t("instructions.import.tab.new"), icon: FilePlus2 },
            ]}
            value={activeTab}
            onChange={(tab) => (activeTab = tab as Tab)}
            fullWidth={true}
          />
        </div>

        {#if activeTab === "github"}
          <div class="space-y-4">
            <p class="text-base-content-muted text-sm">
              {$t("instructions.import.github.description")}
            </p>
            <div class="flex gap-2">
              <input
                type="text"
                class={`${inputClass} flex-1`}
                placeholder={$t("import.github.urlPlaceholder")}
                bind:value={githubUrl}
                onkeydown={(event) => event.key === "Enter" && handleDetectGithub()}
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
            {#if githubItems.length > 0}
              <DetectedInstructionList
                items={githubItems}
                bind:choices
                {existingNames}
                disabled={isImporting}
              />
            {/if}
          </div>
        {:else if activeTab === "local"}
          <div class="space-y-3">
            <p class="text-base-content-muted text-sm">
              {$t("instructions.import.local.description")}
            </p>
            <button
              class={`w-full rounded-xl border-2 border-dashed p-3 transition ${
                isDragOver
                  ? "border-primary bg-base-200"
                  : "border-base-300 hover:border-primary hover:bg-base-200"
              }`}
              onclick={handleSelectFiles}
              ondragover={(event) => event.preventDefault()}
              ondragenter={() => (isDragOver = true)}
              ondragleave={() => (isDragOver = false)}
              ondrop={handleDrop}
              type="button"
            >
              <div class="text-base-content-muted flex flex-col items-center gap-1 text-sm">
                <FileText size={24} />
                <span>{$t("instructions.import.local.select")}</span>
              </div>
            </button>
            <div class="flex items-center justify-between gap-3">
              <button
                class="border-base-300 text-base-content hover:bg-base-200 flex h-8 items-center gap-1.5 rounded-lg border px-3 text-[13px] transition disabled:opacity-50"
                type="button"
                onclick={handleSelectFolder}
                disabled={isDetectingLocal || isImporting}
              >
                <Folder size={14} />
                {$t("instructions.import.local.selectFolder")}
              </button>
              {#if isDetectingLocal}
                <span class="text-base-content-muted flex items-center gap-2 text-sm">
                  <Loader2 size={16} class="animate-spin" />
                  {$t("instructions.import.detecting")}
                </span>
              {/if}
            </div>
            {#if localError}
              <div class="text-error flex items-center gap-2 text-sm">
                <AlertCircle size={16} />
                <span>{localError}</span>
              </div>
            {/if}
            {#if localItems.length > 0}
              <DetectedInstructionList
                items={localItems}
                bind:choices
                {existingNames}
                disabled={isImporting}
              />
            {/if}
          </div>
        {:else}
          <div class="space-y-3">
            <p class="text-base-content-muted text-sm">
              {$t("instructions.import.new.description")}
            </p>
            <label class="block space-y-1 text-sm">
              <span class="text-base-content-muted text-[13px]">
                {$t("instructions.import.name")}
              </span>
              <input
                type="text"
                class={inputClass}
                placeholder={$t("instructions.import.namePlaceholder")}
                bind:value={newName}
                disabled={isImporting}
              />
            </label>
            <label class="block space-y-1 text-sm">
              <span class="text-base-content-muted text-[13px]">
                {$t("instructions.import.new.content")}
              </span>
              <textarea
                class={`${inputClass} min-h-32 resize-y font-mono text-[13px]`}
                bind:value={newContent}
                disabled={isImporting}
              ></textarea>
            </label>
          </div>
        {/if}

        {#if activeTab !== "new" && overwriteNames.length > 0}
          <label class="text-warning-content mt-4 flex cursor-pointer items-start gap-2 text-sm">
            <input type="checkbox" class="mt-0.5" bind:checked={overwriteExisting} />
            <span>
              {$t("instructions.import.overwriteExisting", { names: overwriteNames.join(", ") })}
            </span>
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
        {$t("common.close")}
      </button>
      <PrimaryActionButton onclick={handleInstallNow}>
        {$t("instructions.import.installNow")}
      </PrimaryActionButton>
    {:else}
      <button
        class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
        type="button"
        onclick={handleClose}
        disabled={isImporting}
      >
        {$t("common.cancel")}
      </button>
      <PrimaryActionButton
        onclick={handleConfirm}
        disabled={!canConfirm}
        loading={isImporting}
        loadingText={$t("import.importing")}
      >
        {activeTab === "new" ? $t("instructions.import.create") : $t("import.confirm")}
      </PrimaryActionButton>
    {/if}
  {/snippet}
</Modal>
