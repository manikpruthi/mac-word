#!/bin/bash
# Runs ./build.sh release and puts "Mac-Word.app" on a versioned DMG in release/.
set -euo pipefail
cd "$(dirname "$0")"

VERSION="$(awk '/^\[/ { p = ($0 == "[workspace.package]"); next } p && $1 == "version" { gsub(/[" ]/, "", $3); print $3; exit }' Cargo.toml)"
APP_NAME="Mac-Word"
BUILD_DIR="${MACWORD_BUILD_DIR:-$HOME/Library/Caches/MacWordBuild}"
RELEASE_DIR="release"
APP="$BUILD_DIR/$APP_NAME.app"

./build.sh release

rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
ditto "$APP" "$STAGE/$APP_NAME.app"
ln -s /Applications "$STAGE/Applications"

diskutil image create from --format UDZO --volumeName "$APP_NAME" \
    "$STAGE" "$RELEASE_DIR/Mac-Word-$VERSION.dmg" >/dev/null

touch "$RELEASE_DIR/.metadata_never_index"

echo "Packaged:"
ls -lh "$RELEASE_DIR"
