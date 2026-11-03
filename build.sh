#!/bin/bash
# Builds "Mac-Word.app" as a universal binary (arm64 + x86_64), ad-hoc signed, into
# $HOME/Library/Caches/MacWordBuild. Usage: ./build.sh [debug|release]  (default: debug)
set -euo pipefail
cd "$(dirname "$0")"

APP_NAME="Mac-Word"
APP_EXECUTABLE="Mac-Word"
BUNDLE_ID="com.manikpruthi.mac-word"
BUILD_DIR="${MACWORD_BUILD_DIR:-$HOME/Library/Caches/MacWordBuild}"
APP="$BUILD_DIR/$APP_NAME.app"
MODE="${1:-debug}"

if [ "$MODE" = "release" ] || [ "$MODE" = "--release" ]; then
    PROFILE_ARGS=(--release)
    PROFILE_DIR="release"
else
    PROFILE_ARGS=()
    PROFILE_DIR="debug"
fi

VERSION="$(awk '/^\[/ { p = ($0 == "[workspace.package]"); next } p && $1 == "version" { gsub(/[" ]/, "", $3); print $3; exit }' Cargo.toml)"
SHORT_VERSION="${VERSION%%-*}"
export MACOSX_DEPLOYMENT_TARGET=11.0
export MACWORD_BUILD_SHA="${MACWORD_BUILD_SHA:-$(git rev-parse HEAD 2>/dev/null || echo unknown)}"
export MACWORD_BUILD_DATE="${MACWORD_BUILD_DATE:-$(date -u +%Y-%m-%d)}"
if [ -z "${CRAFT_I18N_DIR:-}" ] && [ -d "../craft-i18n/language-data" ]; then
    export CRAFT_I18N_DIR="$(cd ../craft-i18n && pwd)"
fi

TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)
for t in "${TARGETS[@]}"; do
    rustup target list --installed | grep -qx "$t" || rustup target add "$t"
done

# One cargo invocation for both targets: cargo schedules them together.
echo "Compiling $APP_NAME $VERSION for ${TARGETS[*]} ($MODE)…"
TARGET_ARGS=()
for t in "${TARGETS[@]}"; do TARGET_ARGS+=(--target "$t"); done
cargo build ${PROFILE_ARGS[@]+"${PROFILE_ARGS[@]}"} -p wordcraft -p wordcraft-cli "${TARGET_ARGS[@]}"

TARGET_DIR="${CARGO_TARGET_DIR:-target}"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create "$TARGET_DIR/aarch64-apple-darwin/$PROFILE_DIR/wordcraft" "$TARGET_DIR/x86_64-apple-darwin/$PROFILE_DIR/wordcraft" \
    -output "$APP/Contents/MacOS/$APP_EXECUTABLE"
lipo -create "$TARGET_DIR/aarch64-apple-darwin/$PROFILE_DIR/wordcraft-cli" "$TARGET_DIR/x86_64-apple-darwin/$PROFILE_DIR/wordcraft-cli" \
    -output "$BUILD_DIR/mac-word-cli"
lipo -info "$APP/Contents/MacOS/$APP_EXECUTABLE"

cp assets/app-icon/mac-word.icns "$APP/Contents/Resources/$APP_EXECUTABLE.icns"
cp NOTICE ATTRIBUTION.md LICENSE-MIT LICENSE-APACHE "$APP/Contents/Resources/"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@SHORT_VERSION@/$SHORT_VERSION/g" -e "s/@BUILD_SHA@/$MACWORD_BUILD_SHA/g" \
    packaging/macos/Info.plist.in >"$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleExecutable $APP_EXECUTABLE" -c "Set :CFBundleIconFile $APP_EXECUTABLE" "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist" >/dev/null
printf 'APPL????' >"$APP/Contents/PkgInfo"

# A source file copied in can carry extended attributes, which codesign --strict rejects.
xattr -cr "$APP"
codesign --force --sign - --options runtime --entitlements packaging/macos/entitlements.plist \
    --requirements "=designated => identifier \"$BUNDLE_ID\"" "$APP"
codesign --force --sign - "$BUILD_DIR/mac-word-cli"
codesign --verify --strict "$APP"

touch "$BUILD_DIR/.metadata_never_index"
echo "Built $APP"
