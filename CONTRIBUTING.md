# Contributing to YouSkill

Thanks for wanting to contribute! This guide exists to save both of us time.

## The One Rule

**You must understand your code.** If you can't explain what your changes do and how they interact with the rest of the system, your PR will be closed.

Using AI to write code is fine. You can gain understanding by interrogating an agent with access to the codebase until you grasp all edge cases and effects of your changes. What's not fine is submitting agent-generated slop without that understanding.

If you use an agent, run it from the `you-skill` root directory so it picks up `AGENTS.md` automatically. Your agent must follow the rules and guidelines in that file.

## First-Time Contributors

We use an approval gate for new contributors:

1. Open an issue describing what you want to change and why
2. Keep it concise (if it doesn't fit on one screen, it's too long)
3. Write in your own voice, at least for the intro
4. A maintainer will comment `lgtm` if approved
5. Once approved, you can submit PRs

This exists because AI makes it trivial to generate plausible-looking but low-quality contributions. The issue step lets us filter early.

## Running the App in Development

```bash
npm install
npm run tauri dev
```

The Vite dev server uses port 5173 by default, and fails if the port is in use. To run a second copy at the same time, for example from another git worktree, set `YOUSKILL_DEV_PORT` for that run:

```bash
YOUSKILL_DEV_PORT=5180 npm run tauri dev
```

Vite serves on that port and the Tauri dev window loads from it. No config file changes.

### App identifier and single instance

The app is single instance per bundle identifier: a second app with the same identifier hands the launch to the running one and exits.

- `npm run tauri dev` runs under `com.gumpw.youskill.dev`, so it runs while the installed YouSkill.app (`com.gumpw.youskill`) is open.
- With `YOUSKILL_DEV_PORT` set, the run uses `com.gumpw.youskill.dev<port>`, so runs on different ports do not block each other or the installed app.
- A second launch of a dev run with the same identifier focuses the running one.
- Bundled builds (`npm run tauri build`, also with `--debug`) keep `com.gumpw.youskill`.

### Dev data

Dev runs share the hub with the release app, but keep their own app data:

- Shared by all runs: the hub in `~/.youskill` (skills, instructions, lock files). The `youskill` CLI uses the same hub. The cross-process lock on `~/.youskill/.ops.lock` keeps hub operations from dev runs, the release app, and the CLI safe.
- Dev only: the app data directory `youskill-dev` in the system config directory (`~/Library/Application Support` on macOS). It holds settings, favorites, projects, workspaces, scan folders, custom agent apps, and caches. The release app and the CLI use `youskill`.
- Dev only: the signed-in account, in the keychain service `com.gumpw.youskill.dev.account`. The release app uses `com.gumpw.youskill.account`.
- Per port: WebView local storage (for example the theme) is per origin, and the origin includes the port.

Why: the app data stores are locked only inside one process, and the account rotates its refresh token on each refresh. If the release app and a dev run shared them, one app could overwrite the changes of the other or make the other app's session invalid.

All dev runs, on any port, share `youskill-dev` and the dev session. Do not change the same settings, favorites, projects, or account from two dev runs at the same time. Only dev data is at risk; the release app data is not.

A new dev setup starts with default settings, no projects, no favorites, and no account. Hub changes you make in a dev run change the hub of the installed app. Back up `~/.youskill` before you test destructive flows.

On macOS, a dev run cannot complete sign-in. macOS gives the `youskill://auth/callback` link only to a bundled app that registers the scheme, which is the installed YouSkill.app, not the `tauri dev` binary. The installed app has no pending sign-in in its own keychain service, so it ignores the link. The account server accepts only this redirect URI. To test sign-in on macOS, build a bundle (`npm run tauri build`) and run it in place of the installed app. On Windows and Linux, the dev build registers the scheme at run time and receives the link.

## Before Submitting a PR

```bash
cargo check  # must pass with no errors
cargo test   # must pass
```

Do not edit `CHANGELOG.md`. Changelog entries are added by maintainers.

## Questions?

Open an issue or ask on [Discord](https://discord.gg/j6DR5GK2x).
