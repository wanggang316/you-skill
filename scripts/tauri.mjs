// Entry point for `npm run tauri`. Runs the Tauri CLI and, for `tauri dev`, points the
// dev window at YOUSKILL_DEV_PORT (see dev-port.mjs). `beforeDevCommand` inherits the
// environment, so Vite listens on the same port.
import cli from "@tauri-apps/cli";
import { readFileSync } from "node:fs";
import { tauriArgs } from "./dev-port.mjs";

const configPath = new URL("../src-tauri/tauri.conf.json", import.meta.url);
const { identifier } = JSON.parse(readFileSync(configPath, "utf8"));

let args;
try {
  args = tauriArgs(process.argv.slice(2), process.env, identifier);
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

try {
  await cli.run(args, "npm run tauri");
} catch (error) {
  cli.logError(error.message);
  process.exit(1);
}
