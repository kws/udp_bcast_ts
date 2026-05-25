#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 <vX.Y.Z>" >&2
}

if [[ $# -ne 1 ]]; then
  usage
  exit 2
fi

version="$1"
if [[ ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Version must be a semver tag like v0.1.0" >&2
  exit 2
fi

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  echo "This build script must run on Linux x86_64" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required to read the Cargo package version" >&2
  exit 1
fi

cargo_version="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "udp_bcast_ts") | .version')"
if [[ "$version" != "v${cargo_version}" ]]; then
  echo "Tag ${version} does not match Cargo.toml version ${cargo_version}" >&2
  exit 1
fi

artifact="udp_bcast_ts-${version}-x86_64-unknown-linux-gnu"
mkdir -p dist
rm -f "dist/${artifact}" "dist/${artifact}.sha256"

cargo build --release --locked
install -m 0755 target/release/udp_bcast_ts "dist/${artifact}"

(
  cd dist
  sha256sum "${artifact}" > "${artifact}.sha256"
)

echo "Built dist/${artifact}"
echo "Wrote dist/${artifact}.sha256"
