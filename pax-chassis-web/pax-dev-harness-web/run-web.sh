#!/bin/sh
SHOULD_ALSO_RUN=$1
OUTPUT_PATH=$2

# Set the input and output paths
assets_dir="../../../../assets"
fonts_dir="../../../../assets/fonts"
new_dir="./assets"
# Create the new assets directory if it doesn't exist
mkdir -p "$new_dir"

# Move the assets folder to the new location
cp -r "$assets_dir"/* "$new_dir"

output_dir="assets/fonts"
output_file="fonts.css"

# Clear the output file if it exists
> "$output_file"

font_face_template="@font-face {
    font-family: '%s';
    src: url('%s') format('%s');
}"

# Iterate over the font files and generate the @font-face rules
find "$fonts_dir" -type f \( -iname "*.woff" -o -iname "*.woff2" -o -iname "*.eot" -o -iname "*.ttf" -o -iname "*.otf" \) |
while read -r font_file; do
    if [ -f "$font_file" ]; then
        font_type="${font_file##*.}"
        font_name="$(basename "$font_file" ".$font_type")"

        case "$font_type" in
            woff) format="woff" ;;
            woff2) format="woff2" ;;
            eot) format="embedded-opentype" ;;
            ttf) format="truetype" ;;
            otf) format="opentype" ;;
            *) font_family="$font_name"; format="unknown";;
        esac

        if [ "$format" != "unknown" ]; then
            font_family="$font_name"
            font_url="${output_dir}/${font_name}.${font_type}"
            printf "$font_face_template\n\n" "$font_family" "$font_url" "$format" >> "$output_file"
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
