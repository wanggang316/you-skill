<script lang="ts">
  import { AlertTriangle } from "@lucide/svelte";
  import { t } from "$lib/i18n";
  import type { HubSkillView, InstallView, SyncAction } from "$lib/api/hub";

  let {
    skill,
    busy = false,
    onSync,
    onTargetPush,
    onTargetAdopt,
  }: {
    skill: HubSkillView;
    busy?: boolean;
    onSync: (action: SyncAction) => void;
    onTargetPush: (install: InstallView) => void;
    onTargetAdopt: (install: InstallView) => void;
  } = $props();

  const driftedTargets = $derived(skill.installs.filter((install) => install.state !== "in_sync"));
  const hubIssue = $derived(skill.hubState !== "ok");
  const sourceIssue = $derived(
    skill.sourceState === "update_available" ||
      skill.sourceState === "source_modified" ||
      skill.sourceState === "source_missing"
  );
  const canPull = $derived(skill.source.type === "github" || skill.source.type === "folder");
  const hasOutdated = $derived(
    skill.installs.some((install) => install.state === "outdated" || install.state === "missing")
  );

  const buttonClass =
    "border-base-300 text-base-content hover:bg-base-200 rounded-lg border px-2.5 py-1 text-xs transition disabled:opacity-50";
</script>

<div class="border-warning/40 bg-warning/5 space-y-3 rounded-2xl border p-4 text-sm">
  {#if hubIssue}
    <div class="flex flex-wrap items-center justify-between gap-2">
      <p class="text-base-content flex items-center gap-2">
        <AlertTriangle size={15} class="text-warning-content shrink-0" />
        {$t(`drift.hub.${skill.hubState}`)}
      </p>
      <div class="flex gap-1.5">
        {#if skill.hubState === "modified" || skill.hubState === "name_mismatch"}
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
        {$t(`drift.source.${skill.sourceState}`)}
      </p>
      <div class="flex gap-1.5">
        {#if skill.sourceState !== "source_missing" && canPull}
          <button
            class={buttonClass}
            type="button"
            disabled={busy}
            onclick={() => onSync({ kind: "pull_source" })}
          >
            {$t("drift.pullSource")}
          </button>
        {/if}
        {#if skill.sourceState === "source_modified" && skill.source.type === "folder"}
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
            {#if install.state === "modified" || install.state === "conflict"}
              <button
                class={buttonClass}
                type="button"
                disabled={busy}
                onclick={() => onTargetAdopt(install)}
              >
                {$t("target.adopt")}
              </button>
            {/if}
            {#if install.mode === "copy" || install.state === "broken_link" || install.state === "missing"}
              <button
                class={buttonClass}
                type="button"
                disabled={busy}
                onclick={() => onTargetPush(install)}
              >
                {install.state === "missing" || install.state === "broken_link"
                  ? $t("target.reinstall")
                  : $t("target.push")}
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
