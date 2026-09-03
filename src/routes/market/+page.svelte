<script lang="ts">
  import { get } from "svelte/store";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { Check, ChevronDown, ChevronsUpDown, Loader2, RefreshCw, Search } from "@lucide/svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import { t } from "$lib/i18n";
  import {
    detectGithubAuto,
    importSkills,
    recordInstall,
    syncSkill,
    type RemoteSkill,
    type SkillSource,
  } from "$lib/api";
  import { hubSkillsByName, refreshHub } from "$lib/stores/hub";
  import {
    loadRemote,
    remoteError,
    remoteHasMore,
    remoteLoaded,
    remoteLoading,
    remoteSkills,
  } from "$lib/stores/market";
  import { openInstallModal, performAction } from "$lib/stores/modals";

  const PAGE_SIZE = 20;

  let query = $state("");
  let skip = $state(0);
  let sortBy = $state("heat_score");
  let busySkillId = $state("");
  let log = $state("");
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (!get(remoteLoaded)) {
      load(true).catch(console.error);
    }
  });

  async function load(reset: boolean) {
    if (reset) skip = 0;
    await loadRemote({
      reset,
      skip,
      limit: PAGE_SIZE,
      search: query,
      sortBy,
      sortOrder: "desc",
    });
  }

  function handleSearchInput() {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => load(true).catch(console.error), 500);
  }

  async function loadMore() {
    if (!get(remoteHasMore)) return;
    skip += PAGE_SIZE;
    await load(false);
  }

  const repoSlug = (skill: RemoteSkill) => skill.source.trim();

  function hubEntry(skill: RemoteSkill) {
    return $hubSkillsByName.get(skill.name) ?? null;
  }

  function hasUpdate(skill: RemoteSkill): boolean {
    const entry = hubEntry(skill);
    if (!entry || entry.source.type !== "github") return false;
    if (entry.sourceState === "update_available") return true;
    const remoteSha = skill.skill_path_sha;
    return Boolean(remoteSha && entry.source.remoteSha && entry.source.remoteSha !== remoteSha);
  }

  function buttonLabel(skill: RemoteSkill): string {
    if (busySkillId === skill.id) return $t("market.downloading");
    if (hasUpdate(skill)) return $t("market.update");
    if (hubEntry(skill)) return $t("market.reimport");
    return $t("market.import");
  }

  async function handleImport(skill: RemoteSkill) {
    if (!skill.url || busySkillId) return;
    busySkillId = skill.id;
    log = "";
    try {
      const existing = hubEntry(skill);
      if (existing && hasUpdate(skill) && existing.source.type === "github") {
        const result = await performAction((force) =>
          syncSkill(skill.name, { kind: "pull_source", force })
        );
        if (!result.applied && result.blockers.length > 0) {
          log = result.blockers.join("\n");
        }
        await refreshHub();
        return;
      }

      const detected = await detectGithubAuto(skill.url, skill.name);
      const repo = repoSlug(skill);
      const source: SkillSource = {
        type: "github",
        repo,
        url: `https://github.com/${repo}.git`,
        skillPath: detected.skill_path,
        branch: detected.branch ?? skill.branch ?? null,
        remoteSha: skill.skill_path_sha ?? null,
        marketplaceId: skill.id,
      };
      await importSkills(
        [{ name: detected.name, tmpPath: detected.tmp_path, source }],
        Boolean(existing)
      );
      recordInstall(skill.id).catch((error) => console.error("Failed to record install:", error));
      await refreshHub();
      openInstallModal([detected.name]);
    } catch (error) {
      log = String(error);
    } finally {
      busySkillId = "";
    }
  }

  function handleView(skill: RemoteSkill) {
    const returnTo = `${page.url.pathname}${page.url.search}`;
    goto(
      `/skills/remote/${encodeURIComponent(skill.name)}?returnTo=${encodeURIComponent(returnTo)}`
    );
  }
</script>

<section class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
  <header
    class="border-base-300 flex h-12 flex-none items-center border-b px-7"
    data-window-drag-region
  >
    <h1 class="text-base-content truncate text-[1.05rem] font-semibold tracking-[-0.02em]">
      {$t("market.title")}
    </h1>
  </header>

  <main class="min-h-0 flex-1 overflow-y-auto">
    <div class="mx-auto max-w-6xl space-y-6 px-7 py-6">
      <div class="flex flex-wrap items-center gap-3">
        <div class="relative flex-1">
          <Search
            class="text-base-content-subtle absolute top-1/2 left-3 -translate-y-1/2"
            size={16}
          />
          <input
            class="border-base-300 bg-base-200 text-base-content placeholder:text-base-content-subtle focus:border-base-300 h-9 w-full rounded-xl border px-9 text-sm focus:outline-none"
            placeholder={$t("market.search.placeholder")}
            bind:value={query}
            oninput={handleSearchInput}
          />
        </div>
        <div class="relative">
          <select
            class="border-base-300 bg-base-100 text-base-content focus:border-base-300 h-9 cursor-pointer appearance-none rounded-xl border pr-8 pl-3 text-sm focus:outline-none"
            bind:value={sortBy}
            onchange={() => load(true).catch(console.error)}
          >
            <option value="heat_score">{$t("market.sort.popular")}</option>
            <option value="star_count">{$t("market.sort.stars")}</option>
          </select>
          <ChevronsUpDown
            class="text-base-content-subtle pointer-events-none absolute top-1/2 right-3 -translate-y-1/2"
            size={14}
          />
        </div>
        <IconButton
          variant="outline"
          onclick={() => load(true).catch(console.error)}
          title={$t("library.refresh")}
          ariaLabel={$t("library.refresh")}
          class="h-9 w-9"
        >
          <RefreshCw size={16} class={$remoteLoading ? "animate-spin" : ""} />
        </IconButton>
      </div>

      {#if $remoteError}
        <p class="text-error text-sm">{$remoteError}</p>
      {/if}
      {#if log}
        <div
          class="bg-base-200 text-base-content-muted rounded-lg px-3 py-2 text-xs whitespace-pre-wrap"
        >
          {log}
        </div>
      {/if}

      <div>
        {#if $remoteLoading && $remoteSkills.length === 0}
          <div
            class="border-base-300 text-base-content-muted rounded-2xl border border-dashed p-6 text-center text-sm"
          >
            {$t("market.loading")}
          </div>
        {:else if $remoteSkills.length === 0}
          <div
            class="border-base-300 text-base-content-muted rounded-2xl border border-dashed p-6 text-center text-sm"
          >
            {$t("market.empty")}
          </div>
        {:else}
          {#each $remoteSkills as skill (skill.id)}
            {@const inHub = Boolean(hubEntry(skill))}
            {@const busy = busySkillId === skill.id}
            <div
              class="border-base-300 hover:bg-base-200 border-b px-4 py-4 transition"
              onclick={() => handleView(skill)}
              onkeydown={(e) => (e.key === "Enter" || e.key === " ") && handleView(skill)}
              role="button"
              tabindex="0"
              aria-label={`View ${skill.name}`}
            >
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <p class="cursor-pointer truncate text-base">
                      {skill.name}
                      <span class="text-base-content/70 text-sm font-normal">({skill.source})</span>
                    </p>
                    {#if inHub}
                      <span
                        class="text-success inline-flex items-center rounded-lg px-2 py-0.5 text-xs"
                      >
                        <Check size={10} class="mr-1" />
                        {$t("market.imported")}
                      </span>
                    {/if}
                  </div>
                  {#if skill.description}
                    <p class="text-base-content-subtle mt-1 line-clamp-1 text-xs">
                      {skill.description}
                    </p>
                  {/if}
                </div>
                <div
                  class="flex items-center gap-2"
                  onclick={(e) => e.stopPropagation()}
                  onkeydown={(e) => e.stopPropagation()}
                  role="presentation"
                >
                  <button
                    class="border-base-300 bg-base-300 text-primary hover:bg-primary hover:text-primary-content inline-flex h-6 items-center rounded-lg border px-2 py-0.5 text-xs transition disabled:cursor-not-allowed disabled:opacity-50"
                    onclick={() => handleImport(skill)}
                    disabled={Boolean(busySkillId)}
                    type="button"
                  >
                    {#if busy}
                      <Loader2 size={12} class="animate-spin" />
                    {:else}
                      {buttonLabel(skill)}
                    {/if}
                  </button>
                </div>
              </div>
            </div>
          {/each}
        {/if}
      </div>

      <div class="flex items-center justify-center">
        <IconButton
          variant="outline"
          onclick={() => loadMore().catch(console.error)}
          disabled={!$remoteHasMore || $remoteLoading}
          title={$remoteHasMore ? $t("market.loadMore") : $t("market.noMore")}
          ariaLabel={$remoteHasMore ? $t("market.loadMore") : $t("market.noMore")}
        >
          {#if $remoteLoading}
            <RefreshCw size={16} class="animate-spin" />
          {:else}
            <ChevronDown size={16} />
          {/if}
        </IconButton>
      </div>
    </div>
  </main>
</section>
