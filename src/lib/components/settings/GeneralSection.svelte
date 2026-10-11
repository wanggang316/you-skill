<script lang="ts">
  import SelectField from "$lib/components/ui/SelectField.svelte";
  import { t } from "$lib/i18n";
  import type { AppSettings } from "$lib/api/settings";
  import { settings, updateSettings } from "$lib/stores/settings";
  import SettingsPane from "./SettingsPane.svelte";
  import SettingsRow from "./SettingsRow.svelte";

  const selectClass = "h-8 text-[13px]";
</script>

<SettingsPane
  title={$t("settings.section.general")}
  description={$t("settings.general.description")}
>
  <div class="settings-card">
    <SettingsRow label={$t("settings.language")}>
      <SelectField
        value={$settings.language}
        selectClassName={selectClass}
        onchange={(event) =>
          updateSettings({ language: event.currentTarget.value as AppSettings["language"] })}
      >
        <option value="en">English</option>
        <option value="zh">中文</option>
      </SelectField>
    </SettingsRow>
    <SettingsRow label={$t("settings.theme")}>
      <SelectField
        value={$settings.theme}
        selectClassName={selectClass}
        onchange={(event) =>
          updateSettings({ theme: event.currentTarget.value as AppSettings["theme"] })}
      >
        <option value="system">{$t("settings.theme.system")}</option>
        <option value="light">{$t("settings.theme.light")}</option>
        <option value="dark">{$t("settings.theme.dark")}</option>
      </SelectField>
    </SettingsRow>
  </div>

  <div class="settings-card">
    <SettingsRow label={$t("settings.installMode")}>
      <SelectField
        value={$settings.sync_mode}
        selectClassName={selectClass}
        onchange={(event) =>
          updateSettings({ sync_mode: event.currentTarget.value as AppSettings["sync_mode"] })}
      >
        <option value="copy">{$t("settings.syncMode.copy")}</option>
        <option value="symlink">{$t("settings.syncMode.symlink")}</option>
      </SelectField>
    </SettingsRow>
  </div>
</SettingsPane>
