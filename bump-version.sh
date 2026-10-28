#!/usr/bin/env bash
# Usage: ./bump-version.sh 0.2.1
# Sets the app version in Cargo.toml and Cargo.lock (shown in Settings > About).
set -euo pipefail
cd "$(dirname "$0")"

version="${1:?usage: $0 <version>, e.g. $0 0.2.1}"
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || { echo "not a semver version: $version" >&2; exit 1; }

sed -i "s/^version = .*/version = \"$version\"/" Cargo.toml
awk -v v="$version" '/^name = "(desktop-rs|gd-git|gd-ui|gd-syntax)"$/ {fix=1} fix && /^version = / {$0="version = \"" v "\""; fix=0} 1' Cargo.lock > Cargo.lock.new
mv Cargo.lock.new Cargo.lock
cargo update
echo "Version is now $version"
