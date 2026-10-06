<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { open as openExternal } from "@tauri-apps/plugin-shell";
  import {
    getGithubAuthStatus,
    setGithubAccount,
    type GhAccount,
    type GithubAuthStatus,
  } from "$lib/api";
  import { t } from "$lib/i18n";
  import { ChevronLeft, Loader2, RefreshCw, ExternalLink, Check } from "@lucide/svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";

  const GH_INSTALL_URL = "https://cli.github.com";

  let status = $state<GithubAuthStatus | null>(null);
  let loading = $state(false);
  let saving = $state<string | null>(null);
  let error = $state("");

  onMount(() => {
    loadStatus();
  });

  async function loadStatus() {
    loading = true;
    error = "";
    try {
      status = await getGithubAuthStatus();
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  async function selectAccount(account: GhAccount) {
    if (!status || saving || account.state !== "success") return;
    if (account.login === status.selected_login) return;
    saving = account.login;
    error = "";
    try {
      status = await setGithubAccount(account.login);
    } catch (err) {
      error = String(err);
    } finally {
      saving = null;
    }
  }

  // Fine-grained tokens report no scopes, so only warn when classic scopes lack "repo".
  function lacksRepoScope(account: GhAccount): boolean {
    const scopes = account.scopes
      .split(",")
      .map((scope) => scope.trim())
      .filter(Boolean);
    return scopes.length > 0 && !scopes.includes("repo");
  }
</script>

<section class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
  <header class="border-base-300 bg-base-100 h-12 flex-none border-b" data-window-drag-region>
    <div class="mx-auto flex h-full w-full max-w-6xl items-center justify-between px-6">
      <div class="flex items-center gap-4">
        <IconButton
          variant="outline"
          onclick={() => goto("/settings")}
          title={$t("header.back")}
          ariaLabel={$t("header.back")}
        >
          <ChevronLeft size={16} />
        </IconButton>
        <h1 class="text-base-content text-lg font-medium">{$t("settings.github.title")}</h1>
      </div>
      <IconButton
        variant="outline"
        onclick={loadStatus}
        disabled={loading}
        title={$t("local.refresh")}
      >
        {#if loading}
          <Loader2 size={16} class="animate-spin" />
        {:else}
          <RefreshCw size={16} />
        {/if}
      </IconButton>
    </div>
  </header>

  <main class="min-h-0 flex-1 overflow-y-auto">
    <div class="mx-auto max-w-4xl space-y-4 px-7 py-6">
      <p class="text-base-content-muted text-[13px] leading-relaxed">
        {$t("settings.github.description")}
      </p>

      {#if loading && !status}
        <div class="text-base-content-muted flex items-center justify-center py-20">
          <Loader2 size={32} class="animate-spin" />
        </div>
      {:else if status && !status.gh_installed}
        <div class="bg-base-200 flex flex-col gap-3 rounded-2xl px-4 py-3">
          <span class="text-base-content text-[15px] font-medium">
            {$t("settings.github.noGhTitle")}
          </span>
          <span class="text-base-content-muted text-[13px]">{$t("settings.github.noGhHint")}</span>
          <div>
            <button
              class="bg-primary text-primary-content hover:bg-primary-hover flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-[13px]"
              onclick={() => openExternal(GH_INSTALL_URL)}
              type="button"
            >
              <ExternalLink size={13} />
              {$t("settings.github.installGh")}
            </button>
          </div>
        </div>
      {:else if status?.error}
        <div class="bg-base-200 text-error rounded-2xl px-4 py-3 text-[13px] break-all">
          {$t("settings.github.statusError", { error: status.error })}
        </div>
      {:else if status && status.accounts.length === 0}
        <div class="bg-base-200 flex flex-col gap-2 rounded-2xl px-4 py-3">
          <span class="text-base-content text-[15px] font-medium">
            {$t("settings.github.notSignedInTitle")}
          </span>
          <span class="text-base-content-muted text-[13px]">
            {$t("settings.github.notSignedInHint")}
          </span>
        </div>
      {:else if status}
        <div class="flex flex-col gap-2">
          <div class="flex items-baseline justify-between px-1">
            <h2 class="text-base-content text-[15px] font-medium">
              {$t("settings.github.accounts")}
            </h2>
            {#if status.accounts.length > 1}
              <span class="text-base-content-muted text-xs">
                {$t("settings.github.multipleHint")}
              </span>
            {/if}
          </div>
          {#each status.accounts as account (account.login)}
            {@const selected = account.login === status.selected_login}
            {@const usable = account.state === "success"}
            <button
              class="bg-base-200 flex w-full items-start gap-3 rounded-2xl border px-4 py-2.5 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-60"
              class:border-primary={selected}
              class:border-transparent={!selected}
              class:hover:bg-base-300={usable && !selected}
              disabled={!usable || saving !== null}
              onclick={() => selectAccount(account)}
              type="button"
            >
              <span
                class="mt-0.5 flex h-4 w-4 flex-none items-center justify-center rounded-full border"
                class:border-primary={selected}
                class:bg-primary={selected}
                class:border-base-300={!selected}
              >
                {#if saving === account.login}
                  <Loader2 size={10} class="animate-spin" />
                {:else if selected}
                  <Check size={10} class="text-primary-content" />
                {/if}
              </span>
              <span class="flex min-w-0 flex-1 flex-col gap-1">
                <span class="flex flex-wrap items-center gap-2">
                  <span class="text-base-content text-[14px] font-medium">{account.login}</span>
                  {#if selected}
                    <span class="bg-primary/15 text-primary rounded px-1.5 py-0.5 text-[11px]">
                      {$t("settings.github.inUse")}
                    </span>
                  {/if}
                  {#if account.active}
                    <span
                      class="bg-base-300 text-base-content-muted rounded px-1.5 py-0.5 text-[11px]"
                    >
                      {$t("settings.github.active")}
                    </span>
                  {/if}
                  {#if !usable}
                    <span class="bg-error/15 text-error rounded px-1.5 py-0.5 text-[11px]">
                      {$t("settings.github.invalid")}
                    </span>
                  {/if}
                </span>
                {#if account.scopes}
                  <span class="text-base-content-muted text-xs break-all">
                    {$t("settings.github.scopes", { scopes: account.scopes })}
                  </span>
                {/if}
                {#if usable && lacksRepoScope(account)}
                  <span class="text-warning-content text-xs">
                    {$t("settings.github.noRepoScope")}
                  </span>
                {/if}
              </span>
            </button>
          {/each}
        </div>
      {/if}

      {#if error}
        <div class="text-error text-[13px]">{error}</div>
      {/if}

      {#if status?.gh_path}
        <p class="text-base-content-muted px-1 text-xs break-all">
          {$t("settings.github.ghPath", { path: status.gh_path })}
        </p>
      {/if}
    </div>
  </main>
</section>
