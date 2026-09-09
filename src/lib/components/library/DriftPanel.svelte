<script lang="ts">
  import { AlertTriangle } from "@lucide/svelte";
  import { t } from "$lib/i18n";
  import { openDiffModal, type LibraryKind } from "$lib/stores/modals";
  import type { HubState, InstallView, SkillSource, SourceState, SyncAction } from "$lib/api/hub";

  export type TargetAction = "push" | "adopt" | "reinstall";

  let {
    name,
    kind = "skill",
    hubState,
    sourceState = "not_checkable",
    source = null,
    installs = [],
    busy = false,
    onSync,
    onTargetAction,
  }: {
    name: string;
    kind?: LibraryKind;
    hubState: HubState;
    /** Instructions have no source; they leave these at their defaults. */
    sourceState?: SourceState;
    source?: SkillSource | null;
    installs?: InstallView[];
    busy?: boolean;
    onSync: (action: SyncAction) => void;
    onTargetAction: (install: InstallView, action: TargetAction) => void;
  } = $props();

  const driftedTargets = $derived(installs.filter((install) => install.state !== "in_sync"));
  const hubIssue = $derived(hubState !== "ok");
  const sourceIssue = $derived(
    sourceState === "update_available" ||
      sourceState === "source_modified" ||
      sourceState === "source_missing"
  );
  const canPull = $derived(source?.type === "github" || source?.type === "folder");
  const hasOutdated = $derived(
    installs.some((install) => install.state === "outdated" || install.state === "missing")
  );

  const buttonClass =
    "border-base-300 text-base-content hover:bg-base-200 rounded-lg border px-2.5 py-1 text-xs transition disabled:opacity-50";

  const needsReinstall = (install: InstallView) =>
    install.state === "missing" || install.state === "broken_link";
  const canDiffTarget = (install: InstallView) =>
    install.mode === "copy" && !needsReinstall(install);
  const canDiffSource = $derived(
    (sourceState === "update_available" && source?.type === "github") ||
      (sourceState === "source_modified" && source?.type === "folder")
  );
</script>

<div class="border-warning/40 bg-warning/5 space-y-3 rounded-2xl border p-4 text-sm">
  {#if hubIssue}
    <div class="flex flex-wrap items-center justify-between gap-2">
      <p class="text-base-content flex items-center gap-2">
        <AlertTriangle size={15} class="text-warning-content shrink-0" />
        {$t(`drift.hub.${hubState}`)}
      </p>
      <div class="flex gap-1.5">
        {#if hubState === "modified" || hubState === "name_mismatch"}
          <button
            class={buttonClass}
            type="button"
            disabled={busy}
            onclick={() => onSync({ kind: "accept_hub" })}
          >
            {$t("drift.acceptHub")}
          </button>
        {/if}
        {#if canPull}
          <button
            class={buttonClass}
            type="button"
            disabled={busy}
            onclick={() => onSync({ kind: "pull_source" })}
          >
            {$t("drift.pullSource")}
          </button>
        {/if}
      </div>
    </div>
  {/if}

  {#if sourceIssue}
    <div class="flex flex-wrap items-center justify-between gap-2">
      <p class="text-base-content flex items-center gap-2">
        <AlertTriangle size={15} class="text-warning-content shrink-0" />
        {$t(`drift.source.${sourceState}`)}
      </p>
      <div class="flex gap-1.5">
        {#if canDiffSource}
          <button
            class={buttonClass}
            type="button"
            disabled={busy}
            onclick={() => openDiffModal(name, { kind: "source" }, kind)}
          >
            {$t("diff.view")}
          </button>
        {/if}
        {#if sourceState !== "source_missing" && canPull}
          <button
            class={buttonClass}
            type="button"
            disabled={busy}
            onclick={() => onSync({ kind: "pull_source" })}
          >
            {$t("drift.pullSource")}
          </button>
        {/if}
        {#if sourceState === "source_modified" && source?.type === "folder"}
          <button
            class={buttonClass}
            type="button"
            disabled={busy}
            onclick={() => onSync({ kind: "push_source" })}
          >
            {$t("drift.pushSource")}
          </button>
        {/if}
      </div>
    </div>
  {/if}

  {#if driftedTargets.length > 0}
    <ul class="space-y-1.5">
      {#each driftedTargets as install (install.path)}
        <li class="flex flex-wrap items-center justify-between gap-2">
          <p class="text-base-content-muted flex min-w-0 items-center gap-2">
            <span class={`state-dot state-${install.state} shrink-0`}></span>
            <span class="text-base-content shrink-0">{$t(`target.state.${install.state}`)}</span>
            <span class="truncate text-xs" title={install.path}>{install.path}</span>
          </p>
          <div class="flex gap-1.5">
            {#if canDiffTarget(install)}
              <button
                class={buttonClass}
                type="button"
                disabled={busy}
                onclick={() => openDiffModal(name, { kind: "target", path: install.path }, kind)}
              >
                {$t("diff.view")}
              </button>
            {/if}
            {#if install.state === "modified" || install.state === "conflict"}
              <button
                class={buttonClass}
                type="button"
                disabled={busy}
                onclick={() => onTargetAction(install, "adopt")}
              >
                {$t("target.adopt")}
              </button>
            {/if}
            {#if install.mode === "copy" || needsReinstall(install)}
              <button
                class={buttonClass}
                type="button"
                disabled={busy}
                onclick={() =>
                  onTargetAction(install, needsReinstall(install) ? "reinstall" : "push")}
              >
                {needsReinstall(install) ? $t("target.reinstall") : $t("target.push")}
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
    {#if hasOutdated && driftedTargets.length > 1}
      <div class="flex justify-end">
        <button
          class={buttonClass}
          type="button"
          disabled={busy}
          onclick={() => onSync({ kind: "push_targets" })}
        >
          {$t("drift.pushAll")}
        </button>
      </div>
    {/if}
  {/if}
</div>
