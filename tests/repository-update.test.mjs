import assert from "node:assert/strict";
import { after, beforeEach, test } from "node:test";
import { createServer } from "vite";

// Stand-in for the Tauri IPC bridge; each test sets the command handlers it needs.
let handlers = {};
let calls = [];
globalThis.window = {
  __TAURI_INTERNALS__: {
    invoke: async (command, args) => {
      calls.push({ command, args });
      const handler = handlers[command];
      if (!handler) throw new Error(`unexpected command: ${command}`);
      return handler(args);
    },
  },
};

const server = await createServer({
  configFile: false,
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: "custom",
});
after(() => server.close());
const { updateRepositoryGroup } = await server.ssrLoadModule(
  "/src/lib/stores/repository-update.ts"
);
const modals = await server.ssrLoadModule("/src/lib/stores/modals.ts");

function skill(id, hubState = "ok") {
  return {
    id,
    name: id,
    sourceKey: `github:o/r:skills/${id}`,
    hubState,
    hubHash: `hash-${id}`,
    source: { type: "github", repo: "o/r", url: "", skillPath: `skills/${id}/SKILL.md` },
    installs: [],
  };
}

const library = [skill("a", "modified"), skill("b"), skill("c")];
const check = [
  { id: "a", name: "a", updateAvailable: true },
  { id: "b", name: "b", updateAvailable: true },
  { id: "c", name: "c", updateAvailable: false },
];

/** Answer the overwrite question when it opens; records the items it asked about. */
function answer(overwrite) {
  const asked = [];
  const unsubscribe = modals.forceModal.subscribe((state) => {
    if (!state.open) return;
    asked.push(state.blockers);
    queueMicrotask(() => {
      modals.closeForceModal();
      if (overwrite) void state.onConfirm();
      else state.onCancel();
    });
  });
  return { asked, unsubscribe };
}

beforeEach(() => {
  calls = [];
  handlers = {
    check_source_updates: () => check,
    list_hub_skills: () => library,
    pull_github_sources: ({ ids }) => ids.map((id) => ({ id, name: id, status: "updated" })),
  };
});

test("overwrite all pulls the skills with local changes with force", async () => {
  const { asked, unsubscribe } = answer(true);
  const report = await updateRepositoryGroup(["a", "b", "c"]);
  unsubscribe();

  assert.deepEqual(asked, [["a"]]);
  assert.deepEqual(calls.find((c) => c.command === "check_source_updates").args, {
    ids: ["a", "b", "c"],
  });
  const pulls = calls.filter((c) => c.command === "pull_github_sources");
  assert.equal(pulls.length, 1);
  assert.deepEqual(pulls[0].args, {
    ids: ["a", "b"],
    forced: [{ id: "a", hubHash: "hash-a" }],
  });
  assert.deepEqual(report, { updated: 2, upToDate: 1, skipped: [], failed: [] });
});

test("skip all leaves the skills with local changes out of the pull", async () => {
  const { unsubscribe } = answer(false);
  const report = await updateRepositoryGroup(["a", "b", "c"]);
  unsubscribe();

  const pull = calls.find((c) => c.command === "pull_github_sources");
  assert.deepEqual(pull.args, { ids: ["b"], forced: [] });
  assert.deepEqual(report.skipped, [{ id: "a", name: "a" }]);
  assert.equal(report.updated, 1);
});

test("a group without updates asks nothing and pulls nothing", async () => {
  handlers.check_source_updates = () => check.map((item) => ({ ...item, updateAvailable: false }));
  const { asked, unsubscribe } = answer(true);
  const report = await updateRepositoryGroup(["a", "b", "c"]);
  unsubscribe();

  assert.deepEqual(asked, []);
  assert.equal(
    calls.some((c) => c.command === "pull_github_sources"),
    false
  );
  assert.deepEqual(report, { updated: 0, upToDate: 3, skipped: [], failed: [] });
});

test("a failed check reports every skill as failed", async () => {
  handlers.check_source_updates = () => {
    throw new Error("offline");
  };
  const report = await updateRepositoryGroup(["a", "b"]);
  assert.deepEqual(
    report.failed.map((item) => item.id),
    ["a", "b"]
  );
  assert.match(report.failed[0].error, /offline/);
});

test("a skill without a recorded version is pulled, not counted as up to date", async () => {
  handlers.check_source_updates = () => [
    { id: "b", name: "b", updateAvailable: false, remoteSha: null, latestRemoteSha: "new" },
    { id: "c", name: "c", updateAvailable: false, remoteSha: "same", latestRemoteSha: "same" },
  ];
  const report = await updateRepositoryGroup(["b", "c"]);
  const pull = calls.find((c) => c.command === "pull_github_sources");
  assert.deepEqual(pull.args, { ids: ["b"], forced: [] });
  assert.deepEqual(report, { updated: 1, upToDate: 1, skipped: [], failed: [] });
});
