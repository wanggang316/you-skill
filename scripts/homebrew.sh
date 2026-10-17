#!/bin/bash
# Write the Homebrew formula (youskill CLI) and cask (YouSkill app) for a released
# version. The release assets are fetched from GitHub to compute their checksums. The
# output directory has the tap layout, so the result can be copied into the root of
# wanggang316/homebrew-tap as is:
#
#   <output-dir>/Formula/youskill.rb
#   <output-dir>/Casks/youskill.rb
#
# Usage: ./scripts/homebrew.sh <version> <output-dir>   (e.g. 0.10.0 ./tap)

set -euo pipefail

if [ $# -ne 2 ]; then
  echo "Usage: $0 <version> <output-dir>" >&2
  exit 1
fi

VERSION="$1"
OUT="$2"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "error: version must be x.y.z (got: $VERSION)" >&2
  exit 1
fi

REPO="wanggang316/you-skill"
BASE="https://github.com/${REPO}/releases/download/v${VERSION}"
CLI_ARM="youskill-v${VERSION}-aarch64-apple-darwin.tar.gz"
CLI_INTEL="youskill-v${VERSION}-x86_64-apple-darwin.tar.gz"
DMG_ARM="YouSkill_${VERSION}_aarch64.dmg"
DMG_INTEL="YouSkill_${VERSION}_x64.dmg"

checksum() {
  local sha
  sha="$(curl -fsSL "${BASE}/$1" | shasum -a 256 | cut -d' ' -f1)"
  # An empty download hashes to this value; treat it as a failure.
  if [ "$sha" = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" ]; then
    echo "error: empty download for $1" >&2
    exit 1
  fi
  echo "$sha"
}

CLI_ARM_SHA="$(checksum "$CLI_ARM")"
CLI_INTEL_SHA="$(checksum "$CLI_INTEL")"
DMG_ARM_SHA="$(checksum "$DMG_ARM")"
DMG_INTEL_SHA="$(checksum "$DMG_INTEL")"

mkdir -p "${OUT}/Formula" "${OUT}/Casks"

cat >"${OUT}/Formula/youskill.rb" <<EOF
class Youskill < Formula
  desc "Command-line interface for the YouSkill agent skills manager"
  homepage "https://github.com/${REPO}"
  license "MIT"

  livecheck do
    url :stable
    strategy :github_latest
  end

  on_macos do
    on_arm do
      url "${BASE}/${CLI_ARM}"
      sha256 "${CLI_ARM_SHA}"
    end
    on_intel do
      url "${BASE}/${CLI_INTEL}"
      sha256 "${CLI_INTEL_SHA}"
    end
  end

  def install
    bin.install "youskill"
    generate_completions_from_executable(bin/"youskill", "completions")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/youskill --version")
  end
end
EOF

# The app updates itself (Tauri updater), so the cask sets auto_updates. The CLI is not
# linked by the cask: it is the youskill formula, or Settings > "Command line tool" in
# the app. zap leaves ~/.youskill alone, because it holds the user's skill library.
cat >"${OUT}/Casks/youskill.rb" <<EOF
cask "youskill" do
  arch arm: "aarch64", intel: "x64"

  version "${VERSION}"
  sha256 arm:   "${DMG_ARM_SHA}",
         intel: "${DMG_INTEL_SHA}"

  url "https://github.com/${REPO}/releases/download/v#{version}/YouSkill_#{version}_#{arch}.dmg"
  name "YouSkill"
  desc "Desktop manager for agent skills"
  homepage "https://github.com/${REPO}"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates true
  depends_on :macos

  app "YouSkill.app"

  zap trash: [
    "~/Library/Application Support/com.gumpw.youskill",
    "~/Library/Caches/com.gumpw.youskill",
    "~/Library/Preferences/com.gumpw.youskill.plist",
    "~/Library/Saved Application State/com.gumpw.youskill.savedState",
    "~/Library/WebKit/com.gumpw.youskill",
  ]
end
EOF

echo "Wrote ${OUT}/Formula/youskill.rb and ${OUT}/Casks/youskill.rb"
