---
name: release
description: Cut a YouSkill release through a pull request. Write the release notes into CHANGELOG, pick the Semantic Versioning bump from them, bump the version in every file that carries it, and open a release PR; merging it makes CI tag vX.Y.Z, build, sign, notarize, publish the GitHub Release with the updater feed, and update the Homebrew tap. Use when shipping a build to GitHub Releases and the in-app updater.
---

# release: Cut a YouSkill release

A release is **a release PR**. The PR (head branch `release/vX.Y.Z`) lands the
version bump and the release notes on `main`. Its merge starts
`.github/workflows/release-pr.yml`, which verifies the merged commit, pushes the
annotated tag `vX.Y.Z` on it, and calls `release.yml`: build on macOS arm64,
macOS x64 and Windows, sign and notarize, upload the updater artifacts
(`latest.json` and `.sig` files) and the CLI archives, publish the release, and
update the Homebrew tap. **Merging the release PR ships the release** to every
installed client on its next update check.

The contract CI enforces (`scripts/check-release-version.sh`, on every push to
the PR, on the merged commit, and again in `release.yml`):

> tag `vX.Y.Z` ⇔ version `X.Y.Z` in `package.json`, `package-lock.json` (two
> fields), `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`
> (`[workspace.package]`) and the workspace crates in `src-tauri/Cargo.lock`
> ⇔ `CHANGELOG.md` has `## [X.Y.Z] - YYYY-MM-DD` with entries.

A missed file breaks the updater: an old `tauri.conf.json` makes `latest.json`
carry the old version (nobody upgrades); an old `Cargo.toml` makes the app
under-report its version (the update is offered again after each install).

CI publishes the CHANGELOG section verbatim as the GitHub Release body and the
in-app update notes. It is published prose, not a work record.

Don't use this for fixing the release pipeline itself (no version bump).
Never push to `main` directly and never push a `v*` tag by hand; every release
goes through a PR.

## Process

The release PR is the single gate. Run steps 1–6 without stopping: they write
only to a release branch and an open PR, which is the review surface. After the
user approves the merge in step 7, run steps 7–8 without stopping, up to the
published release.

### 1. Pre-flight

```bash
git status --porcelain                                  # empty
git fetch origin --tags
git describe --tags --abbrev=0 --match 'v[0-9]*' origin/main   # last release
node -p "require('./package.json').version"             # current version
gh pr list --search 'head:release/' --state open        # none
```

Stop if the tree is dirty, a release PR is already open, or the target tag
already exists.

### 2. Write the release notes

`[Unreleased]` is **input, not output**: it collects one bullet per PR, so a
single feature arrives with several bullets from different stages. Never
promote it as-is. Always cross-check it against

```bash
git log --no-merges --pretty='%h %s' \
  "$(git describe --tags --abbrev=0 --match 'v[0-9]*' origin/main)..origin/main"
```

and keep only what a user can perceive.

**Consolidate.**
- One entry per user-visible surface (Skill Library, Instructions, Projects,
  Settings, a CLI verb) per category, however many bullets it had.
- Describe the end state. Superseded work is dropped, not merged.
- Fold small polish on one area into a single bullet.
- A release reads as **3–8 entries**.

**Keep each entry short**: one or two sentences, three lines at most.

**Write for users.**
- Lead with what the user gets or the symptom that's gone, not the mechanism.
- Leave out engineering-only work (refactors, CI, lint, dependency bumps).
  Exceptions that are always listed: user-perceivable side effects
  (minimum-OS bump, faster startup) and anything deprecated, removed, or
  breaking.
- No commit prefixes, hashes, type or module names. Name the UI surface the
  user sees. Issue links in the existing format
  (`([#95](https://github.com/wanggang316/youskill/issues/95))`) may stay.

Categories, in this order: `Breaking Changes`, `Added`, `Changed`,
`Deprecated`, `Removed`, `Fixed`, `Security` (`AGENTS.md` lists the five the
project seeds; add `Deprecated` / `Security` only when they apply).

If nothing user-visible is left, stop: there is nothing to release.

### 3. Choose the version

Versions follow Semantic Versioning 2.0.0. Derive the bump from the notes of
step 2 — they are the user-visible change set — not from commit prefixes:

| The release contains | Bump |
|---|---|
| `Breaking Changes`, anything under `Removed`, or a `Changed` entry that breaks a CLI verb or flag, the hub layout, or a lock / settings file | **major** — but **minor** while the version is `0.y.z` |
| Anything under `Added`, `Changed`, or `Deprecated` | **minor** |
| Only `Fixed` and `Security` | **patch** |

The highest row wins. Never bump to `1.0.0` on your own; going to 1.0 is the
user's call.

This step is not a gate: state the version and the row that chose it, and go
on. The user corrects it in the PR if needed.

### 4. Release commit on a detached HEAD, bump

```bash
git switch --detach origin/main
```

Work on a detached HEAD and push to `release/vX.Y.Z` by refspec. A local
branch named `release` (for example a worktree branch) blocks any local
`release/...` branch, and a branch started from `origin/main` tracks `main`,
so a bare `git push` could push to `main`.

In `CHANGELOG.md`, put the notes of step 2 under `## [X.Y.Z] - YYYY-MM-DD`
(today, local TZ), drop its empty categories, and reseed an empty
`## [Unreleased]` above it with the five headers (`Breaking Changes`,
`Added`, `Changed`, `Fixed`, `Removed`). Never touch a released section.

Bump the version:

```bash
V=X.Y.Z
npm version "$V" --no-git-tag-version --allow-same-version   # package.json + package-lock.json
```

Edit `src-tauri/tauri.conf.json` (top-level `"version"`) and
`src-tauri/Cargo.toml` (`[workspace.package] version`), then refresh only the
workspace entries of the lockfile:

```bash
cargo update --manifest-path src-tauri/Cargo.toml --workspace --offline
git diff --stat src-tauri/Cargo.lock    # only you-skill, youskill-cli, youskill-core
scripts/check-release-version.sh "$V"   # must print "release contract ok"
```

Never use `cargo generate-lockfile`: it re-resolves the whole dependency graph.

### 5. Local checks, commit, push

There is no PR build check, and a broken build leaves the release in draft
after the tag exists. Run the checks before pushing:

```bash
npm run check
npm run test
```

Both compile only the host platform. When `main` gained platform-gated code
(`#[cfg(target_os = "macos")]`) since the last release, compile the Windows
variant: flip that `cfg` to `"windows"` in a scratch copy of the file, run
`cargo check --manifest-path src-tauri/Cargo.toml`, restore. Never gate a
trait `use` (like `tauri::Manager`) by eyeballing a grep.

Stage only the release files, in one commit, with no trailers:

```bash
git add CHANGELOG.md package.json package-lock.json src-tauri/tauri.conf.json \
  src-tauri/Cargo.toml src-tauri/Cargo.lock
git status --short          # nothing else staged
git commit -m "chore(release): bump to $V"
git push origin "HEAD:refs/heads/release/v$V"
```

### 6. Open the release PR

```bash
gh pr create --base main --head "release/v$V" \
  --title "chore(release): bump to $V" --body-file - <<'EOF'
Release X.Y.Z: <bump> because <row of step 3>.

<the new CHANGELOG section, verbatim>

Merging this PR ships X.Y.Z: release-pr.yml tags the merged commit, builds,
signs and notarizes it, publishes the release to the updater, and updates the
Homebrew tap.
EOF
```

Not a draft. No agent names in the title or body. Then wait for the `verify`
check: `gh pr checks <PR> --watch`.

### 7. Merge — the only gate

Report the PR URL, the version, and the check result, and wait for the user to
approve the merge (or to merge it themselves). Edits the user asks for go onto
the release branch as new commits (`git push origin "HEAD:refs/heads/release/v$V"`).
If `main` gained user-visible changes since step 2, rebuild the release commit
on the new `origin/main` (update the notes and, if the bump row changed, the
version and the branch name) before the merge.

```bash
gh pr merge <PR> --rebase
git push origin --delete "release/v$V"
```

Rebase keeps the bump commit as is on `main`. Don't pass `--delete-branch`: it
fails when `main` is checked out in another worktree. Never tag by hand:
`release-pr.yml` tags the merged commit.

### 8. Follow the release run

```bash
RUN=$(gh run list --workflow=release-pr.yml --branch "release/v$V" --event pull_request \
  --limit 1 --json databaseId --jq '.[0].databaseId')
gh run watch "$RUN" --exit-status
gh release view "v$V" --json url,isDraft,assets --jq '{url, isDraft, assets: [.assets[].name]}'
```

The build takes about 30–60 minutes. `release.yml` publishes the release only
when every build job passed; otherwise it stays a draft. Report the release
URL, whether `latest.json` is attached (without it the updater has nothing to
read), and the Homebrew job result.

Then return the worktree to its own branch, or leave it detached on the new
`origin/main`:

```bash
git fetch origin
git switch --detach origin/main
```

## Recovery

Roll forward; never force-push `main` or rewrite its history. Fixes to `main`
go through a PR. Deleting a pushed tag or release needs explicit user approval.

| Symptom | Fix |
|---|---|
| `verify` fails on the PR | Fix the release commit on the branch and push again. Nothing was tagged. |
| `verify` fails on the merged commit (version, CHANGELOG, tag exists elsewhere) | Nothing was tagged. Fix it in a new `release/vX.Y.Z` PR. |
| A build job fails (notarization, transient) | `gh run rerun <RUN> --failed`. Assets of passing jobs are already attached; the tag job accepts the existing tag. |
| A build job fails on code (for example Windows `E0599: no variant named X`, an unguarded macOS-only `tauri::` API) | With approval, delete the draft release (`gh release delete vX.Y.Z --yes`) and the tag (`git push origin :refs/tags/vX.Y.Z`), land the fix through a PR, then open a new release PR for the same version. A fix for a bug no published version had gets no changelog entry. |
| Homebrew job fails | `gh run rerun <RUN> --failed`, or `gh workflow run homebrew.yml -f tag=vX.Y.Z`. |
| Release PR closed without merge | Delete the branch; nothing was released. |
| CI cannot run the PR workflow and a release must ship | Fallback: push the annotated tag on the merged commit by hand; the tag push trigger of `release.yml` builds and publishes it. |
| Wrong version shipped | Supersede it with the next version. Do not retag. |
| Published release is broken | Mark its header `## [X.Y.Z] - YYYY-MM-DD [YANKED]` with a one-line reason; ship the fix as a new version. Do not delete the release: that rolls every client back to the previous `latest.json`. |
