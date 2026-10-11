<script lang="ts">
  import { onMount } from "svelte";
  import { open as openExternal } from "@tauri-apps/plugin-shell";
  import { Check, ExternalLink, Loader2, RefreshCw } from "@lucide/svelte";
  import {
    getGithubAuthStatus,
    setGithubAccount,
    type GhAccount,
    type GithubAuthStatus,
  } from "$lib/api";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "$lib/i18n";
  import SettingsPane from "./SettingsPane.svelte";
  import SettingsRow from "./SettingsRow.svelte";

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

<SettingsPane title={$t("settings.github.title")} description={$t("settings.github.description")}>
  {#snippet actions()}
    <IconButton
      variant="outline"
      onclick={loadStatus}
      disabled={loading}
      title={$t("local.refresh")}
      class="h-8 w-8 p-0"
    >
      <RefreshCw size={15} class={loading ? "animate-spin" : ""} />
    </IconButton>
  {/snippet}

  {#if loading && !status}
    <div class="text-base-content-muted flex items-center justify-center py-16">
      <Loader2 size={24} class="animate-spin" />
    </div>
  {:else if status && !status.gh_installed}
    <div class="settings-card">
      <SettingsRow
        label={$t("settings.github.noGhTitle")}
        description={$t("settings.github.noGhHint")}
      >
        <PrimaryActionButton
          className="h-8 rounded-lg px-3 py-0 text-[13px]"
          onclick={() => openExternal(GH_INSTALL_URL)}
        >
          <ExternalLink size={13} />
          {$t("settings.github.installGh")}
        </PrimaryActionButton>
      </SettingsRow>
    </div>
  {:else if status?.error}
    <p class="text-error text-[13px] break-all">
      {$t("settings.github.statusError", { error: status.error })}
    </p>
  {:else if status && status.accounts.length === 0}
    <div class="settings-card">
      <SettingsRow
        label={$t("settings.github.notSignedInTitle")}
        description={$t("settings.github.notSignedInHint")}
      />
    </div>
  {:else if status}
    <div class="space-y-2">
      <div class="flex items-baseline justify-between gap-4 px-1">
        <h3 class="text-base-content-subtle text-[11px] font-medium tracking-wide uppercase">
          {$t("settings.github.accounts")}
        </h3>
        {#if status.accounts.length > 1}
          <span class="text-base-content-muted text-xs">{$t("settings.github.multipleHint")}</span>
        {/if}
      </div>
      <div class="settings-card overflow-hidden">
        {#each status.accounts as account (account.login)}
          {@const selected = account.login === status.selected_login}
          {@const usable = account.state === "success"}
          <button
            class={`flex w-full items-start gap-3 px-4 py-3 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-60 ${
              usable && !selected ? "hover:bg-base-200" : ""
            }`}
            disabled={!usable || saving !== null}
            onclick={() => selectAccount(account)}
            type="button"
          >
            <span
              class={`mt-0.5 flex size-4 flex-none items-center justify-center rounded-full border ${
                selected ? "border-primary bg-primary" : "border-base-300"
              }`}
            >
              {#if saving === account.login}
                <Loader2 size={10} class="animate-spin" />
              {:else if selected}
                <Check size={10} class="text-primary-content" />
              {/if}
            </span>
            <span class="flex min-w-0 flex-1 flex-col gap-1">
              <span class="flex flex-wrap items-center gap-2">
                <span class="text-base-content text-[13.5px] font-medium">{account.login}</span>
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
                <span class="text-warning-content text-xs">{$t("settings.github.noRepoScope")}</span
                >
              {/if}
            </span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  {#if error}
    <p class="text-error text-[13px] break-all">{error}</p>
  {/if}

  {#if status?.gh_path}
    <p class="text-base-content-muted px-1 text-xs break-all">
      {$t("settings.github.ghPath", { path: status.gh_path })}
    </p>
  {/if}
</SettingsPane>
