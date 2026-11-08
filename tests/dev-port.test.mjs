import assert from "node:assert/strict";
import { test } from "node:test";
import { DEFAULT_DEV_PORT, devPort, tauriArgs } from "../scripts/dev-port.mjs";

test("devPort uses 5173 when YOUSKILL_DEV_PORT is unset or empty", () => {
  assert.equal(DEFAULT_DEV_PORT, 5173);
  assert.equal(devPort({}), 5173);
  assert.equal(devPort({ YOUSKILL_DEV_PORT: "" }), 5173);
});

test("devPort reads YOUSKILL_DEV_PORT", () => {
  assert.equal(devPort({ YOUSKILL_DEV_PORT: "5180" }), 5180);
  assert.equal(devPort({ YOUSKILL_DEV_PORT: " 65535 " }), 65535);
});

test("devPort rejects a value that is not a valid port", () => {
  for (const value of ["abc", "0", "65536", "51.5", "-1", "5180x"]) {
    assert.throws(() => devPort({ YOUSKILL_DEV_PORT: value }), /YOUSKILL_DEV_PORT/, value);
  }
});

test("tauriArgs leaves arguments unchanged without YOUSKILL_DEV_PORT", () => {
  assert.deepEqual(tauriArgs(["dev"], {}), ["dev"]);
  assert.deepEqual(tauriArgs(["build"], {}), ["build"]);
});

test("tauriArgs points the dev window at the chosen port", () => {
  const args = tauriArgs(["dev", "--", "--", "--flag"], { YOUSKILL_DEV_PORT: "5180" });
  assert.deepEqual(args, [
    "dev",
    "--config",
    JSON.stringify({ build: { devUrl: "http://localhost:5180" } }),
    "--",
    "--",
    "--flag",
  ]);
});

test("tauriArgs changes only the dev command", () => {
  const env = { YOUSKILL_DEV_PORT: "5180" };
  assert.deepEqual(tauriArgs(["build"], env), ["build"]);
  assert.deepEqual(tauriArgs(["android", "dev"], env), ["android", "dev"]);
  assert.deepEqual(tauriArgs([], env), []);
});
