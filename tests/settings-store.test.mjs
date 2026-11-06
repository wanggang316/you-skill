import assert from "node:assert/strict";
import { after, beforeEach, test } from "node:test";
import { createServer } from "vite";

// Stand-in for the Tauri IPC bridge; window theme calls are accepted and ignored.
let handlers = {};
let stored = null;
globalThis.window = {
  matchMedia: () => ({ matches: false, addEventListener() {}, removeEventListener() {} }),
  __TAURI_INTERNALS__: {
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
    transformCallback: () => 0,
    invoke: async (command, args) => {
      const handler = handlers[command];
      if (handler) return handler(args);
      if (command.startsWith("plugin:")) return null;
      throw new Error(`unexpected command: ${command}`);
    },
  },
};
globalThis.document = { documentElement: { dataset: {} } };
globalThis.localStorage = { setItem() {} };

const server = await createServer({
  configFile: false,
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: "custom",
});
after(() => server.close());
const store = await server.ssrLoadModule("/src/lib/stores/settings.ts");
const { get } = await server.ssrLoadModule("svelte/store");

const errors = console.error;
beforeEach(async () => {
  console.error = () => {};
  stored = {
    language: "en",
    theme: "light",
    sync_mode: "copy",
    shortcuts: {},
    translate_target_language: "",
    translate_model: "",
    translate_display_mode: "bilingual",
    translate_text_style: "none",
    auto_sync: {
      enabled: false,
      backup: true,
      restore: true,
      interval: "1h",
      policy: { repositoryChange: "apply", removal: "ask", conflict: "ask" },
    },
  };
  handlers = {
    get_settings: () => structuredClone(stored),
    update_settings: ({ settings }) => {
      stored = structuredClone(settings);
      return stored;
    },
  };
  await store.loadSettings();
});
after(() => {
  console.error = errors;
});

test("a failed save rejects and shows the stored settings again", async () => {
  handlers.update_settings = () => Promise.reject(new Error("disk full"));
  const auto = get(store.settings).auto_sync;
  await assert.rejects(
    store.updateSettings({ auto_sync: { ...auto, enabled: true } }),
    /disk full/
  );
  assert.equal(get(store.settings).auto_sync.enabled, false);
});

test("a successful save keeps the change", async () => {
  const auto = get(store.settings).auto_sync;
  await store.updateSettings({ auto_sync: { ...auto, enabled: true } });
  assert.equal(get(store.settings).auto_sync.enabled, true);
  assert.equal(stored.auto_sync.enabled, true);
});
