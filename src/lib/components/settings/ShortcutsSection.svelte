<script lang="ts">
  import { RotateCcw } from "@lucide/svelte";
  import ShortcutKeys from "$lib/components/ui/ShortcutKeys.svelte";
  import { t } from "$lib/i18n";
  import {
    bindingKeys,
    DEFAULT_SHORTCUTS,
    eventToBinding,
    HINT_KEY,
    isReservedBinding,
    resetShortcuts,
    setShortcut,
    SHORTCUT_ACTIONS,
    SHORTCUT_LABEL_KEYS,
    shortcuts,
    type ShortcutAction,
  } from "$lib/shortcuts";
  import { settings } from "$lib/stores/settings";
  import SettingsPane from "./SettingsPane.svelte";
  import SettingsRow from "./SettingsRow.svelte";

  /** Action whose new keys are being recorded. */
  let recording = $state<ShortcutAction | null>(null);
  let error = $state("");

  const hasChanges = $derived(Object.keys($settings.shortcuts ?? {}).length > 0);
  const hintKey = HINT_KEY === "Meta" ? "⌘" : "Ctrl";
  const label = (action: ShortcutAction) => $t(SHORTCUT_LABEL_KEYS[action]);

  function startRecording(action: ShortcutAction) {
    recording = recording === action ? null : action;
    error = "";
  }

  /** Runs before the top bar shortcuts, so a recorded key does not also trigger one. */
  function handleKeydown(event: KeyboardEvent) {
    const action = recording;
    if (!action) return;
    const plain = !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;
    // Tab moves on; the blur ends recording.
    if (event.key === "Tab" && plain) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    if (event.key === "Escape" && plain) {
      recording = null;
      return;
    }

    const binding = eventToBinding(event);
    if (!binding) return;
    recording = null;
    const keys = bindingKeys(binding).join("");
    if (isReservedBinding(binding)) {
      error = $t("settings.shortcuts.reserved", { keys });
      return;
    }
    const owner = SHORTCUT_ACTIONS.find(
      (other) => other !== action && $shortcuts[other] === binding
    );
    if (owner) {
      error = $t("settings.shortcuts.conflict", { keys, action: label(owner) });
      return;
    }
    error = "";
    void setShortcut(action, binding);
  }

  function reset(action: ShortcutAction) {
    error = "";
    void setShortcut(action, null);
  }

  function resetAll() {
    error = "";
    recording = null;
    void resetShortcuts();
  }
</script>

<svelte:window onkeydowncapture={handleKeydown} />

<SettingsPane
  title={$t("settings.shortcuts.title")}
  description={$t("settings.shortcuts.description", { key: hintKey })}
>
  {#snippet actions()}
    <button class="settings-button" type="button" onclick={resetAll} disabled={!hasChanges}>
      {$t("settings.shortcuts.resetAll")}
    </button>
  {/snippet}

  <div class="settings-card">
    {#each SHORTCUT_ACTIONS as action (action)}
      {@const active = recording === action}
      {@const changed = $shortcuts[action] !== DEFAULT_SHORTCUTS[action]}
      <SettingsRow label={label(action)}>
        {#if changed}
          <button
            class="text-base-content-muted hover:bg-base-content/[0.06] hover:text-base-content inline-flex size-8 items-center justify-center rounded-lg transition"
            type="button"
            onclick={() => reset(action)}
            title={$t("settings.shortcuts.reset")}
            aria-label={$t("settings.shortcuts.reset")}
          >
            <RotateCcw size={14} />
          </button>
        {/if}
        <button
          class={`settings-button min-w-28 justify-center ${
            active ? "border-primary ring-primary/20 ring-2" : ""
          }`}
          type="button"
          onclick={() => startRecording(action)}
          onblur={() => {
            if (recording === action) recording = null;
          }}
          aria-label={$t("settings.shortcuts.change", { action: label(action) })}
          aria-pressed={active}
        >
          {#if active}
            <span class="text-base-content-muted text-[12px]"
              >{$t("settings.shortcuts.record")}</span
            >
          {:else}
            <ShortcutKeys binding={$shortcuts[action]} />
          {/if}
        </button>
      </SettingsRow>
    {/each}
  </div>

  {#if error}
    <p class="text-error px-1 text-xs">{error}</p>
  {/if}
</SettingsPane>
