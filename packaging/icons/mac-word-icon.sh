#!/bin/bash
# Regenerate the stable Mac-Word icon assets from the original SVG.
set -euo pipefail
cd "$(dirname "$0")/../.."

out=assets/app-icon
source_svg="$out/mac-word.svg"
stage="$(mktemp -d)"
trap 'rm -r "$stage"' EXIT
iconset="$stage/Mac-Word.iconset"
mkdir -p "$iconset"

for size in 16 24 32 48 64 128 256 512 1024; do
    sips -s format png -z "$size" "$size" "$source_svg" --out "$stage/$size.png" >/dev/null
done

for size in 16 32 128 256 512; do
    cp "$stage/$size.png" "$iconset/icon_${size}x${size}.png"
    cp "$stage/$((size * 2)).png" "$iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$iconset" -o "$stage/mac-word.icns"
cargo xtask ico "$stage/mac-word.ico" "$stage/"{16,24,32,48,64,128,256}.png

cp "$stage/mac-word.icns" "$stage/mac-word.ico" "$out/"
cp "$stage/512.png" "$out/mac-word-macos-512.png"
cp "$stage/256.png" "$out/mac-word-256.png"
cp "$stage/256.png" "$out/hicolor/256x256/apps/com.manikpruthi.mac-word.png"
