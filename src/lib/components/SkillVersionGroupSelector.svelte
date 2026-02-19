<script lang="ts">
  import type { SourceVersionGroup } from "../api/skills";

  let {
    groups = [],
    selectedSourcePath = $bindable(""),
    radioName = "versionGroup",
  } = $props<{
    groups?: SourceVersionGroup[];
    selectedSourcePath?: string;
    radioName?: string;
  }>();
</script>

<div class="space-y-3">
  {#each groups as group}
    <label
      class="border-base-300 bg-base-100 hover:bg-base-200 flex w-full cursor-pointer items-start gap-3 rounded-lg border px-3 py-2"
    >
      <input
        class="mt-1 shrink-0"
        type="radio"
        name={radioName}
        value={group.source_path}
        checked={selectedSourcePath === group.source_path}
        onchange={() => {
          selectedSourcePath = group.source_path;
        }}
      />
      <div class="min-w-0 flex-1">
        <div class="text-base-content mb-1 text-sm font-medium">{group.version}</div>
        <div class="space-y-1">
          {#each group.paths as path}
            <div class="text-base-content-subtle break-all text-xs">{path}</div>
          {/each}
        </div>
      </div>
    </label>
  {/each}
</div>
