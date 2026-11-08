import assert from "node:assert/strict";
import { after, test } from "node:test";
import { createServer } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const server = await createServer({
  configFile: false,
  plugins: [svelte({ configFile: false })],
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: "custom",
});
after(() => server.close());
const { ClickMotion } = await server.ssrLoadModule("/src/lib/components/ui/collapsible.svelte.ts");

/** A group like the location picker's: a search keeps it open whatever the user toggled. */
function pickerGroup() {
  const group = { collapsed: false, search: "" };
  const isOpen = () => group.search !== "" || !group.collapsed;
  const motion = new ClickMotion(isOpen, async () => {});
  const click = () => motion.click(() => (group.collapsed = !group.collapsed));
  return { group, isOpen, motion, click };
}

test("a header click that changes the open state animates until the transition ends", async () => {
  const { isOpen, motion, click } = pickerGroup();
  await click();
  assert.equal(isOpen(), false);
  assert.equal(motion.animate, true);
  motion.done();
  assert.equal(motion.animate, false);
});

test("a header click that does not change the open state does not animate a later change", async () => {
  const { group, isOpen, motion, click } = pickerGroup();
  group.search = "app";
  await click();
  assert.equal(isOpen(), true);
  assert.equal(motion.animate, false);

  group.search = "";
  assert.equal(isOpen(), false);
  assert.equal(motion.animate, false);
});

test("a transition that ends before the click settles stays instant afterwards", async () => {
  const { group } = pickerGroup();
  const isOpen = () => !group.collapsed;
  let motion;
  motion = new ClickMotion(isOpen, async () => motion.done());
  await motion.click(() => (group.collapsed = true));
  assert.equal(motion.animate, false);
});

test("a search during a click transition does not animate", async () => {
  const { group, isOpen, motion, click } = pickerGroup();
  await click();
  assert.equal(isOpen(), false);
  assert.equal(motion.animate, true);

  // The search forces the group open before the collapse ends.
  group.search = "app";
  assert.equal(isOpen(), true);
  assert.equal(motion.animate, false);
  motion.done();

  group.search = "";
  assert.equal(motion.animate, false);
});

test("clearing a search before a reversal ends does not animate the next change", async () => {
  const { group, isOpen, motion, click } = pickerGroup();
  await click();
  assert.equal(motion.start(), true);

  // A search reverses the collapse, and is cleared before that reversal ends.
  group.search = "app";
  assert.equal(motion.start(), false);
  group.search = "";
  assert.equal(isOpen(), false);
  assert.equal(motion.start(), false);
});
