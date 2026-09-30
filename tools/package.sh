#!/usr/bin/env bash
# Package a test build: the executable, the assets the game loads at run
# time (not the Blender sources, review renders or the whistle's source
# recording) and the player notes, zipped.
#
#   tools/package.sh EXECUTABLE OS VERSION     e.g. tools/package.sh target/release/el_silbon linux 0.1.0-test.1
set -euo pipefail
exe=$1 os=$2 version=$3
root=$(cd "$(dirname "$0")/.." && pwd)
name="el_silbon-${version}-${os}"
out="$root/target/dist/$name"
rm -rf "$out" "$out.zip"
mkdir -p "$out/assets/models" "$out/assets/branding"
cp "$exe" "$out/"
cp -r "$root/assets/audio" "$root/assets/fonts" "$root/assets/shaders" "$root/assets/ui" "$out/assets/"
rm -rf "$out/assets/audio/source"
cp "$root"/assets/models/*.glb "$out/assets/models/"
# The launch splash's cover; the icons are built into the executable.
cp "$root/assets/branding/whistle-cover.png" "$out/assets/branding/"
cp "$root/docs/PLAYING.txt" "$out/"
# Git LFS pointers instead of models mean the checkout lacks the real files.
if head -c 40 "$out/assets/models/silbon.glb" | grep -q "git-lfs"; then
    echo "assets/models are Git LFS pointers: run git lfs pull first" >&2
    exit 1
fi
(cd "$root/target/dist" && zip -qr "$name.zip" "$name")
echo "$root/target/dist/$name.zip"
