#!/bin/bash
# Verify the release contract for one version:
#   package.json, package-lock.json, src-tauri/tauri.conf.json,
#   src-tauri/Cargo.toml and the workspace entries in src-tauri/Cargo.lock
#   all carry X.Y.Z, and CHANGELOG.md has a "## [X.Y.Z] - YYYY-MM-DD" section
#   with content.
# CI does not otherwise compare the tag to these files, and a mismatch breaks
# the updater (see docs/UPDATER.md). Run from the repository root.
# Usage: scripts/check-release-version.sh <version>

set -euo pipefail

version="${1:-}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "usage: check-release-version.sh X.Y.Z" >&2
    exit 2
fi

status=0

expect() {
    local file="$1" actual="$2"
    if [ "$actual" != "$version" ]; then
        echo "error: $file has version '${actual}', expected '$version'" >&2
        status=1
    fi
}

expect package.json "$(node -p "require('./package.json').version")"
expect "package-lock.json (version)" "$(node -p "require('./package-lock.json').version")"
expect "package-lock.json (packages[''].version)" "$(node -p "require('./package-lock.json').packages[''].version")"
expect src-tauri/tauri.conf.json "$(node -p "require('./src-tauri/tauri.conf.json').version")"
expect "src-tauri/Cargo.toml ([workspace.package])" \
    "$(awk -F'"' '/^\[/{s=($0=="[workspace.package]")} s && /^version = /{print $2; exit}' src-tauri/Cargo.toml)"

# Workspace crates: you-skill and youskill-*.
lock_entries=$(awk -F'"' '
    /^name = /{name=$2; next}
    /^version = / && (name == "you-skill" || name ~ /^youskill-/) {print name " " $2}
' src-tauri/Cargo.lock)
if [ -z "$lock_entries" ]; then
    echo "error: src-tauri/Cargo.lock has no workspace crate entries" >&2
    status=1
fi
while read -r name actual; do
    [ -n "$name" ] && expect "src-tauri/Cargo.lock ($name)" "$actual"
done <<<"$lock_entries"

if ! grep -Eq "^## \[${version//./\\.}\] - [0-9]{4}-[0-9]{2}-[0-9]{2}\$" CHANGELOG.md; then
    echo "error: CHANGELOG.md has no '## [$version] - YYYY-MM-DD' section" >&2
    status=1
elif ! "$(dirname "$0")/release-notes.sh" "$version" | grep -Eq '^- '; then
    echo "error: CHANGELOG.md section [$version] has no entries" >&2
    status=1
fi

if [ "$status" -eq 0 ]; then
    echo "release contract ok: $version"
fi
exit "$status"
