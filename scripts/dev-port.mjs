// Dev server port shared by the Vite config and the `npm run tauri` wrapper.
// Set YOUSKILL_DEV_PORT to run `npm run tauri dev` on another port, for example
// in a second worktree. Without it, the port is 5173.

export const DEFAULT_DEV_PORT = 5173;

export function devPort(env = process.env) {
  const value = env.YOUSKILL_DEV_PORT?.trim();
  if (!value) return DEFAULT_DEV_PORT;
  const port = Number(value);
  if (!/^\d+$/.test(value) || port < 1 || port > 65535) {
    throw new Error(`YOUSKILL_DEV_PORT must be a port number from 1 to 65535, got "${value}"`);
  }
  return port;
}

/**
 * Adds config overrides to `tauri dev` when YOUSKILL_DEV_PORT is set: `build.devUrl`
 * for the port, and a per-port identifier. The single-instance plugin is keyed on the
 * identifier, so without it a second dev run hands its launch to the first and exits.
 */
export function tauriArgs(args, env, identifier) {
  if (args[0] !== "dev" || !env.YOUSKILL_DEV_PORT?.trim()) return args;
  const port = devPort(env);
  const config = {
    identifier: `${identifier}.dev${port}`,
    build: { devUrl: `http://localhost:${port}` },
  };
  return ["dev", "--config", JSON.stringify(config), ...args.slice(1)];
}
