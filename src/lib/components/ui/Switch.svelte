<script lang="ts">
  /** An on/off switch with the theme switch colors. */
  let {
    checked,
    label,
    disabled = false,
    onchange,
  }: {
    checked: boolean;
    /** Accessible name; the visible label is in the row. */
    label: string;
    disabled?: boolean;
    onchange: (checked: boolean) => void;
  } = $props();
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
  class={`relative inline-flex h-[18px] w-8 shrink-0 items-center rounded-full transition-colors disabled:opacity-60 ${
    checked ? "bg-primary" : "bg-switch-track"
  }`}
>
  <span
    class={`bg-switch-thumb absolute left-0.5 h-3.5 w-3.5 rounded-full shadow-sm transition-transform ${
      checked ? "translate-x-3.5" : "translate-x-0"
    }`}
  ></span>
</button>
