#!/bin/bash
# Print the Homebrew formula for a released youskill version. The archives are fetched
# from the GitHub release to compute their checksums. Commit the output as
# Formula/youskill.rb in the tap repository.
#
# Usage: ./scripts/homebrew-formula.sh <version>   (e.g. 0.10.0)

set -euo pipefail

if [ $# -ne 1 ]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

VERSION="$1"
REPO="wanggang316/you-skill"
BASE="https://github.com/${REPO}/releases/download/v${VERSION}"
ARM="youskill-v${VERSION}-aarch64-apple-darwin.tar.gz"
INTEL="youskill-v${VERSION}-x86_64-apple-darwin.tar.gz"

checksum() {
  curl -fsSL "$1" | shasum -a 256 | cut -d' ' -f1
}

ARM_SHA="$(checksum "${BASE}/${ARM}")"
INTEL_SHA="$(checksum "${BASE}/${INTEL}")"

cat <<EOF
class Youskill < Formula
  desc "Command line interface for the YouSkill agent skills manager"
  homepage "https://github.com/${REPO}"
  version "${VERSION}"
  license "MIT"

  livecheck do
    url :stable
    strategy :github_latest
  end

  on_macos do
    on_arm do
      url "${BASE}/${ARM}"
      sha256 "${ARM_SHA}"
    end
    on_intel do
      url "${BASE}/${INTEL}"
      sha256 "${INTEL_SHA}"
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
