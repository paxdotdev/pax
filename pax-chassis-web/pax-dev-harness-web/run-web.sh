#!/bin/sh
SHOULD_ALSO_RUN=$1
OUTPUT_PATH=$2

# Assets
# This section copies an assets directory to a chassis, searches for font files in the directory,
# and generates a CSS file with @font-face rules for each font file found.
# Supported font formats include WOFF, WOFF2, EOT, TTF, and OTF.

assets_dir="../../../../assets"
fonts_dir="../../../../assets/fonts"
new_dir="./assets"
mkdir -p "$new_dir"
cp -r "$assets_dir"/* "$new_dir"
output_file="fonts.css"
> "$output_file"

find "$fonts_dir" -type f \( -iname "*.woff" -o -iname "*.woff2" -o -iname "*.eot" -o -iname "*.ttf" -o -iname "*.otf" \) |
while read -r font_file; do
    if [ -f "$font_file" ]; then
        font_type="${font_file##*.}"
        font_name="$(basename "$font_file" ".$font_type")"
        output_dir="assets/fonts"
        font_url="${output_dir}/${font_name}.${font_type}"

        case "$font_type" in
            woff) format="woff" ;;
            woff2) format="woff2" ;;
            eot) format="embedded-opentype" ;;
            ttf) format="truetype" ;;
            otf) format="opentype" ;;
            *) format="unknown" ;;
        esac

        if [ "$format" != "unknown" ]; then
            font_face_template="@font-face {
    font-family: '%s';
    src: url('%s') format('%s');
}"
            printf "$font_face_template\n\n" "$font_name" "$font_url" "$format" >> "$output_file"
        fi
    fi
done

# Clear old build and move to output directory
rm -rf "$OUTPUT_PATH"
mkdir -p "$OUTPUT_PATH"
cp -r . "$OUTPUT_PATH"
cd "$OUTPUT_PATH"

# Remove this script in output directory
rm -- "$0"

if [ "$SHOULD_ALSO_RUN" = "true" ]; then
  # Run
  set -ex
  yarn serve || (yarn && yarn serve)
fi
