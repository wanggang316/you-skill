<script lang="ts">
  import { getItemScope, setItemScope, type ItemScope } from "$lib/api";
  import { t } from "$lib/i18n";

  /**
   * The library sync scope of one item as a "Shared" switch: on syncs the item to every
   * computer, off keeps it on this computer only. Shown only when the library is connected.
   */
  let {
    itemKey,
    size = "md",
  }: {
    /** `skill:<id>`, `instruction:<id>` or `agent:<id>`. */
    itemKey: string;
    /** `md` matches the detail header buttons, `sm` fits a dense list row. */
    size?: "md" | "sm";
  } = $props();

  let scope = $state<ItemScope | null>(null);
  let saving = $state(false);
  let error = $state("");

  const shared = $derived(scope === "shared");

  $effect(() => {
    const key = itemKey;
    scope = null;
    error = "";
    getItemScope(key)
      .then((value) => {
        if (key === itemKey) scope = value;
      })
      .catch(() => {
        if (key === itemKey) scope = null;
      });
  });

  async function toggle() {
    if (saving || !scope) return;
    const previous = scope;
    const next: ItemScope = shared ? "device" : "shared";
    scope = next;
    saving = true;
    error = "";
    try {
      await setItemScope(itemKey, next);
    } catch (err) {
      scope = previous;
      error = err instanceof Error ? err.message : String(err);
    } finally {
      saving = false;
    }
  }
</script>

{#if scope}
  <button
    type="button"
    role="switch"
    aria-checked={shared}
    aria-label={$t("sync.shared.label")}
    title={error || $t("sync.shared.hint")}
    aria-busy={saving}
    onclick={toggle}
    class={`inline-flex shrink-0 items-center gap-2 ${
      size === "md" ? "h-8 px-1 text-[13px]" : "h-6 px-1 text-[11px]"
    } ${error ? "text-error" : "text-base-content-muted"}`}
  >
    <span class="whitespace-nowrap">{$t("sync.shared.label")}</span>
    <span
      class={`relative inline-flex shrink-0 items-center rounded-full transition-colors ${
        size === "md" ? "h-[18px] w-8" : "h-3.5 w-6"
      } ${shared ? "bg-primary" : "bg-switch-track"}`}
    >
      <span
        class={`bg-switch-thumb absolute left-0.5 rounded-full shadow-sm transition-transform ${
          size === "md" ? "h-3.5 w-3.5" : "h-2.5 w-2.5"
        } ${shared ? (size === "md" ? "translate-x-3.5" : "translate-x-2.5") : ""}`}
      ></span>
    </span>
  </button>
{/if}
