import assert from "node:assert/strict";
import { after, test } from "node:test";
import { createServer } from "vite";

const server = await createServer({
  configFile: false,
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: "custom",
});
after(() => server.close());
const actions = await server.ssrLoadModule("/src/lib/repository-actions.ts");

function skill(id, { branch, path = `skills/${id}/SKILL.md`, repo = "Owner/Repo", hubState } = {}) {
  return {
    id,
    name: id,
    hubState: hubState ?? "ok",
    source: { type: "github", repo, url: "", skillPath: path, branch },
  };
}

test("import uses the branch that every skill of the group has", () => {
  assert.equal(
    actions.repositoryImportBranch([skill("a", { branch: "dev" }), skill("b", { branch: "dev" })]),
    "dev"
  );
});

test("import uses the default branch for mixed or missing branches", () => {
  assert.equal(
    actions.repositoryImportBranch([skill("a", { branch: "dev" }), skill("b", { branch: "main" })]),
    null
  );
  assert.equal(
    actions.repositoryImportBranch([skill("a", { branch: "dev" }), skill("b", { branch: null })]),
    null
  );
  assert.equal(actions.repositoryImportBranch([skill("a", { branch: "" })]), null);
  assert.equal(actions.repositoryImportBranch([]), null);
});

test("a detected skill is imported when the library has its repository folder", () => {
  const library = [skill("a", { branch: "main" }), skill("b", { path: "SKILL.md" })];
  const imported = (skillPath, branch) =>
    actions.isImportedGithubSkill(library, "owner/repo", { skill_path: skillPath, branch });
  assert.equal(imported("skills/a/SKILL.md", "main"), true);
  assert.equal(imported("./skills/a/SKILL.md", null), true);
  assert.equal(imported("skills/a/SKILL.md", "next"), false);
  assert.equal(imported("SKILL.md", "next"), true);
  assert.equal(imported("skills/c/SKILL.md", "main"), false);
  assert.equal(
    actions.isImportedGithubSkill(library, "other/repo", {
      skill_path: "skills/a/SKILL.md",
      branch: "main",
    }),
    false
  );
});

test("skills with an update and local changes need the overwrite question", () => {
  const skills = [
    skill("a", { hubState: "modified" }),
    skill("b", { hubState: "name_mismatch" }),
    skill("c", { hubState: "ok" }),
    skill("d", { hubState: "modified" }),
  ];
  const check = [
    { id: "a", updateAvailable: true },
    { id: "b", updateAvailable: true },
    { id: "c", updateAvailable: true },
    { id: "d", updateAvailable: false },
  ];
  assert.deepEqual(actions.localChangeIds(check, skills), ["a", "b"]);
});

test("the report counts updated and up to date skills and lists skipped and failed ones", () => {
  const check = [
    { id: "a", name: "a", updateAvailable: true },
    { id: "b", name: "b", updateAvailable: true },
    { id: "c", name: "c", updateAvailable: false },
    { id: "d", name: "d", updateAvailable: false, error: "rate limited" },
    { id: "e", name: "e", updateAvailable: true },
    { id: "f", name: "f", updateAvailable: true },
  ];
  const pulled = [
    { id: "a", name: "a", status: "updated" },
    { id: "b", name: "b", status: "skipped", message: "Hub copy of 'b' has local changes" },
    { id: "e", name: "e", status: "failed", message: "HTTP error: 404" },
  ];
  const report = actions.summarizeGroupUpdate(check, ["f"], pulled);
  assert.equal(report.updated, 1);
  assert.equal(report.upToDate, 1);
  assert.deepEqual(report.skipped, [
    { id: "f", name: "f", reason: "local_changes" },
    { id: "b", name: "b", reason: "local_changes" },
  ]);
  assert.deepEqual(report.failed, [
    { id: "d", name: "d", error: "rate limited" },
    { id: "e", name: "e", error: "HTTP error: 404" },
  ]);
  assert.equal(actions.reportNeedsDialog(report), true);
});

test("a report without skipped or failed skills needs only a toast", () => {
  const report = actions.summarizeGroupUpdate(
    [
      { id: "a", name: "a", updateAvailable: true },
      { id: "c", name: "c", updateAvailable: false },
    ],
    [],
    [{ id: "a", name: "a", status: "updated" }]
  );
  assert.deepEqual(report, { updated: 1, upToDate: 1, skipped: [], failed: [] });
  assert.equal(actions.reportNeedsDialog(report), false);
  assert.equal(
    actions.reportNeedsDialog({ updated: 0, upToDate: 0, skipped: [], failed: [{ id: "x" }] }),
    true
  );
  assert.equal(
    actions.reportNeedsDialog({ updated: 0, upToDate: 0, skipped: [{ id: "x" }], failed: [] }),
    true
  );
});

test("an unknown installed version needs a pull; an error or a known equal version does not", () => {
  assert.equal(
    actions.needsPull({ updateAvailable: false, remoteSha: null, latestRemoteSha: "x" }),
    true
  );
  assert.equal(
    actions.needsPull({ updateAvailable: false, remoteSha: "x", latestRemoteSha: "x" }),
    false
  );
  assert.equal(
    actions.needsPull({ updateAvailable: false, remoteSha: null, latestRemoteSha: null }),
    false
  );
  assert.equal(actions.needsPull({ updateAvailable: true, error: "rate limited" }), false);
  const report = actions.summarizeGroupUpdate(
    [{ id: "a", name: "a", updateAvailable: false, remoteSha: null, latestRemoteSha: "x" }],
    ["a"],
    []
  );
  assert.deepEqual(report.skipped, [{ id: "a", name: "a", reason: "local_changes" }]);
  assert.equal(report.upToDate, 0);
});

test("the report gives the reason of each skill that the pull skipped", () => {
  const report = actions.summarizeGroupUpdate(
    ["a", "b", "c"].map((id) => ({ id, name: id, updateAvailable: true })),
    [],
    [
      { id: "a", name: "a", status: "skipped", reason: "changed_after_check" },
      { id: "b", name: "b", status: "skipped", reason: "changed_after_check" },
      { id: "c", name: "c", status: "skipped", reason: "local_changes" },
    ],
    ["a"]
  );
  assert.deepEqual(report.skipped, [
    { id: "a", name: "a", reason: "changed_after_confirmation" },
    { id: "b", name: "b", reason: "changed_after_check" },
    { id: "c", name: "c", reason: "local_changes" },
  ]);
});

test("import detection passes a shared branch with a slash separately", () => {
  const target = actions.repositoryImportTarget("o/r", "feature/foo");
  assert.equal(target.url, "https://github.com/o/r/tree/feature/foo");
  assert.deepEqual(actions.githubDetectArgs(target.url, target), {
    path: "o/r",
    branch: "feature/foo",
  });
  // An address the user typed is parsed as before.
  assert.deepEqual(actions.githubDetectArgs(" other/repo ", target), {
    path: "other/repo",
    branch: null,
  });
  const plain = actions.repositoryImportTarget("o/r", null);
  assert.equal(plain.url, "o/r");
  assert.deepEqual(actions.githubDetectArgs("o/r", plain), { path: "o/r", branch: null });
});
