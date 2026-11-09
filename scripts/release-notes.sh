#!/bin/bash
# Print the CHANGELOG.md body of one version, without its "## [X.Y.Z]" header.
# Usage: scripts/release-notes.sh <version> [changelog-path]

set -euo pipefail

version="${1:-}"
changelog="${2:-CHANGELOG.md}"

if [ -z "$version" ]; then
    echo "usage: release-notes.sh <version> [changelog-path]" >&2
    exit 2
fi

awk -v v="$version" '
    index($0, "## [" v "]") == 1 { found = 1; in_section = 1; next }
    in_section && /^## \[/ { exit }
    in_section { print }
    END { if (!found) exit 1 }
' "$changelog"
