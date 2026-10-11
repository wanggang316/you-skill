<script lang="ts">
  import { onMount } from "svelte";
  import { RefreshCw } from "@lucide/svelte";
  import { getCliStatus, installCli, uninstallCli, type CliStatus } from "$lib/api";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "$lib/i18n";
  import SettingsPane from "./SettingsPane.svelte";
  import SettingsRow from "./SettingsRow.svelte";

  let status = $state<CliStatus | null>(null);
  let loading = $state(false);
  let busy = $state(false);
  let message = $state("");
  let messageIsError = $state(false);

  const summary = $derived.by(() => {
    if (!status) return "";
    if (!status.bundled_path) return $t("settings.cli.notBundled");
    if (!status.installed) return $t("settings.cli.notInstalled");
    if (status.current) return $t("settings.cli.installedAt", { path: status.install_path });
    return $t("settings.cli.outdated", { path: status.install_path });
  });

  onMount(() => {
    loadStatus();
  });

  async function loadStatus() {
    loading = true;
    try {
      status = await getCliStatus();
    } catch (error) {
      console.error("Failed to load command line tool status:", error);
    } finally {
      loading = false;
    }
  }

  async function run(work: () => Promise<CliStatus>, fallback: string, done = "") {
    busy = true;
    message = "";
    messageIsError = false;
    try {
      status = await work();
      message = done;
    } catch (error) {
      message =
        error instanceof Error && error.message
          ? error.message
          : typeof error === "string" && error.trim()
            ? error
            : fallback;
      messageIsError = true;
      console.error(fallback, error);
    } finally {
      busy = false;
    }
  }
</script>

<SettingsPane title={$t("settings.cli.title")} description={$t("settings.cli.description")}>
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

  <div class="settings-card">
    <SettingsRow label={$t("settings.cli.status")} description={summary}>
      {#if status?.installed}
        <button
          class="settings-button"
          type="button"
          onclick={() => run(uninstallCli, $t("settings.cli.removeFailed"))}
          disabled={busy}
        >
          {$t("settings.cli.remove")}
        </button>
      {/if}
      <PrimaryActionButton
        className="h-8 rounded-lg px-3 py-0 text-[13px]"
        onclick={() =>
          run(installCli, $t("settings.cli.installFailed"), $t("settings.cli.newTerminal"))}
        disabled={!status?.bundled_path || status?.current}
        loading={busy}
        loadingText={$t("settings.cli.installing")}
      >
        {status?.installed ? $t("settings.cli.reinstall") : $t("settings.cli.install")}
      </PrimaryActionButton>
    </SettingsRow>
  </div>

  {#if message}
    <p
      class={`px-1 text-xs break-all ${messageIsError ? "text-error" : "text-base-content-muted"}`}
    >
      {message}
    </p>
  {/if}
</SettingsPane>
