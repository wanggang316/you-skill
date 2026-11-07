import assert from "node:assert/strict";
import { after, test } from "node:test";
import { createServer } from "vite";

const server = await createServer({
  configFile: false,
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: "custom",
});
after(() => server.close());
const { removeEmptySections, renderMarkdown, renderBilingualMarkdown, renderTranslatedMarkdown } =
  await server.ssrLoadModule("/src/lib/utils/markdown.ts");

const headingIds = (html) =>
  [...html.matchAll(/<h[1-6]( id="([^"]*)")?>/g)].map((m) => m[2] ?? null);

test("removeEmptySections drops changelog categories without entries", () => {
  const notes = [
    "### Breaking Changes",
    "",
    "### Added",
    "",
    "### Fixed",
    "",
    "- Recover empty skill names.",
    "",
    "### Removed",
    "",
  ].join("\n");
  assert.equal(removeEmptySections(notes), "### Fixed\n\n- Recover empty skill names.");
});

test("removeEmptySections drops a parent heading whose sub-sections are all empty", () => {
  const notes = "## 1.0\n\n### Added\n\n## 0.9\n\n### Fixed\n\n- Fix.\n";
  assert.equal(removeEmptySections(notes), "## 0.9\n\n### Fixed\n\n- Fix.");
});

test("removeEmptySections keeps text before the first heading and code blocks", () => {
  const notes = "Intro.\n\n### Changed\n\n```sh\n### not a heading\n```\n\n### Removed\n";
  assert.equal(
    removeEmptySections(notes),
    "Intro.\n\n### Changed\n\n```sh\n### not a heading\n```"
  );
});

test("removeEmptySections returns an empty string when no section has content", () => {
  assert.equal(removeEmptySections("### Added\n\n### Fixed\n"), "");
  assert.equal(removeEmptySections(""), "");
});

test("renderMarkdown gives headings GitHub slug ids with suffixes for duplicates", () => {
  const html = renderMarkdown("# Hello, World!\n\n## Setup\n\n## Setup\n\n### Setup\n");
  assert.deepEqual(headingIds(html), ["hello-world", "setup", "setup-1", "setup-2"]);
});

test("renderMarkdown keeps CJK text in heading slugs", () => {
  const html = renderMarkdown("## 安装 指南\n\n## 使用：API\n");
  assert.deepEqual(headingIds(html), ["安装-指南", "使用api"]);
});

test("renderMarkdown makes ids from the heading text without markup", () => {
  const html = renderMarkdown("## The `run` *command* & [link](x.md)\n");
  assert.deepEqual(headingIds(html), ["the-run-command--link"]);
});

test("renderMarkdown resets duplicate counters between documents", () => {
  assert.deepEqual(headingIds(renderMarkdown("# A\n")), ["a"]);
  assert.deepEqual(headingIds(renderMarkdown("# A\n")), ["a"]);
});

test("renderBilingualMarkdown gives ids only to original headings", () => {
  const html = renderBilingualMarkdown(
    "# Intro\n\nText.\n\n## Usage\n\n## Usage\n",
    "# 简介\n\n文本。\n\n## 用法\n\n## 用法\n"
  );
  assert.deepEqual(headingIds(html), ["intro", "usage", "usage-1"]);
  assert.match(
    html,
    /<h1 id="intro">Intro<br><span class="markdown-translation">简介<\/span><\/h1>/
  );
});

test("renderBilingualMarkdown gives no id to a translation-only heading", () => {
  const html = renderBilingualMarkdown("Text.\n", "文本。\n\n## 额外\n");
  assert.deepEqual(headingIds(html), [null]);
});

test("renderTranslatedMarkdown uses original ids when the heading counts match", () => {
  const html = renderTranslatedMarkdown(
    "# Intro\n\n- item\n\n  ## Nested\n\n## Intro\n",
    "# 简介\n\n- 条目\n\n  ## 嵌套\n\n## 简介\n"
  );
  assert.deepEqual(headingIds(html), ["intro", "nested", "intro-1"]);
});

test("renderTranslatedMarkdown makes ids from the translation when the heading counts differ", () => {
  const html = renderTranslatedMarkdown("# Intro\n\n## Usage\n", "# 简介\n\n## 用法\n\n## 用法\n");
  assert.deepEqual(headingIds(html), ["简介", "用法", "用法-1"]);
});
