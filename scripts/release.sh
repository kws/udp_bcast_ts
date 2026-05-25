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

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "$(git rev-parse --abbrev-ref HEAD)" != "main" ]]; then
  echo "Releases must be cut from main" >&2
  exit 1
fi

git fetch --tags origin

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Working tree must be clean before cutting a release" >&2
  exit 1
fi

if [[ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]]; then
  echo "main must match origin/main before cutting a release" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required to read the Cargo package version" >&2
  exit 1
fi

cargo_version="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "udp_bcast_ts") | .version')"
if [[ "$version" != "v${cargo_version}" ]]; then
  echo "Tag ${version} does not match Cargo.toml version ${cargo_version}" >&2
  exit 1
fi

if git rev-parse -q --verify "refs/tags/${version}" >/dev/null; then
  echo "Local tag ${version} already exists" >&2
  exit 1
fi

if git ls-remote --exit-code --tags origin "refs/tags/${version}" >/dev/null 2>&1; then
  echo "Remote tag ${version} already exists" >&2
  exit 1
fi

scripts/check.sh

git tag -a "$version" -m "Release ${version}"
git push origin "$version"

echo "Pushed ${version}. GitHub Actions will build and publish the release assets."
