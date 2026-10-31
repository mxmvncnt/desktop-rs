#!/usr/bin/env bash
# Usage: ./bump-version.sh 0.2.1
# Sets the app version in Cargo.toml and Cargo.lock (shown in Settings > About),
# commits it and tags the commit with the same version. Nothing is pushed.
# Prints release notes (commits since the previous version tag, grouped by their
# "type: subject" prefix) at the end.
# Refuses to run unless the tree is clean and in sync with its upstream.
set -euo pipefail
cd "$(dirname "$0")"

version="${1:?usage: $0 <version>, e.g. $0 0.2.1}"
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || { echo "not a semver version: $version" >&2; exit 1; }

[[ -z $(git status --porcelain) ]] || { echo "uncommitted changes, commit or stash them first" >&2; exit 1; }
upstream=$(git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null) || { echo "current branch has no upstream" >&2; exit 1; }
git fetch --quiet --tags "${upstream%%/*}"
[[ $(git rev-parse HEAD) == "$(git rev-parse "$upstream")" ]] || { echo "HEAD differs from $upstream, push or pull first" >&2; exit 1; }
[[ -z $(git tag -l "$version") ]] || { echo "tag $version already exists" >&2; exit 1; }

previous=$(git tag -l '[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname | head -n1)
subjects=$(git log --no-merges --format=%s ${previous:+"$previous..HEAD"} | grep -v '^chore: bump version' || true)

sed -i "s/^version = .*/version = \"$version\"/" Cargo.toml
awk -v v="$version" '/^name = "(desktop-rs|gd-git|gd-ui|gd-syntax)"$/ {fix=1} fix && /^version = / {$0="version = \"" v "\""; fix=0} 1' Cargo.lock > Cargo.lock.new
mv Cargo.lock.new Cargo.lock
cargo update --workspace
git commit --quiet -m "chore: bump version to $version" Cargo.toml Cargo.lock
git tag "$version"
echo "Version is now $version (committed and tagged, not pushed)"

echo
echo "# Release notes for $version"
for type in feat fix perf refactor docs style test build ci chore; do
  lines=$(sed -nE "s/^$type(\([^)]*\))?!?: //p" <<< "$subjects")
  [[ -z $lines ]] && continue
  heading=${type^}; [[ $type == feat ]] && heading=Feature
  printf '\n## %s\n' "$heading"
  sed 's/^/- /' <<< "$lines"
done
other=$(grep -vE '^(feat|fix|perf|refactor|docs|style|test|build|ci|chore)(\([^)]*\))?!?: ' <<< "$subjects" || true)
[[ -z $other ]] || { printf '\n## Other\n'; sed 's/^/- /' <<< "$other"; }
