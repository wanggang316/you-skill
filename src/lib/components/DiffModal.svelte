<script lang="ts">
  import { FileMinus, FilePlus, FileDiff as FileDiffIcon, Loader2 } from "@lucide/svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import { t } from "../i18n";
  import { closeDiffModal, diffModal, type LibraryKind } from "../stores/modals";
  import { diffSkill, type DiffAgainst, type DiffStatus, type SkillDiff } from "../api/hub";
  import { diffInstruction } from "../api/instructions";

  let open = $state(false);
  let loading = $state(false);
  let error = $state("");
  let diff = $state<SkillDiff | null>(null);
  let selectedPath = $state<string | null>(null);
  let requestId = 0;

  const selectedFile = $derived(
    diff?.files.find((file) => file.path === selectedPath) ?? diff?.files[0] ?? null
  );

  $effect(() => {
    const state = $diffModal;
    open = state.open;
    if (state.open && state.name) {
      void load(state.name, state.against, state.kind);
    }
  });

  async function load(name: string, against: DiffAgainst, kind: LibraryKind) {
    const id = ++requestId;
    loading = true;
    error = "";
    diff = null;
    selectedPath = null;
    try {
      const result =
        kind === "instruction" && against.kind === "target"
          ? await diffInstruction(name, against.path)
          : await diffSkill(name, against);
      if (id !== requestId) return;
      diff = result;
      selectedPath = result.files[0]?.path ?? null;
    } catch (err) {
      if (id !== requestId) return;
      error = String(err);
    } finally {
      if (id === requestId) loading = false;
    }
  }

  function handleClose() {
    requestId += 1;
    closeDiffModal();
    open = false;
    diff = null;
    error = "";
    loading = false;
  }

  function statusClass(status: DiffStatus): string {
    switch (status) {
      case "added":
        return "text-success";
      case "removed":
        return "text-error";
      default:
        return "text-warning-content";
    }
  }
</script>

<Modal
  bind:open
  title={$diffModal.name ? `${$t("diff.title")} · ${$diffModal.name}` : $t("diff.title")}
  onClose={handleClose}
  containerClass="max-w-[min(94vw,76rem)]"
>
  <div class="flex h-[78vh] min-h-0 flex-col">
    {#if loading}
      <div
        class="text-base-content-muted flex flex-1 flex-col items-center justify-center gap-2 text-sm"
      >
        <Loader2 size={24} class="animate-spin" />
        <span>{$t("diff.loading")}</span>
      </div>
    {:else if error}
      <div class="p-6">
        <p class="text-error text-sm whitespace-pre-wrap">{error}</p>
      </div>
    {:else if diff}
      <div
        class="border-base-200 text-base-content-muted flex flex-none flex-wrap items-center gap-x-4 gap-y-1 border-b px-5 py-2 text-xs"
      >
        <span class="inline-flex min-w-0 items-center gap-1.5">
          <span class="text-error font-medium">−</span>
          <span class="text-base-content shrink-0">{$t("diff.hub")}</span>
          <span class="truncate" title={diff.leftLabel}>{diff.leftLabel}</span>
        </span>
        <span class="inline-flex min-w-0 items-center gap-1.5">
          <span class="text-success font-medium">+</span>
          <span class="truncate" title={diff.rightLabel}>{diff.rightLabel}</span>
        </span>
        <span class="ml-auto shrink-0">
          {diff.files.length}
          {$t("diff.changed")} · {diff.unchanged}
          {$t("diff.unchanged")}
        </span>
      </div>

      {#if diff.files.length === 0}
        <div class="text-base-content-muted flex flex-1 items-center justify-center text-sm">
          {$t("diff.none")}
        </div>
      {:else}
        <div class="flex min-h-0 flex-1">
          <ul class="border-base-200 w-60 shrink-0 overflow-y-auto border-r py-2">
            {#each diff.files as file (file.path)}
              <li>
                <button
                  class={`hover:bg-base-200 flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs transition ${
                    selectedFile?.path === file.path ? "bg-base-200" : ""
                  }`}
                  type="button"
                  onclick={() => (selectedPath = file.path)}
                  title={`${file.path} · ${$t(`diff.status.${file.status}`)}`}
                >
                  <span class={`shrink-0 ${statusClass(file.status)}`}>
                    {#if file.status === "added"}
                      <FilePlus size={13} />
                    {:else if file.status === "removed"}
                      <FileMinus size={13} />
                    {:else}
                      <FileDiffIcon size={13} />
                    {/if}
                  </span>
                  <span class="text-base-content truncate font-mono">{file.path}</span>
                </button>
              </li>
            {/each}
          </ul>

          <div class="min-w-0 flex-1 overflow-auto font-mono text-[12px] leading-5">
            {#if selectedFile}
              <div
                class="border-base-200 bg-base-100 sticky top-0 z-10 flex items-center gap-2 border-b px-4 py-1.5"
              >
                <span class={`text-[11px] font-medium ${statusClass(selectedFile.status)}`}>
                  {$t(`diff.status.${selectedFile.status}`)}
                </span>
                <span class="text-base-content truncate">{selectedFile.path}</span>
              </div>
              {#if selectedFile.binary}
                <p class="text-base-content-muted px-4 py-6 font-sans text-sm">
                  {$t("diff.binary")}
                </p>
              {:else}
                {#each selectedFile.hunks as hunk, index (index)}
                  <div class="text-base-content-subtle bg-base-200/70 px-4 py-0.5 select-none">
                    {hunk.header}
                  </div>
                  {#each hunk.lines as line, lineIndex (lineIndex)}
                    <div
                      class={`grid grid-cols-[3rem_3rem_1rem_minmax(0,1fr)] ${
                        line.kind === "insert"
                          ? "bg-success/10"
                          : line.kind === "delete"
                            ? "bg-error/10"
                            : ""
                      }`}
                    >
                      <span class="text-base-content-faint pr-2 text-right select-none">
                        {line.oldLine ?? ""}
                      </span>
                      <span class="text-base-content-faint pr-2 text-right select-none">
                        {line.newLine ?? ""}
                      </span>
                      <span
                        class={`select-none ${
                          line.kind === "insert"
                            ? "text-success"
                            : line.kind === "delete"
                              ? "text-error"
                              : "text-base-content-faint"
                        }`}
                      >
                        {line.kind === "insert" ? "+" : line.kind === "delete" ? "−" : " "}
                      </span>
                      <pre
                        class="text-base-content m-0 pr-4 break-all whitespace-pre-wrap">{line.text}</pre>
                    </div>
                  {/each}
                {/each}
                {#if selectedFile.truncated}
                  <p class="text-base-content-muted px-4 py-3 font-sans text-xs">
                    {$t("diff.truncated")}
                  </p>
                {/if}
              {/if}
            {/if}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</Modal>
