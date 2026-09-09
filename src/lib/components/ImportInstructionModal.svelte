<script lang="ts">
  import {
    AlertCircle,
    CheckCircle2,
    FilePlus2,
    FileText,
    Loader2,
    ScanSearch,
  } from "@lucide/svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import SegmentedTabs from "$lib/components/ui/SegmentedTabs.svelte";
  import InstructionScanList, {
    type InstructionScanChoice,
  } from "./instructions/InstructionScanList.svelte";
  import { t } from "../i18n";
  import {
    createInstruction,
    importInstructions,
    importScannedInstructions,
    scanInstructionFiles,
    type InstructionImportOutcome,
    type InstructionScanDecision,
    type InstructionScanItem,
  } from "../api/instructions";
  import { instructionsByName, refreshInstructions } from "../stores/instructions";
  import {
    closeImportInstructionModal,
    importInstructionModal,
    openInstallModal,
  } from "../stores/modals";

  type Tab = "file" | "scan" | "new";

  let open = $state(false);
  let activeTab = $state<Tab>("file");

  // File
  let filePath = $state("");
  let fileName = $state("");
  let name = $state("");
  let fileError = $state("");
  let isDragOver = $state(false);
  let suppressClick = $state(false);
  let overwriteExisting = $state(false);

  // Scan
  let scanned = $state(false);
  let isScanning = $state(false);
  let scanItems = $state<InstructionScanItem[]>([]);
  let scanChoices = $state<Record<string, InstructionScanChoice>>({});
  let scanError = $state("");

  // New
  let newName = $state("");
  let newContent = $state("");

  // Import
  let isImporting = $state(false);
  let importError = $state("");
  let importedOutcomes = $state<InstructionImportOutcome[] | null>(null);
  let unlistenNativeDragDrop: (() => void) | null = null;

  const existingName = $derived.by(() => {
    const wanted = name.trim().toLowerCase();
    if (!wanted) return "";
    for (const key of $instructionsByName.keys()) {
      if (key.toLowerCase() === wanted) return key;
    }
    return "";
  });
  const selectedScanCount = $derived(
    scanItems.filter((item) => scanChoices[item.path]?.selected).length
  );
  const canConfirm = $derived.by(() => {
    if (isImporting) return false;
    switch (activeTab) {
      case "file":
        return Boolean(filePath) && Boolean(name.trim()) && (!existingName || overwriteExisting);
      case "scan":
        return selectedScanCount > 0;
      default:
        return Boolean(newName.trim());
    }
  });

  $effect(() => {
    const state = $importInstructionModal;
    if (state.open && !open) resetState();
    open = state.open;
  });

  // The agent locations are scanned the first time the tab is shown.
  $effect(() => {
    if (open && activeTab === "scan" && !scanned) void runScan();
  });

  function resetState() {
    activeTab = "file";
    filePath = "";
    fileName = "";
    name = "";
    fileError = "";
    isDragOver = false;
    suppressClick = false;
    overwriteExisting = false;
    scanned = false;
    isScanning = false;
    scanItems = [];
    scanChoices = {};
    scanError = "";
    newName = "";
    newContent = "";
    isImporting = false;
    importError = "";
    importedOutcomes = null;
  }

  function handleClose() {
    open = false;
    closeImportInstructionModal();
  }

  // ---------------------------------------------------------------------------
  // Drag & drop (Tauri native events + HTML5 fallback)
  // ---------------------------------------------------------------------------

  $effect(() => {
    if (!open) return;
    let disposed = false;
    const onWindowDragOver = (event: DragEvent) => {
      event.preventDefault();
      if (activeTab === "file") isDragOver = true;
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
            if (activeTab === "file") isDragOver = true;
            return;
          }
          if (payload.type === "leave") {
            isDragOver = false;
            return;
          }
          if (payload.type === "drop") {
            isDragOver = false;
            const dropped = payload.paths?.[0] ?? "";
            if (dropped && activeTab === "file") applyFilePath(dropped);
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
    const raw = dt.getData("text/uri-list") || dt.getData("text/plain");
    const firstLine = raw
      .split("\n")
      .map((line) => line.trim())
      .find((line) => line && !line.startsWith("#"));
    if (!firstLine) return null;
    return pathFromUri(firstLine) ?? firstLine.replace(/^['"]|['"]$/g, "");
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    isDragOver = false;
    suppressClick = true;
    setTimeout(() => (suppressClick = false), 100);
    const path = extractPathFromTransfer(event.dataTransfer);
    if (path) applyFilePath(path);
    else fileError = $t("instructions.import.file.dropReadError");
  }

  // ---------------------------------------------------------------------------
  // File
  // ---------------------------------------------------------------------------

  async function handleSelectFile() {
    if (suppressClick) return;
    try {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const result = await openDialog({
        multiple: false,
        directory: false,
        filters: [{ name: "Markdown", extensions: ["md", "markdown", "txt"] }],
      });
      if (result) applyFilePath(result);
    } catch (error) {
      console.error("Failed to select file:", error);
    }
  }

  const baseName = (path: string) => path.split(/[/\\]/).filter(Boolean).pop() || "";

  /** `~/.claude/CLAUDE.md` → `claude`, `<project>/AGENTS.md` → `<project>`, `notes.md` → `notes`. */
  function suggestName(path: string): string {
    const parts = path.split(/[/\\]/).filter(Boolean);
    const file = parts.pop() ?? "";
    const stem = file.replace(/\.[^.]+$/, "");
    const base = /^[A-Z0-9_-]+$/.test(stem) ? (parts.pop() ?? stem).replace(/^\.+/, "") : stem;
    return base
      .toLowerCase()
      .replace(/[^\w.-]+/g, "-")
      .replace(/^[-.]+|[-.]+$/g, "");
  }

  function applyFilePath(path: string) {
    const normalized = path.trim().startsWith("file://")
      ? (pathFromUri(path.trim()) ?? "")
      : path.trim();
    if (!normalized) {
      fileError = $t("instructions.import.file.dropReadError");
      return;
    }
    if (!/\.(md|markdown|txt)$/i.test(normalized)) {
      fileError = $t("instructions.import.invalidFile");
      return;
    }
    fileError = "";
    filePath = normalized;
    fileName = baseName(normalized);
    name = suggestName(normalized);
    overwriteExisting = false;
  }

  // ---------------------------------------------------------------------------
  // Scan
  // ---------------------------------------------------------------------------

  function defaultChoice(item: InstructionScanItem): InstructionScanChoice {
    const base = { name: item.name, registerInstall: true };
    switch (item.status) {
      case "new":
        return { ...base, selected: true, resolution: "import" };
      case "identical":
        return { ...base, selected: !item.registered, resolution: "import" };
      case "different":
        return { ...base, selected: true, resolution: "adopt_into_hub" };
      default:
        return { ...base, selected: false, resolution: "skip", registerInstall: false };
    }
  }

  async function runScan() {
    scanned = true;
    isScanning = true;
    scanError = "";
    try {
      const items = await scanInstructionFiles();
      scanItems = items;
      const choices: Record<string, InstructionScanChoice> = {};
      for (const item of items) choices[item.path] = defaultChoice(item);
      scanChoices = choices;
    } catch (error) {
      scanError = String(error);
    } finally {
      isScanning = false;
    }
  }

  function buildScanDecisions(): InstructionScanDecision[] {
    return scanItems
      .filter((item) => scanChoices[item.path]?.selected)
      .map((item) => {
        const choice = scanChoices[item.path];
        return {
          name: choice.name.trim() || item.name,
          path: item.path,
          resolution: choice.resolution,
          registerInstall: choice.registerInstall,
          hubName: item.hubName ?? null,
        };
      });
  }

  // ---------------------------------------------------------------------------
  // Confirm
  // ---------------------------------------------------------------------------

  async function handleConfirm() {
    if (!canConfirm) return;
    importError = "";
    isImporting = true;
    try {
      let outcomes: InstructionImportOutcome[];
      if (activeTab === "file") {
        outcomes = await importInstructions(
          [{ name: name.trim(), path: filePath }],
          overwriteExisting
        );
      } else if (activeTab === "scan") {
        outcomes = await importScannedInstructions(buildScanDecisions());
      } else {
        outcomes = [await createInstruction(newName.trim(), newContent)];
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
              { value: "file", label: $t("instructions.import.tab.file"), icon: FileText },
              { value: "scan", label: $t("instructions.import.tab.scan"), icon: ScanSearch },
              { value: "new", label: $t("instructions.import.tab.new"), icon: FilePlus2 },
            ]}
            value={activeTab}
            onChange={(tab) => (activeTab = tab as Tab)}
            fullWidth={true}
          />
        </div>

        {#if activeTab === "file"}
          <div class="space-y-3">
            <p class="text-base-content-muted text-sm">
              {$t("instructions.import.file.description")}
            </p>
            <button
              class={`w-full rounded-xl border-2 border-dashed p-3 transition ${
                isDragOver
                  ? "border-primary bg-base-200"
                  : "border-base-300 hover:border-primary hover:bg-base-200"
              }`}
              onclick={handleSelectFile}
              ondragover={(event) => event.preventDefault()}
              ondragenter={() => (isDragOver = true)}
              ondragleave={() => (isDragOver = false)}
              ondrop={handleDrop}
              type="button"
            >
              {#if filePath}
                <div class="text-base-content flex items-center justify-center gap-2 text-sm">
                  <FileText size={16} class="text-primary" />
                  <span class="font-medium">{fileName}</span>
                </div>
                <p class="text-base-content-muted mt-1 truncate text-[11px]" title={filePath}>
                  {filePath}
                </p>
              {:else}
                <div class="text-base-content-muted flex flex-col items-center gap-1 text-sm">
                  <FileText size={24} />
                  <span>{$t("instructions.import.file.select")}</span>
                </div>
              {/if}
            </button>
            {#if fileError}
              <div class="text-error flex items-center gap-2 text-sm">
                <AlertCircle size={16} />
                <span>{fileError}</span>
              </div>
            {/if}
            {#if filePath}
              <label class="block space-y-1 text-sm">
                <span class="text-base-content-muted text-[13px]">
                  {$t("instructions.import.name")}
                </span>
                <input
                  type="text"
                  class={inputClass}
                  placeholder={$t("instructions.import.namePlaceholder")}
                  bind:value={name}
                  disabled={isImporting}
                />
              </label>
              {#if existingName}
                <label class="text-warning-content flex cursor-pointer items-start gap-2 text-sm">
                  <input type="checkbox" class="mt-0.5" bind:checked={overwriteExisting} />
                  <span>
                    {$t("instructions.import.overwriteExisting", { name: existingName })}
                  </span>
                </label>
              {/if}
            {/if}
          </div>
        {:else if activeTab === "scan"}
          <div class="space-y-3">
            <p class="text-base-content-muted text-sm">
              {$t("instructions.import.scan.description")}
            </p>
            {#if scanError}
              <div class="text-error flex items-center gap-2 text-sm">
                <AlertCircle size={16} />
                <span>{scanError}</span>
              </div>
            {/if}
            {#if isScanning}
              <div class="text-base-content-muted flex items-center gap-2 text-sm">
                <Loader2 size={16} class="animate-spin" />
                <span>{$t("instructions.import.scan.scanning")}</span>
              </div>
            {:else if scanItems.length === 0 && !scanError}
              <p class="text-base-content-muted text-sm">{$t("instructions.import.scan.empty")}</p>
            {:else if scanItems.length > 0}
              <InstructionScanList
                items={scanItems}
                bind:choices={scanChoices}
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
      <PrimaryActionButton onclick={handleConfirm} disabled={!canConfirm} loading={isImporting}>
        {activeTab === "new" ? $t("instructions.import.create") : $t("import.confirm")}
      </PrimaryActionButton>
    {/if}
  {/snippet}
</Modal>
