<script lang="ts">
  import type { AutoSyncStatus, SyncPolicy } from "$lib/api";
  import type { AutoSyncSettings } from "$lib/api/settings";
  import SelectField from "$lib/components/ui/SelectField.svelte";
  import Switch from "$lib/components/ui/Switch.svelte";
  import { t } from "$lib/i18n";
  import { autoSyncStatus } from "$lib/stores/auto-sync";
  import { settings, updateSettings } from "$lib/stores/settings";
  import SettingsRow from "./SettingsRow.svelte";

  /**
   * Library auto sync of this computer: the schedule, the sync policy and the result of the
   * last cycle. See docs/decisions/0005-library-auto-sync.md.
   */
  let { formatTime }: { formatTime: (value?: string) => string } = $props();

  const selectClass = "h-8 text-[13px]";
  const auto = $derived($settings.auto_sync);

  function save(patch: Partial<AutoSyncSettings>) {
    // A failed save puts back the stored settings.
    updateSettings({ auto_sync: { ...auto, ...patch } }).catch(console.error);
  }

  function savePolicy(patch: Partial<SyncPolicy>) {
    save({ policy: { ...auto.policy, ...patch } });
  }

  /** Turning auto sync on starts with both steps on. */
  function setEnabled(enabled: boolean) {
    save(enabled ? { enabled, backup: true, restore: true } : { enabled });
  }

  function lastText(status: AutoSyncStatus | null): string {
    if (!status) return $t("settings.sync.auto.notRun");
    const time = formatTime(status.at);
    if (status.error) return $t("settings.sync.auto.failed", { time, error: status.error });
    const outcome = status.outcome;
    if (!outcome) return "";
    if (outcome.result === "needs_attention") {
      const reason = outcome.reason ?? "ask";
      return $t(`settings.sync.auto.attention.${reason}`, { time, count: outcome.ask.length });
    }
    return outcome.result === "synced"
      ? $t("settings.sync.auto.synced", { time })
      : $t("settings.sync.auto.upToDate", { time });
  }

  const attention = $derived(
    Boolean($autoSyncStatus?.error || $autoSyncStatus?.outcome?.result === "needs_attention")
  );
</script>

<div class="settings-card">
  <SettingsRow
    label={$t("settings.sync.auto.title")}
    description={auto.enabled ? lastText($autoSyncStatus) : $t("settings.sync.auto.hint")}
    descriptionTone={auto.enabled && attention ? "warning" : "muted"}
  >
    <Switch checked={auto.enabled} label={$t("settings.sync.auto.title")} onchange={setEnabled} />
  </SettingsRow>
  {#if auto.enabled}
    <SettingsRow label={$t("settings.sync.auto.backup")}>
      <Switch
        checked={auto.backup}
        label={$t("settings.sync.auto.backup")}
        onchange={(backup) => save({ backup })}
      />
    </SettingsRow>
    <SettingsRow label={$t("settings.sync.auto.restore")}>
      <Switch
        checked={auto.restore}
        label={$t("settings.sync.auto.restore")}
        onchange={(restore) => save({ restore })}
      />
    </SettingsRow>
    <SettingsRow label={$t("settings.sync.auto.interval")}>
      <SelectField
        value={auto.interval}
        fit
        selectClassName={selectClass}
        onchange={(event) =>
          save({ interval: event.currentTarget.value as AutoSyncSettings["interval"] })}
      >
        {#each ["15m", "1h", "6h", "24h"] as const as interval (interval)}
          <option value={interval}>{$t(`settings.sync.auto.every.${interval}`)}</option>
        {/each}
      </SelectField>
    </SettingsRow>
    {#if auto.restore}
      <SettingsRow
        label={$t("settings.sync.auto.repositoryChange")}
        description={$t("settings.sync.auto.repositoryChangeHint")}
      >
        <SelectField
          value={auto.policy.repositoryChange}
          fit
          selectClassName={selectClass}
          onchange={(event) =>
            savePolicy({
              repositoryChange: event.currentTarget.value as SyncPolicy["repositoryChange"],
            })}
        >
          <option value="apply">{$t("settings.sync.auto.policy.apply")}</option>
          <option value="ask">{$t("settings.sync.auto.policy.ask")}</option>
        </SelectField>
      </SettingsRow>
      <SettingsRow
        label={$t("settings.sync.auto.removal")}
        description={$t("settings.sync.auto.removalHint")}
      >
        <SelectField
          value={auto.policy.removal}
          fit
          selectClassName={selectClass}
          onchange={(event) =>
            savePolicy({ removal: event.currentTarget.value as SyncPolicy["removal"] })}
        >
          <option value="ask">{$t("settings.sync.auto.policy.ask")}</option>
          <option value="remove">{$t("settings.sync.auto.policy.remove")}</option>
        </SelectField>
      </SettingsRow>
      <SettingsRow
        label={$t("settings.sync.auto.conflict")}
        description={$t("settings.sync.auto.conflictHint")}
      >
        <SelectField
          value={auto.policy.conflict}
          fit
          selectClassName={selectClass}
          onchange={(event) =>
            savePolicy({ conflict: event.currentTarget.value as SyncPolicy["conflict"] })}
        >
          <option value="ask">{$t("settings.sync.auto.policy.ask")}</option>
          <option value="keep_local">{$t("settings.sync.auto.policy.keep_local")}</option>
          <option value="take_remote">{$t("settings.sync.auto.policy.take_remote")}</option>
          <option value="keep_both">{$t("settings.sync.auto.policy.keep_both")}</option>
        </SelectField>
      </SettingsRow>
    {/if}
  {/if}
</div>
