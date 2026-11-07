import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import vm from "node:vm";

const script = readFileSync(
  new URL("../src-tauri/src/suppress_context_menu.js", import.meta.url),
  "utf8"
);

class Element {}
class HTMLElement extends Element {
  isContentEditable = false;
}
class HTMLInputElement extends HTMLElement {
  constructor(type = "text", disabled = false) {
    super();
    this.type = type;
    this.disabled = disabled;
  }
}
class HTMLTextAreaElement extends HTMLElement {
  constructor(disabled = false) {
    super();
    this.disabled = disabled;
  }
}

/**
 * Runs the init script in a fake window. `dispatch(type, init)` sends one event through the
 * capture listeners and returns whether the default action was prevented and whether page
 * handlers were cut off.
 */
function loadScript() {
  const listeners = new Map();
  const window = {
    addEventListener: (type, listener, capture) => {
      assert.equal(capture, true, `${type} listener must use the capture phase`);
      listeners.set(type, [...registered(type), listener]);
    },
  };
  function registered(type) {
    return listeners.get(type) ?? [];
  }
  vm.runInNewContext(script, {
    window,
    Element,
    HTMLElement,
    HTMLInputElement,
    HTMLTextAreaElement,
  });
  const dispatch = (type, { target = new HTMLElement(), button = 0 } = {}) => {
    let prevented = false;
    let stopped = false;
    for (const listener of registered(type)) {
      listener({
        button,
        composedPath: () => [target],
        preventDefault: () => (prevented = true),
        stopPropagation: () => (stopped = true),
        stopImmediatePropagation: () => (stopped = true),
      });
    }
    return { prevented, stopped };
  };
  return dispatch;
}

/** Dispatches `contextmenu` on a target and returns which menu the WebView would show. */
function menuFor(target) {
  const { prevented, stopped } = loadScript()("contextmenu", { target, button: 2 });
  assert.equal(stopped, false, "page handlers must still receive the event");
  return prevented ? "blocked" : "native";
}

/**
 * Sends a secondary press in the macOS WebKit order (contextmenu on mouse down) and returns
 * what happens to the default action of its `auxclick`, which activates the control.
 */
function webKitRightClick(target) {
  const dispatch = loadScript();
  dispatch("mousedown", { target, button: 2 });
  dispatch("contextmenu", { target, button: 2 });
  dispatch("mouseup", { target, button: 2 });
  return dispatch("auxclick", { target, button: 2 });
}

function editable() {
  const element = new HTMLElement();
  element.isContentEditable = true;
  return element;
}

test("non-editable targets get no native menu", () => {
  assert.equal(menuFor(new HTMLElement()), "blocked");
  assert.equal(menuFor(new Element()), "blocked");
  assert.equal(menuFor(null), "blocked");
});

test("text inputs, textareas and contenteditable keep the native menu", () => {
  for (const type of ["text", "search", "email", "url", "tel", "password", "number"]) {
    assert.equal(menuFor(new HTMLInputElement(type)), "native", type);
  }
  assert.equal(menuFor(new HTMLTextAreaElement()), "native");
  assert.equal(menuFor(editable()), "native");
});

test("non-text and disabled inputs get no native menu", () => {
  for (const type of ["checkbox", "radio", "button", "submit", "range", "color", "file"]) {
    assert.equal(menuFor(new HTMLInputElement(type)), "blocked", type);
  }
  assert.equal(menuFor(new HTMLInputElement("text", true)), "blocked");
  assert.equal(menuFor(new HTMLTextAreaElement(true)), "blocked");
});

test("a suppressed right-click does not activate the control under it", () => {
  assert.deepEqual(webKitRightClick(new HTMLInputElement("checkbox")), {
    prevented: true,
    stopped: false,
  });
});

test("auxclick keeps its default action when no menu was suppressed", () => {
  // A right-click in a text field keeps the native menu.
  assert.deepEqual(webKitRightClick(new HTMLInputElement("text")), {
    prevented: false,
    stopped: false,
  });

  // WebView2 order: auxclick comes before contextmenu.
  const webView2 = loadScript();
  webView2("mousedown", { button: 2 });
  webView2("mouseup", { button: 2 });
  assert.deepEqual(webView2("auxclick", { button: 2 }), { prevented: false, stopped: false });
  webView2("contextmenu", { button: 2 });

  // A later middle click after a keyboard menu (Shift+F10) or an earlier suppressed press.
  const later = loadScript();
  later("contextmenu");
  later("mousedown", { button: 1 });
  later("mouseup", { button: 1 });
  assert.deepEqual(later("auxclick", { button: 1 }), { prevented: false, stopped: false });
});
