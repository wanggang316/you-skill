<script>
  import { FileArchive, Github, AlertCircle, Loader2, Folder } from "@lucide/svelte";
  import { get } from "svelte/store";
  import { t } from "../i18n";
  import {
    detectZip,
    detectGithubManual,
    detectFolder,
    installFromNative,
    installFromGithub,
  } from "../api/skills";
  import { listUserProjects } from "../api/user-projects";
  import Modal from "./ui/Modal.svelte";
  import PrimaryActionButton from "./ui/PrimaryActionButton.svelte";
  import SegmentedTabs from "./ui/SegmentedTabs.svelte";
  import SelectField from "./ui/SelectField.svelte";
  import AgentSelector from "./AgentSelector.svelte";
  import DetectedSkillList from "./DetectedSkillList.svelte";
  import { settings } from "../stores/settings";

  /** @type {{ open?: boolean; agents?: import('../api/skills').AgentInfo[]; initialScope?: "global" | "project"; initialProjectPath?: string | null; onSuccess?: () => void }} */
  let {
    open = $bindable(false),
    agents = [],
    initialScope = "global",
    initialProjectPath = null,
    onSuccess = () => {},
  } = $props();

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
  /** @type {"global" | "project"} */
  let selectedScope = $state("global");
  /** @type {import('../api/user-projects').UserProject[]} */
  let userProjects = $state([]);
  /** @type {string | null} */
  let selectedProjectPath = $state(null);
  let isZipDragOver = $state(false);
  let isFolderDragOver = $state(false);
  let suppressZipClick = $state(false);
  let suppressFolderClick = $state(false);
  /** @type {null | (() => void)} */
  let unlistenNativeDragDrop = null;

  // Global loading state
  let isInstalling = $state(false);
  let installError = $state("");
  let wasOpen = $state(false);

  // Reset state when modal opens
  $effect(() => {
    if (open && !wasOpen) {
      resetState();
      void loadUserProjects();
    }
    wasOpen = open;
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
    selectedMethod = get(settings).sync_mode || "symlink";
    selectedScope = initialScope;
    userProjects = [];
    selectedProjectPath = initialScope === "project" ? initialProjectPath : null;
    suppressZipClick = false;
    suppressFolderClick = false;
    isInstalling = false;
    installError = "";
  }

  async function loadUserProjects() {
    try {
      userProjects = await listUserProjects();
    } catch {
      userProjects = [];
    }
  }

  const scopeLabel = $derived.by(() => {
    if (selectedScope !== "project") return $t("installScope.global");
    if (!selectedProjectPath) return $t("installScope.project");
    return (
      userProjects.find((project) => project.path === selectedProjectPath)?.name ??
      $t("installScope.project")
    );
  });

  function closeModal() {
    open = false;
  }

  /**
   * @param {"zip" | "folder"} type
   * @param {string} path
   * @param {string} [displayName]
   */
  async function applyDroppedPath(type, path, displayName = "") {
    if (type === "zip") {
      await applyDroppedZipPath(path, displayName);
    } else {
      await applyDroppedFolderPath(path, displayName);
    }
  }

  // Use Tauri native drag-drop events as a fallback because some webview environments
  // do not dispatch HTML5 drop events to DOM nodes reliably.
  $effect(() => {
    if (!open) return;
    let disposed = false;
    /** @param {DragEvent} event */
    const onWindowDragOver = (event) => {
      event.preventDefault();
      if (activeTab === "zip") isZipDragOver = true;
      if (activeTab === "folder") isFolderDragOver = true;
    };
    const onWindowDragLeave = () => {
      isZipDragOver = false;
      isFolderDragOver = false;
    };
    /** @param {DragEvent} event */
    const onWindowDrop = (event) => {
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
            if (activeTab !== "zip" && activeTab !== "folder") return;
            const droppedPath = payload.paths && payload.paths.length > 0 ? payload.paths[0] : "";
            if (!droppedPath) return;
            const name = droppedPath.split(/[/\\]/).pop() || "";
            void applyDroppedPath(activeTab, droppedPath, name);
          }
        });

        if (disposed) {
          unlisten();
          return;
        }
        unlistenNativeDragDrop = unlisten;
      } catch {
        // no-op: likely running in browser preview
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
      if (unlistenNativeDragDrop) {
        unlistenNativeDragDrop();
        unlistenNativeDragDrop = null;
      }
    };
  });

  /** @param {string} url */
  function toGitRepoUrl(url) {
    const trimmed = url.trim();
    return trimmed.endsWith(".git") ? trimmed : `${trimmed}.git`;
  }

  /** @param {string} uri */
  function pathFromUri(uri) {
    if (!uri.startsWith("file://")) return null;
    try {
      return decodeURIComponent(new URL(uri).pathname);
    } catch {
      return null;
    }
  }

  /** @param {string} raw */
  function pathFromDropText(raw) {
    const firstLine = raw
      .split("\n")
      .map((line) => line.trim())
      .find((line) => line && !line.startsWith("#"));
    if (!firstLine) return null;

    const uriPath = pathFromUri(firstLine);
    if (uriPath) return uriPath;

    // Some platforms provide plain absolute paths in text/plain instead of file:// URIs.
    const unquoted = firstLine.replace(/^['"]|['"]$/g, "");
    return unquoted || null;
  }

  /** @param {string} path */
  function normalizeDroppedPath(path) {
    const normalized = path.trim();
    if (!normalized) return "";
    if (normalized.startsWith("file://")) {
      return pathFromUri(normalized) || "";
    }
    return normalized;
  }

  /**
   * @param {DataTransfer | null} dt
   * @returns {string}
   */
  function buildDropDebugInfo(dt) {
    if (!dt) return "dataTransfer=null";

    const filesLen = dt.files?.length || 0;
    const items = Array.from(dt.items || []);
    const kinds = items.map((i) => i.kind).join(",");
    const types = items.map((i) => i.type || "(empty)").join(",");
    const textPlain = (dt.getData("text/plain") || "").trim().slice(0, 200);
    const uriList = (dt.getData("text/uri-list") || "").trim().slice(0, 200);

    /** @type {string[]} */
    const filePaths = [];
    for (const file of Array.from(dt.files || [])) {
      const fileWithPath = /** @type {{ path?: string }} */ (file);
      if (fileWithPath.path) filePaths.push(fileWithPath.path);
    }

    return [
      `files=${filesLen}`,
      `items=${items.length}`,
      `kinds=[${kinds}]`,
      `types=[${types}]`,
      `paths=[${filePaths.join(" | ")}]`,
      `textPlain="${textPlain}"`,
      `uriList="${uriList}"`,
    ].join("; ");
  }

  /**
   * @param {File} file
   * @returns {Promise<string>}
   */
  async function persistDroppedZipFile(file) {
    const { mkdir, writeFile, BaseDirectory } = await import("@tauri-apps/plugin-fs");
    const { appLocalDataDir, join } = await import("@tauri-apps/api/path");

    const folder = "dropped-skills";
    await mkdir(folder, { baseDir: BaseDirectory.AppLocalData, recursive: true });

    const safeName = (file.name || "dropped.skill").replace(/[^\w.-]/g, "_");
    const uniqueName = `${Date.now()}-${Math.random().toString(36).slice(2, 10)}-${safeName}`;
    const relativePath = `${folder}/${uniqueName}`;
    const bytes = new Uint8Array(await file.arrayBuffer());

    await writeFile(relativePath, bytes, { baseDir: BaseDirectory.AppLocalData });

    const baseDir = await appLocalDataDir();
    return await join(baseDir, relativePath);
  }

  /**
   * @param {DataTransfer | null | undefined} dt
   * @returns {File | null}
   */
  function firstDroppedFile(dt) {
    return dt?.files && dt.files.length > 0 ? dt.files[0] : null;
  }

  /**
   * @param {DataTransfer | null | undefined} dt
   * @returns {any | null}
   */
  function droppedDirectoryEntry(dt) {
    if (!dt) return null;
    for (const item of Array.from(dt.items || [])) {
      const entry = /** @type {{ webkitGetAsEntry?: () => any }} */ (item).webkitGetAsEntry?.();
      if (entry?.isDirectory) return entry;
    }
    return null;
  }

  /** @param {any} entry */
  function readFileFromEntry(entry) {
    return new Promise((resolve, reject) => entry.file(resolve, reject));
  }

  /** @param {any} dirEntry */
  function readAllDirectoryEntries(dirEntry) {
    return new Promise((resolve, reject) => {
      const reader = dirEntry.createReader();
      /** @type {any[]} */
      const all = [];
      const readChunk = () => {
        reader.readEntries(
          /** @param {any[]} entries */
          (entries) => {
            if (!entries.length) {
              resolve(all);
              return;
            }
            all.push(...entries);
            readChunk();
          },
          /** @param {unknown} error */
          (error) => reject(error)
        );
      };
      readChunk();
    });
  }

  /**
   * @param {any} dirEntry
   * @param {string} basePath
   * @returns {Promise<Array<{ relativePath: string; file: File }>>}
   */
  async function collectDroppedDirectoryFiles(dirEntry, basePath = "") {
    /** @type {Array<{ relativePath: string; file: File }>} */
    const result = [];
    const entries = await readAllDirectoryEntries(dirEntry);

    for (const entry of entries) {
      const relativePath = basePath ? `${basePath}/${entry.name}` : entry.name;
      if (entry.isDirectory) {
        const nested = await collectDroppedDirectoryFiles(entry, relativePath);
        result.push(...nested);
      } else if (entry.isFile) {
        const file = await readFileFromEntry(entry);
        result.push({ relativePath, file });
      }
    }

    return result;
  }

  /** @param {string} path */
  function sanitizeRelativePath(path) {
    return path
      .replace(/\\/g, "/")
      .split("/")
      .filter((part) => part && part !== "." && part !== "..")
      .join("/");
  }

  /**
   * @param {any} dirEntry
   * @returns {Promise<string>}
   */
  async function persistDroppedFolderEntry(dirEntry) {
    const { mkdir, writeFile, BaseDirectory } = await import("@tauri-apps/plugin-fs");
    const { appLocalDataDir, join } = await import("@tauri-apps/api/path");

    const folder = "dropped-folders";
    await mkdir(folder, { baseDir: BaseDirectory.AppLocalData, recursive: true });

    const safeName = (dirEntry.name || "dropped-folder").replace(/[^\w.-]/g, "_");
    const uniqueRoot = `${Date.now()}-${Math.random().toString(36).slice(2, 10)}-${safeName}`;
    const relativeRoot = `${folder}/${uniqueRoot}`;
    await mkdir(relativeRoot, { baseDir: BaseDirectory.AppLocalData, recursive: true });

    const files = await collectDroppedDirectoryFiles(dirEntry);
    for (const { relativePath, file } of files) {
      const cleanRelativePath = sanitizeRelativePath(relativePath);
      if (!cleanRelativePath) continue;

      const outputRelativePath = `${relativeRoot}/${cleanRelativePath}`;
      const outputRelativeDir = parentDirectory(outputRelativePath);
      await mkdir(outputRelativeDir, { baseDir: BaseDirectory.AppLocalData, recursive: true });
      await writeFile(outputRelativePath, new Uint8Array(await file.arrayBuffer()), {
        baseDir: BaseDirectory.AppLocalData,
      });
    }

    const baseDir = await appLocalDataDir();
    return await join(baseDir, relativeRoot);
  }

  /**
   * @param {"zip" | "folder"} type
   * @param {DataTransfer | null | undefined} dt
   * @param {unknown} [error]
   */
  function reportDropFailure(type, dt, error) {
    const debug = buildDropDebugInfo(dt || null);
    if (type === "zip") {
      console.error("[AddSkillModal] ZIP drop failed:", error || "", debug);
      zipError = $t("addSkill.zip.dropReadError");
      return;
    }
    console.error("[AddSkillModal] Folder drop failed:", error || "", debug);
    folderError = $t("addSkill.folder.dropReadError");
  }

  /** @param {string} path */
  function parentDirectory(path) {
    return path.replace(/[/\\][^/\\]+$/, "");
  }

  /**
   * @param {string[]} paths
   * @returns {string}
   */
  function commonDirectory(paths) {
    if (paths.length === 0) return "";
    if (paths.length === 1) return parentDirectory(paths[0]);

    const splitPaths = paths.map((p) => p.replace(/\\/g, "/").split("/"));
    let common = splitPaths[0];

    for (let i = 1; i < splitPaths.length; i += 1) {
      const current = splitPaths[i];
      let j = 0;
      while (j < common.length && j < current.length && common[j] === current[j]) {
        j += 1;
      }
      common = common.slice(0, j);
      if (common.length === 0) return "";
    }

    return common.join("/");
  }

  async function handleSelectZipFile() {
    if (suppressZipClick) return;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "Skill Files", extensions: ["skill", "zip"] }],
      });
      if (result) {
        await applyDroppedZipPath(result, result.split(/[/\\]/).pop() || "");
      }
    } catch (error) {
      console.error("Failed to select zip file:", error);
    }
  }

  /**
   * @param {DragEvent} event
   * @returns {string | null}
   */
  function extractZipPathFromDrop(event) {
    return extractPathFromTransfer(event.dataTransfer);
  }

  /**
   * @param {DataTransfer | null} dt
   * @returns {string | null}
   */
  function extractPathFromTransfer(dt) {
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
      const path = pathFromDropText(rawUri);
      if (path) return path;
    }

    return null;
  }

  /**
   * @param {DragEvent} event
   * @returns {string | null}
   */
  function extractFolderPathFromDrop(event) {
    const dt = event.dataTransfer;
    if (!dt) return null;

    for (const item of Array.from(dt.items || [])) {
      const entry = /** @type {{ webkitGetAsEntry?: () => { isDirectory?: boolean } | null }} */ (
        item
      ).webkitGetAsEntry?.();
      if (entry?.isDirectory) {
        const file = item.getAsFile?.();
        const fileWithPath = /** @type {{ path?: string }} */ (file || {});
        if (typeof fileWithPath.path === "string" && fileWithPath.path) {
          return fileWithPath.path;
        }
      }
    }

    if (dt.files && dt.files.length > 0) {
      /** @type {string[]} */
      const filePaths = [];
      for (const file of Array.from(dt.files)) {
        const fileWithPath = /** @type {{ path?: string }} */ (file);
        if (typeof fileWithPath.path === "string" && fileWithPath.path) {
          filePaths.push(fileWithPath.path);
        }
      }
      if (filePaths.length > 0) {
        const dir = commonDirectory(filePaths);
        if (dir) return dir;
      }
    }

    const rawUri = dt.getData("text/uri-list") || dt.getData("text/plain");
    if (rawUri) {
      const path = pathFromDropText(rawUri);
      if (path) return path;
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
    suppressZipClick = true;
    setTimeout(() => (suppressZipClick = false), 100);
    const path = extractZipPathFromDrop(event);
    if (path) {
      await applyDroppedZipPath(path, path.split(/[/\\]/).pop() || "");
      return;
    }

    const fallbackFile = firstDroppedFile(event.dataTransfer);
    if (!fallbackFile) {
      reportDropFailure("zip", event.dataTransfer);
      return;
    }

    try {
      const tempPath = await persistDroppedZipFile(fallbackFile);
      await applyDroppedZipPath(tempPath, fallbackFile.name || "");
    } catch (error) {
      reportDropFailure("zip", event.dataTransfer, error);
    }
  }

  /** @param {DragEvent} event */
  async function handleFolderDrop(event) {
    event.preventDefault();
    isFolderDragOver = false;
    suppressFolderClick = true;
    setTimeout(() => (suppressFolderClick = false), 100);
    const path = extractFolderPathFromDrop(event);
    if (path) {
      await applyDroppedFolderPath(path, path.split(/[/\\]/).pop() || "");
      return;
    }

    const entry = droppedDirectoryEntry(event.dataTransfer);
    if (!entry) {
      reportDropFailure("folder", event.dataTransfer);
      return;
    }

    try {
      const tempPath = await persistDroppedFolderEntry(entry);
      await applyDroppedFolderPath(tempPath, entry.name || "");
    } catch (error) {
      reportDropFailure("folder", event.dataTransfer, error);
    }
  }

  /**
   * @param {string} path
   * @param {string} [displayName]
   */
  async function applyDroppedZipPath(path, displayName = "") {
    const normalizedPath = normalizeDroppedPath(path);
    if (!normalizedPath) {
      zipError = $t("addSkill.zip.dropReadError");
      return;
    }
    if (!/\.(zip|skill)$/i.test(normalizedPath)) {
      zipError = $t("addSkill.zip.invalidCompressedFile");
      return;
    }
    zipError = "";
    selectedZipPath = normalizedPath;
    zipFileName = displayName || normalizedPath.split(/[/\\]/).pop() || "";
    await handleDetectZip();
  }

  /**
   * @param {string} path
   * @param {string} [displayName]
   */
  async function applyDroppedFolderPath(path, displayName = "") {
    const normalizedPath = normalizeDroppedPath(path);
    if (!normalizedPath) {
      folderError = $t("addSkill.folder.dropReadError");
      return;
    }
    folderError = "";
    selectedFolderPath = normalizedPath;
    folderName = displayName || normalizedPath.split(/[/\\]/).pop() || "";
    await handleDetectFolder();
  }

  async function handleDetectZip() {
    if (!selectedZipPath) return;

    isDetectingZip = true;
    zipError = "";
    detectedZipSkills = [];
    selectedZipSkill = null;

    try {
      const skills = await detectZip(selectedZipPath);
      detectedZipSkills = skills;
      selectedZipSkill = skills.length === 1 ? skills[0] : null;
      if (skills.length === 0) {
        zipError = $t("addSkill.noSkillsFound");
      }
    } catch (error) {
      zipError = String(error);
    } finally {
      isDetectingZip = false;
    }
  }

  async function handleSelectFolder() {
    if (suppressFolderClick) return;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({
        multiple: false,
        directory: true,
      });
      if (result) {
        await applyDroppedFolderPath(result, result.split(/[/\\]/).pop() || "");
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
      const skills = await detectFolder(selectedFolderPath);
      detectedFolderSkills = skills;
      selectedFolderSkill = skills.length === 1 ? skills[0] : null;
      if (skills.length === 0) {
        folderError = $t("addSkill.noSkillsFound");
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
    if (!validateConfirm()) return;

    isInstalling = true;
    installError = "";

    try {
      await installCurrentSelection();
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
    if (selectedScope === "project" && !selectedProjectPath) return false;
    if (activeTab === "zip") {
      return !!selectedZipPath && !!selectedZipSkill;
    } else if (activeTab === "folder") {
      return !!selectedFolderPath && !!selectedFolderSkill;
    } else {
      return !!selectedSkill;
    }
  }

  function validateConfirm() {
    if (selectedAgents.length === 0) {
      installError = $t("addSkill.noAgentsSelected");
      return false;
    }
    if (selectedScope === "project" && !selectedProjectPath) {
      installError = $t("addSkill.noProjectsSelected");
      return false;
    }
    if (activeTab === "zip") {
      if (!selectedZipPath) {
        installError = $t("addSkill.noZipSelected");
        return false;
      }
      if (!selectedZipSkill) {
        installError = $t("addSkill.noSkillSelected");
        return false;
      }
      return true;
    }
    if (activeTab === "folder") {
      if (!selectedFolderPath) {
        installError = $t("addSkill.noFolderSelected");
        return false;
      }
      if (!selectedFolderSkill) {
        installError = $t("addSkill.noSkillSelected");
        return false;
      }
      return true;
    }
    if (!selectedSkill) {
      installError = $t("addSkill.noSkillSelected");
      return false;
    }
    return true;
  }

  async function installCurrentSelection() {
    if (activeTab === "zip") {
      const zipSkill = selectedZipSkill;
      if (!zipSkill) return;
      await installFromNative({
        name: zipSkill.name,
        tmp_path: zipSkill.tmp_path,
        skill_path: zipSkill.skill_path,
        agent_apps: selectedAgents,
        method: selectedMethod,
        scope: selectedScope,
        project_path: selectedProjectPath,
      });
      return;
    }

    if (activeTab === "folder") {
      const folderSkill = selectedFolderSkill;
      if (!folderSkill) return;
      await installFromNative({
        name: folderSkill.name,
        tmp_path: folderSkill.tmp_path,
        skill_path: folderSkill.skill_path,
        agent_apps: selectedAgents,
        method: selectedMethod,
        scope: selectedScope,
        project_path: selectedProjectPath,
      });
      return;
    }

    const githubSkill = selectedSkill;
    if (!githubSkill) return;
    await installFromGithub({
      name: githubSkill.name,
      tmp_path: githubSkill.tmp_path,
      skill_path: githubSkill.skill_path,
      source_url: toGitRepoUrl(githubUrl),
      skill_folder_hash: null,
      agent_apps: selectedAgents,
      method: selectedMethod,
      scope: selectedScope,
      project_path: selectedProjectPath,
    });
  }
</script>

<Modal bind:open title={$t("addSkill.title")} onClose={closeModal} containerClass="max-w-xl">
  <div class="flex h-full min-h-0 w-full flex-col">
    <!-- Content -->
    <div class="flex-1 p-6 pt-1">
      <!-- Tabs -->
      <div class="sticky top-0 z-10 mb-4 pb-0">
        <SegmentedTabs
          items={[
            { value: "github", label: $t("addSkill.tab.github"), icon: Github },
            { value: "zip", label: $t("addSkill.tab.zip"), icon: FileArchive },
            { value: "folder", label: $t("addSkill.tab.folder"), icon: Folder },
          ]}
          value={activeTab}
          onChange={(tab) => (activeTab = tab)}
          fullWidth={true}
        />
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
            <PrimaryActionButton
              onclick={handleDetectGithub}
              disabled={!githubUrl.trim() || isDetecting}
              loading={isDetecting}
            >
              {$t("addSkill.github.detect")}
            </PrimaryActionButton>
          </div>

          {#if githubError}
            <div class="text-error flex items-center gap-2 text-sm">
              <AlertCircle size={16} />
              <span>{githubError}</span>
            </div>
          {/if}

          {#if detectedSkills.length > 0}
            <div class="space-y-2">
              <p class="text-base-content text-sm">
                {$t("addSkill.github.selectSkill")}
              </p>
              <DetectedSkillList
                skills={detectedSkills}
                {selectedSkill}
                onSelect={(skill) => (selectedSkill = skill)}
              />
            </div>
          {/if}
        </div>
      {:else if activeTab === "zip"}
        <!-- ZIP Mode -->
        <div class="space-y-3">
          <p class="text-base-content-muted text-sm">
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
              <p class="text-base-content text-sm">
                {$t("addSkill.zip.selectSkill")}
              </p>
              <DetectedSkillList
                skills={detectedZipSkills}
                selectedSkill={selectedZipSkill}
                onSelect={(skill) => (selectedZipSkill = skill)}
              />
            </div>
          {/if}
        </div>
      {:else}
        <!-- Folder Mode -->
        <div class="space-y-3">
          <p class="text-base-content-muted text-sm">
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
              <p class="text-base-content text-sm">
                {$t("addSkill.folder.selectSkill")}
              </p>
              <DetectedSkillList
                skills={detectedFolderSkills}
                selectedSkill={selectedFolderSkill}
                onSelect={(skill) => (selectedFolderSkill = skill)}
              />
            </div>
          {/if}
        </div>
      {/if}
      <!-- Agent Selection -->
      <div class="mt-6 space-y-3">
        <p class="text-base-content text-sm">
          {$t("addSkill.selectAgents")}
        </p>
        <AgentSelector {agents} bind:selectedIds={selectedAgents} />
      </div>

      {#if installError}
        <div class="text-error mt-4 flex items-center gap-2 text-sm">
          <AlertCircle size={16} />
          <span>{installError}</span>
        </div>
      {/if}
    </div>
  </div>
  {#snippet footer()}
    <div class="text-base-content-subtle mr-auto text-xs">{scopeLabel}</div>
    <SelectField bind:value={selectedMethod} disabled={isInstalling} className="mr-3">
        <option value="symlink">{$t("settings.syncMode.symlink")}</option>
        <option value="copy">{$t("settings.syncMode.copy")}</option>
    </SelectField>
    <PrimaryActionButton
      onclick={handleConfirm}
      disabled={!canConfirm() || isInstalling}
      loading={isInstalling}
      loadingText={$t("addSkill.installing")}
      className="select-none"
    >
      {$t("addSkill.confirm")}
    </PrimaryActionButton>
  {/snippet}
</Modal>
