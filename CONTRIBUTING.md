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

## Development Setup

`npm run tauri dev` runs the app under the identifier `com.gumpw.youskill.dev`. The release app uses `com.gumpw.youskill`. The single-instance lock follows the identifier, so the dev build runs while the installed YouSkill.app is open. A second launch of the dev build focuses the running dev instance. Bundled builds (`npm run tauri build`, also with `--debug`) keep `com.gumpw.youskill`.

The dev build uses the same app data as the release app:

- The hub in `~/.youskill` (skills, instructions, lock file) is shared. The `youskill` CLI uses the same hub.
- The settings file `youskill/youskill.config` in the system config directory (`~/Library/Application Support` on macOS) is shared.
- The signed-in account in the keychain (service `com.gumpw.youskill.account`) is shared.

These locations are fixed. They do not depend on the identifier. We keep one data set on purpose: a separate dev data set needs a second hub root in the app and the CLI. The cross-process lock on `~/.youskill/.ops.lock` keeps operations from the dev app, the release app, and the CLI safe.

Thus the dev build shows your real data. Changes you make in the dev build change the data of the installed app. Back up `~/.youskill` before you test destructive flows.

## Before Submitting a PR

```bash
cargo check  # must pass with no errors
cargo test   # must pass
```

Do not edit `CHANGELOG.md`. Changelog entries are added by maintainers.

## Questions?

Open an issue or ask on [Discord](https://discord.gg/j6DR5GK2x).
