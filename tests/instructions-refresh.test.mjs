import assert from "node:assert/strict";
import { after, test } from "node:test";
import { createServer } from "vite";

// Stand-in for the Tauri IPC bridge.
let handlers = {};
globalThis.window = {
  __TAURI_INTERNALS__: {
    invoke: async (command) => {
      const handler = handlers[command];
      if (!handler) throw new Error(`unexpected command: ${command}`);
      return handler();
    },
  },
};

const server = await createServer({
  configFile: false,
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: "custom",
});
after(() => server.close());
const store = await server.ssrLoadModule("/src/lib/stores/instructions.ts");
const { get } = await server.ssrLoadModule("svelte/store");

function deferred() {
  let resolve;
  const promise = new Promise((done) => (resolve = done));
  return { promise, resolve };
}

const template = { id: "t1", name: "Rules" };

test("a fresh refresh reads again after a run that started before it", async () => {
  const reads = [deferred(), deferred()];
  let calls = 0;
  handlers.list_instructions = () => reads[calls++].promise;
  handlers.list_instruction_files = () => Promise.resolve([]);
  const early = store.refreshInstructions();
  const fresh = store.refreshInstructions({ fresh: true });
  reads[0].resolve([]);
  await early;
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(calls, 2);
  reads[1].resolve([template]);
  await fresh;
  assert.deepEqual(
    get(store.instructions).map((item) => item.id),
    ["t1"]
  );
});

test("a plain refresh joins the running one", async () => {
  const read = deferred();
  let calls = 0;
  handlers.list_instructions = () => {
    calls += 1;
    return read.promise;
  };
  handlers.list_instruction_files = () => Promise.resolve([]);
  const first = store.refreshInstructions();
  const second = store.refreshInstructions();
  read.resolve([template]);
  await Promise.all([first, second]);
  assert.equal(calls, 1);
});
