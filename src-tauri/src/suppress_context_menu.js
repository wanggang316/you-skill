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

  window.addEventListener(
    "contextmenu",
    (event) => {
      if (!isEditable(event.composedPath()[0])) {
        event.preventDefault();
      }
    },
    true
  );
})();
