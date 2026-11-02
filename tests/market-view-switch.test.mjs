import assert from "node:assert/strict";
import { after, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const server = await createServer({
  configFile: false,
  plugins: [svelte({ configFile: false })],
  resolve: { alias: { $lib: fileURLToPath(new URL("../src/lib", import.meta.url)) } },
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: "custom",
});
after(() => server.close());
const { render } = await server.ssrLoadModule("svelte/server");
const { default: MarketViewSwitch } = await server.ssrLoadModule(
  "/src/lib/components/MarketViewSwitch.svelte"
);
const { createMarketplace } = await server.ssrLoadModule("/src/lib/stores/market.ts");

const tabs = [
  { value: "categories", label: "Categories" },
  { value: "repositories", label: "Hot repos" },
];
const html = (favorites) =>
  render(MarketViewSwitch, {
    props: {
      tabs,
      tab: "categories",
      favorites,
      favoritesLabel: "Favorites",
      onSelectTab: () => {},
      onShowFavorites: () => {},
    },
  }).body;
const favoritesButton = (body) =>
  body.match(/<button[^>]*data-favorites-button[^>]*>[\s\S]*?<\/button>/)[0];

test("the favorites button is labeled, shows no count and an outline bookmark when browsing", () => {
  const button = favoritesButton(html(false));
  assert.match(button, /aria-label="Favorites"/);
  assert.match(button, /title="Favorites"/);
  assert.match(button, /aria-pressed="false"/);
  assert.match(button, /lucide-bookmark/);
  assert.match(button, /fill="none"/);
  assert.doesNotMatch(button.replace(/<[^>]*>/g, ""), /\d/);
});

test("the favorites button is active with a filled bookmark in the favorites view", () => {
  const body = html(true);
  const button = favoritesButton(body);
  assert.match(button, /aria-pressed="true"/);
  assert.match(button, /fill="currentColor"/);
  // No tab looks selected while the favorites view shows.
  assert.equal(body.match(/text-base-content-muted" type="button" data-tab-value/g).length, 2);
});

test("the button handler enters the favorites view and a tab handler leaves it", async () => {
  const market = createMarketplace({
    fetchSkills: async () => ({ skills: [], total: 0, has_more: false }),
    fetchCategories: async () => [],
    fetchRepositories: async () => [],
  });
  let state;
  market.state.subscribe((value) => (state = value));
  await market.initialize();
  let selected = null;
  const props = {
    tabs,
    get tab() {
      return state.tab;
    },
    get favorites() {
      return state.view === "favorites";
    },
    favoritesLabel: "Favorites",
    onSelectTab: (value) => (selected = market.selectTab(value)),
    onShowFavorites: () => (selected = market.showFavorites()),
  };
  // The page passes these handlers to the component; invoke them as a click would.
  props.onShowFavorites();
  await selected;
  assert.equal(state.view, "favorites");
  assert.match(favoritesButton(render(MarketViewSwitch, { props }).body), /aria-pressed="true"/);
  props.onSelectTab("categories");
  await selected;
  assert.equal(state.view, "browse");
  assert.match(favoritesButton(render(MarketViewSwitch, { props }).body), /aria-pressed="false"/);
});
