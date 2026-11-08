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

const IDENTIFIER = "com.gumpw.youskill";

test("tauriArgs leaves arguments unchanged without YOUSKILL_DEV_PORT", () => {
  assert.deepEqual(tauriArgs(["dev"], {}, IDENTIFIER), ["dev"]);
  assert.deepEqual(tauriArgs(["build"], {}, IDENTIFIER), ["build"]);
});

test("tauriArgs points the dev window at the chosen port with a per-port identifier", () => {
  const args = tauriArgs(["dev", "--", "--", "--flag"], { YOUSKILL_DEV_PORT: "5180" }, IDENTIFIER);
  assert.deepEqual(args, [
    "dev",
    "--config",
    JSON.stringify({
      identifier: "com.gumpw.youskill.dev5180",
      build: { devUrl: "http://localhost:5180" },
    }),
    "--",
    "--",
    "--flag",
  ]);
});

test("tauriArgs gives runs on different ports different single-instance identifiers", () => {
  const identifier = (port) =>
    JSON.parse(tauriArgs(["dev"], { YOUSKILL_DEV_PORT: port }, IDENTIFIER)[2]).identifier;
  assert.notEqual(identifier("5180"), identifier("5181"));
  assert.notEqual(identifier("5180"), IDENTIFIER);
});

test("tauriArgs changes only the dev command", () => {
  const env = { YOUSKILL_DEV_PORT: "5180" };
  assert.deepEqual(tauriArgs(["build"], env, IDENTIFIER), ["build"]);
  assert.deepEqual(tauriArgs(["android", "dev"], env, IDENTIFIER), ["android", "dev"]);
  assert.deepEqual(tauriArgs([], env, IDENTIFIER), []);
});
