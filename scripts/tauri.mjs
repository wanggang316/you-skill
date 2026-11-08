// Entry point for `npm run tauri`. Runs the Tauri CLI and, for `tauri dev`, points the
// dev window at YOUSKILL_DEV_PORT (see dev-port.mjs). `beforeDevCommand` inherits the
// environment, so Vite listens on the same port.
import cli from "@tauri-apps/cli";
import { tauriArgs } from "./dev-port.mjs";

try {
  await cli.run(tauriArgs(process.argv.slice(2)), "npm run tauri");
} catch (error) {
  cli.logError(error.message);
  process.exit(1);
}
