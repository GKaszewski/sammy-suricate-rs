#!/usr/bin/env bash
# Fills vita/package (the VPK's extra files, `package.metadata.vita.assets`) before
# `cargo vita build vpk`: LiveArea art rendered from the game data by the desktop build
# (icon: Sammy's portrait; background and startup gate: the main menu) and the
# SoundFont. Nothing here is committed: it comes from the game and the SoundFont.
#
# Needs the game folder set up for the desktop build, ImageMagick, and a .sf2
# (SAMMY_SOUNDFONT, else the first one in ~/.local/share/sammy-suricate).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
out="$here/package"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cargo build --release -p sammy-sdl --manifest-path "$root/Cargo.toml"
game="$root/target/release/sammy-sdl"

# shot <name> <args> [VAR=value ...]: one frame at 3 s, saved as $work/<name>_3.png.
shot() {
    local name=$1 arg=$2
    shift 2
    env SAMMY_NOSAVE=1 SAMMY_SHOT_SECS=3 SAMMY_SCREENSHOT="$work/$name.png" "$@" "$game" "$arg" >/dev/null 2>&1
    [[ -f "$work/${name}_3.png" ]] || { echo "no $name shot" >&2; exit 1; }
}
shot menu menu SAMMY_NOINTRO=1 SAMMY_PAD_NAV=1
shot portrait 21 SAMMY_DRAW=scene,level,sky,entities SAMMY_PORTRAIT=60,78,20

# LiveArea images are 8-bit palette PNGs.
# png8 <out> <magick input and operations...>
png8() {
    local dst=$1
    shift
    magick "$@" -strip -colors 256 "PNG8:$dst"
}
rm -rf "$out"
mkdir -p "$out/sce_sys/livearea/contents"
# The menu without the version label in its corner.
png8 "$out/sce_sys/livearea/contents/bg.png" "$work/menu_3.png" -crop 896x533+32+11 +repage -resize '840x500!'
png8 "$out/sce_sys/livearea/contents/startup.png" "$work/menu_3.png" -crop 940x530+10+14 +repage -resize '280x158!'
png8 "$out/sce_sys/icon0.png" "$work/portrait_3.png" -crop 360x360+250+40 +repage -resize '128x128!'
cp "$here/livearea/template.xml" "$out/sce_sys/livearea/contents/"

sf2=${SAMMY_SOUNDFONT:-$(find "$HOME/.local/share/sammy-suricate" -maxdepth 1 -iname '*.sf2' 2>/dev/null | sort | head -n1)}
if [[ -n "$sf2" ]]; then
    cp "$sf2" "$out/"
else
    echo "no SoundFont: the package plays no music" >&2
fi
ls -lR "$out"
