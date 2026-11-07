// Release-only init script: prevent the native WebView context menu unless the right-click
// target is an enabled text control or contenteditable, where the native menu is the editing
// menu (Cut / Copy / Paste). It listens in the capture phase and does not stop propagation,
// so page `contextmenu` handlers still receive the event.
(() => {
  // `HTMLInputElement.type` reports an invalid or missing type as "text".
  const TEXT_INPUT_TYPES = new Set(["text", "search", "email", "url", "tel", "password", "number"]);

  const isEditable = (target) => {
    if (target instanceof HTMLInputElement) {
      return !target.disabled && TEXT_INPUT_TYPES.has(target.type);
    }
    if (target instanceof HTMLTextAreaElement) {
      return !target.disabled;
    }
    return target instanceof HTMLElement && target.isContentEditable;
  };

  // On macOS, WebKit fires `contextmenu` on mouse down. When no native menu opens, the release
  // of the secondary button reaches the page, and the default action of its `auxclick`
  // activates the control under it: a checkbox toggles, and a `label` or `button` gets a
  // synthetic `click`. Prevent that default action for a press whose menu was suppressed.
  // WebView2 fires `contextmenu` after `auxclick`, so this does not apply there.
  let suppressedThisPress = false;

  window.addEventListener(
    "mousedown",
    () => {
      suppressedThisPress = false;
    },
    true
  );

  window.addEventListener(
    "contextmenu",
    (event) => {
      if (!isEditable(event.composedPath()[0])) {
        event.preventDefault();
        suppressedThisPress = true;
      }
    },
    true
  );

  window.addEventListener(
    "auxclick",
    (event) => {
      if (suppressedThisPress) {
        suppressedThisPress = false;
        event.preventDefault();
      }
    },
    true
  );
})();
