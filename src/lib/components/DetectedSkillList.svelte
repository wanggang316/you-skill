<script>
  import { Check } from "@lucide/svelte";
  import { t } from "../i18n";

  /** @type {{ skills?: import('../api/skills').DetectedSkill[]; selectedSkills?: import('../api/skills').DetectedSkill[]; onToggle?: (skill: import('../api/skills').DetectedSkill) => void; onToggleAll?: () => void; showHeader?: boolean; headerLabel?: string }} */
  let { skills = [], selectedSkills = [], onToggle = () => {}, onToggleAll = () => {}, showHeader = false, headerLabel = "" } = $props();

  const allSelected = $derived(selectedSkills.length === skills.length && skills.length > 0);
  const isMulti = $derived(skills.length > 1);

  /** @param {import('../api/skills').DetectedSkill} skill */
  function isSelected(skill) {
    return selectedSkills.some((s) => s.skill_path === skill.skill_path);
  }
</script>

<div class="space-y-2">
  {#if showHeader}
    <div class="flex items-center justify-between">
      <p class="text-base-content text-sm">
        {headerLabel}
      </p>
      {#if isMulti}
        <label class="text-base-content-muted inline-flex items-center gap-2 text-[13px]">
          <input type="checkbox" checked={allSelected} onchange={onToggleAll} />
          {$t("addSkill.selectAll")}
        </label>
      {/if}
    </div>
  {/if}
  <div class="border-base-300 bg-base-200 max-h-48 space-y-2 overflow-y-auto rounded-xl border p-2">
    {#each skills as skill}
    <button
      class={`flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition ${isSelected(skill) ? "bg-primary text-primary-content" : "bg-base-100 text-base-content hover:bg-base-300"}`}
      onclick={() => onToggle(skill)}
      type="button"
    >
      {#if isMulti}
        <input
          type="checkbox"
          checked={isSelected(skill)}
          class="pointer-events-none"
          tabindex="-1"
        />
      {/if}
      <div class="flex-1">
        <p class="font-medium">{skill.name}</p>
        <p
          class={`text-[10px] ${isSelected(skill) ? "text-primary-content opacity-60" : "text-base-content-subtle opacity-50"}`}
        >
          {skill.skill_path}
        </p>
      </div>
      {#if isSelected(skill) && !isMulti}
        <Check size={16} />
      {/if}
    </button>
  {/each}
  </div>
</div>
