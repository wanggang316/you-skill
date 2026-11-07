import assert from "node:assert/strict";
import { after, test } from "node:test";
import { createServer } from "vite";

const server = await createServer({
  configFile: false,
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: "custom",
});
after(() => server.close());
const { linkPreviewText, linkAnchor, resolveSkillPath } = await server.ssrLoadModule(
  "/src/lib/utils/markdown-links.ts"
);

test("linkPreviewText shows the full URL of an external link", () => {
  const href = "https://example.com/docs?q=1#part";
  assert.equal(linkPreviewText(href, "SKILL.md"), href);
  assert.equal(linkPreviewText("mailto:a@example.com", "SKILL.md"), "mailto:a@example.com");
});

test("linkPreviewText shows a link to a section of the same file as #<slug>", () => {
  assert.equal(linkPreviewText("#setup", "references/api.md"), "#setup");
});

test("linkPreviewText shows a relative file link as the resolved path in the skill", () => {
  assert.equal(linkPreviewText("references/api.md#auth", "SKILL.md"), "references/api.md#auth");
  assert.equal(linkPreviewText("./api.md", "references/guide.md"), "references/api.md");
  assert.equal(linkPreviewText("../SKILL.md#usage", "references/guide.md"), "SKILL.md#usage");
  assert.equal(linkPreviewText("/scripts/run.sh?x=1", "references/guide.md"), "scripts/run.sh");
});

test("linkPreviewText shows the link as written when the path cannot be resolved", () => {
  assert.equal(linkPreviewText("../../outside.md", "SKILL.md"), "../../outside.md");
  assert.equal(linkPreviewText("docs/a.md#b"), "docs/a.md#b");
});

test("resolveSkillPath resolves a link against the directory of the current file", () => {
  assert.equal(resolveSkillPath("references/guide.md", "./api.md#x"), "references/api.md");
  assert.equal(resolveSkillPath("SKILL.md", "#x"), null);
  assert.equal(resolveSkillPath("SKILL.md", "../x.md"), null);
});

test("linkAnchor returns the decoded section of a link", () => {
  assert.equal(linkAnchor("other.md#auth"), "auth");
  assert.equal(linkAnchor("#%E5%AE%89%E8%A3%85"), "安装");
  assert.equal(linkAnchor("#bad%E5"), "bad%E5");
  assert.equal(linkAnchor("other.md"), "");
});
