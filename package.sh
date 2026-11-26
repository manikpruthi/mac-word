#!/bin/bash
# Runs ./build.sh release and puts "Mac-Word.app" on a versioned DMG in release/.
set -euo pipefail
cd "$(dirname "$0")"

VERSION="$(awk '/^\[/ { p = ($0 == "[workspace.package]"); next } p && $1 == "version" { gsub(/[" ]/, "", $3); print $3; exit }' Cargo.toml)"
APP_NAME="Mac-Word"
BUILD_DIR="${MACWORD_BUILD_DIR:-$HOME/Library/Caches/MacWordBuild}"
RELEASE_DIR="release"
APP="$BUILD_DIR/$APP_NAME.app"
ARCH_SUFFIX=""
if [ "${MACWORD_ARM64_ONLY:-0}" = 1 ]; then
    ARCH_SUFFIX="-arm64"
fi
DMG="$RELEASE_DIR/Mac-Word-$VERSION$ARCH_SUFFIX.dmg"

./build.sh release

mkdir -p "$RELEASE_DIR"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
mkdir "$STAGE/payload"
ditto "$APP" "$STAGE/payload/$APP_NAME.app"
ln -s /Applications "$STAGE/payload/Applications"

diskutil image create from --format UDZO --volumeName "$APP_NAME" \
    "$STAGE/payload" "$STAGE/Mac-Word.dmg" >/dev/null
mv -f "$STAGE/Mac-Word.dmg" "$DMG"

touch "$RELEASE_DIR/.metadata_never_index"

echo "Packaged:"
ls -lh "$DMG"
