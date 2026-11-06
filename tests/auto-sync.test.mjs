import assert from "node:assert/strict";
import { after, test } from "node:test";
import { get, writable } from "svelte/store";
import { createServer } from "vite";

const server = await createServer({
  configFile: false,
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: "custom",
});
after(() => server.close());
const { startAutoSyncStatus } = await server.ssrLoadModule("/src/lib/stores/auto-sync.ts");
const { createLatestWriter } = await server.ssrLoadModule("/src/lib/utils/latest-writer.ts");

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

function deferred() {
  let resolve;
  const promise = new Promise((done) => (resolve = done));
  return { promise, resolve };
}

const failed = { at: "t1", error: "network down" };
const older = { at: "t0", outcome: { result: "up_to_date", ask: [], restored: false } };

function setup() {
  const status = writable(null);
  const registered = deferred();
  const read = deferred();
  const calls = [];
  let handler = null;
  const stop = startAutoSyncStatus({
    status,
    listen: (next) => {
      calls.push("listen");
      handler = next;
      return registered.promise;
    },
    getStatus: () => {
      calls.push("read");
      return read.promise;
    },
    onRestored: () => calls.push("restored"),
  });
  return { status, registered, read, calls, emit: (value) => handler(value), stop };
}

test("reads the last status only after the listener is registered", async () => {
  const { status, registered, read, calls } = setup();
  await settle();
  assert.deepEqual(calls, ["listen"]);
  registered.resolve(() => {});
  await settle();
  assert.deepEqual(calls, ["listen", "read"]);
  read.resolve(failed);
  await settle();
  assert.deepEqual(get(status), failed);
});

test("an event during the read wins over the read result", async () => {
  const { status, registered, read, emit } = setup();
  registered.resolve(() => {});
  await settle();
  emit(failed);
  read.resolve(older);
  await settle();
  assert.deepEqual(get(status), failed);
});

test("an auto restore reloads the library", async () => {
  const { registered, calls, emit } = setup();
  registered.resolve(() => {});
  await settle();
  emit({ at: "t2", outcome: { result: "synced", ask: [], restored: true, agentsChanged: false } });
  assert.ok(calls.includes("restored"));
});

test("overlapping saves end with the latest value and never run at once", async () => {
  const writes = [];
  const pending = [];
  let active = 0;
  let maxActive = 0;
  const save = createLatestWriter(async (value) => {
    active += 1;
    maxActive = Math.max(maxActive, active);
    writes.push(value);
    const gate = deferred();
    pending.push(gate);
    await gate.promise;
    active -= 1;
  });
  const first = save("a");
  save("b");
  const last = save("c");
  await settle();
  pending.shift().resolve();
  await settle();
  pending.shift().resolve();
  await Promise.all([first, last]);
  assert.deepEqual(writes, ["a", "c"]);
  assert.equal(maxActive, 1);
});
