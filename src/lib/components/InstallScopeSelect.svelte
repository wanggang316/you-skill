<script lang="ts">
  import { t } from "../i18n";
  import SelectField from "./ui/SelectField.svelte";

  let {
    value = $bindable("global"),
    projects = [],
    disabled = false,
    className = "",
  } = $props<{
    value?: string;
    projects?: Array<{ name: string; path: string }>;
    disabled?: boolean;
    className?: string;
  }>();
</script>

<SelectField bind:value {disabled} {className} selectClassName="w-32">
  <option value="global">{$t("installScope.global")}</option>
  {#if projects.length > 0}
    <option value="" disabled>------</option>
  {/if}
  {#each projects as project}
    <option value={`project:${encodeURIComponent(project.path)}`}>{project.name}</option>
  {/each}
</SelectField>
