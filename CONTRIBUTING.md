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

The app is single instance per bundle identifier: a second app with the same identifier hands the launch to the running one and exits. So with `YOUSKILL_DEV_PORT` set, the run also uses the identifier `com.gumpw.youskill.dev<port>`, and runs on different ports do not block each other. A run without the variable keeps the identifier from `tauri.conf.json`, so it still hands off to an open YouSkill.app.

Skill data and the account token stay in the home directory and the keychain, so all runs share them. WebView storage (for example the theme) is per identifier, so a run with `YOUSKILL_DEV_PORT` starts with its own UI settings.

## Before Submitting a PR

```bash
cargo check  # must pass with no errors
cargo test   # must pass
```

Do not edit `CHANGELOG.md`. Changelog entries are added by maintainers.

## Questions?

Open an issue or ask on [Discord](https://discord.gg/j6DR5GK2x).
