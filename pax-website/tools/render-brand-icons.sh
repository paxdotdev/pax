#!/usr/bin/env bash
set -euo pipefail

website_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
logo_source="$website_dir/../pax-std/src/drawing/pax_logo/pax-logo.svg"
output_dir="$website_dir/assets/branding"

command -v magick >/dev/null || {
    echo "Install ImageMagick to regenerate the website brand icons." >&2
    exit 1
}
mkdir -p "$output_dir"

# Preserve the canonical artwork and aspect ratio. An opaque ink ground keeps
# the paper-colored mark legible in both light and dark browser chrome.
magick -background none "$logo_source" \
    -resize 960x960 -fill '#FCF9F2' -colorize 100 \
    -background '#11110F' -gravity center -extent 1024x1024 \
    -alpha remove -alpha off -strip "PNG24:$output_dir/pax-logo.png"

magick "$output_dir/pax-logo.png" \
    -define icon:auto-resize=64,48,32,16 "$output_dir/pax-favicon.ico"
