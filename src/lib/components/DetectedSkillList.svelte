<script>
  import { Check } from "@lucide/svelte";

  /** @type {{ skills?: import('../api/skills').DetectedSkill[]; selectedSkill?: import('../api/skills').DetectedSkill | null; onSelect?: (skill: import('../api/skills').DetectedSkill) => void }} */
  let { skills = [], selectedSkill = null, onSelect = () => {} } = $props();
</script>

<div class="border-base-300 bg-base-200 max-h-48 space-y-2 overflow-y-auto rounded-xl border p-2">
  {#each skills as skill}
    <button
      class={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition ${selectedSkill?.skill_path === skill.skill_path ? "bg-primary text-primary-content" : "bg-base-100 text-base-content hover:bg-base-300"}`}
      onclick={() => onSelect(skill)}
      type="button"
    >
      <div>
        <p class="font-medium">{skill.name}</p>
        <p
          class={`text-xs ${selectedSkill?.skill_path === skill.skill_path ? "text-primary-content opacity-70" : "text-base-content-subtle opacity-80"}`}
        >
          {skill.skill_path}
        </p>
      </div>
      {#if selectedSkill?.skill_path === skill.skill_path}
        <Check size={16} />
      {/if}
    </button>
  {/each}
</div>
