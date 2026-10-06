/**
 * GitHub API
 *
 * GitHub account detection through the local GitHub CLI (gh)
 */

import { apiCall } from "./index";

export interface GhAccount {
  login: string;
  /** The account gh uses by default for github.com. */
  active: boolean;
  /** "success" when the stored token is valid. */
  state: string;
  scopes: string;
  token_source: string;
}

export interface GithubAuthStatus {
  gh_installed: boolean;
  gh_path: string | null;
  accounts: GhAccount[];
  /** Account used for GitHub requests. */
  selected_login: string | null;
  error: string | null;
}

export async function getGithubAuthStatus(): Promise<GithubAuthStatus> {
  return apiCall<GithubAuthStatus>("get_github_auth_status");
}

/** Select the account to use. `null` follows the active gh account. */
export async function setGithubAccount(login: string | null): Promise<GithubAuthStatus> {
  return apiCall<GithubAuthStatus>("set_github_account", { login });
}
