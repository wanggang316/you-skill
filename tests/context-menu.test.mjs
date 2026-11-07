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

/** Runs the init script and returns a function that dispatches `contextmenu` on a target. */
function loadScript() {
  const listeners = [];
  const window = {
    addEventListener: (type, listener, capture) => listeners.push({ type, listener, capture }),
  };
  vm.runInNewContext(script, {
    window,
    Element,
    HTMLElement,
    HTMLInputElement,
    HTMLTextAreaElement,
  });
  assert.equal(listeners.length, 1);
  assert.equal(listeners[0].type, "contextmenu");
  assert.equal(listeners[0].capture, true);
  return (target) => {
    let prevented = false;
    let stopped = false;
    listeners[0].listener({
      composedPath: () => [target],
      preventDefault: () => (prevented = true),
      stopPropagation: () => (stopped = true),
      stopImmediatePropagation: () => (stopped = true),
    });
    assert.equal(stopped, false, "page handlers must still receive the event");
    return prevented ? "blocked" : "native";
  };
}

function editable() {
  const element = new HTMLElement();
  element.isContentEditable = true;
  return element;
}

test("non-editable targets get no native menu", () => {
  const dispatch = loadScript();
  assert.equal(dispatch(new HTMLElement()), "blocked");
  assert.equal(dispatch(new Element()), "blocked");
  assert.equal(dispatch(null), "blocked");
});

test("text inputs, textareas and contenteditable keep the native menu", () => {
  const dispatch = loadScript();
  for (const type of ["text", "search", "email", "url", "tel", "password", "number"]) {
    assert.equal(dispatch(new HTMLInputElement(type)), "native", type);
  }
  assert.equal(dispatch(new HTMLTextAreaElement()), "native");
  assert.equal(dispatch(editable()), "native");
});

test("non-text and disabled inputs get no native menu", () => {
  const dispatch = loadScript();
  for (const type of ["checkbox", "radio", "button", "submit", "range", "color", "file"]) {
    assert.equal(dispatch(new HTMLInputElement(type)), "blocked", type);
  }
  assert.equal(dispatch(new HTMLInputElement("text", true)), "blocked");
  assert.equal(dispatch(new HTMLTextAreaElement(true)), "blocked");
});
