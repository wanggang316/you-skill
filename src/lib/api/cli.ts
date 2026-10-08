/**
 * Command line tool API
 *
 * The bundled `youskill` binary and its link on the PATH.
 */

import { apiCall } from "./index";

export interface CliStatus {
  /** The binary shipped with this build; null when the build has none. */
  bundled_path: string | null;
  /** Where the tool is (or would be) installed. */
  install_path: string;
  installed: boolean;
  /** The installed tool is this build's binary. */
  current: boolean;
}

export async function getCliStatus(): Promise<CliStatus> {
  return apiCall<CliStatus>("cli_status");
}

export async function installCli(): Promise<CliStatus> {
  return apiCall<CliStatus>("install_cli");
}

export async function uninstallCli(): Promise<CliStatus> {
  return apiCall<CliStatus>("uninstall_cli");
}
