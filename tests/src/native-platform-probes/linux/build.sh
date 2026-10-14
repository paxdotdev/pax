#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
output=${PAX_PROBE_OUTPUT:-/tmp/pax-1038-linux-probes}
mkdir -p "$output"
cc -std=c11 -O1 -g -Wall -Wextra -Werror vulkan-capabilities.c -o "$output/vulkan-capabilities" $(pkg-config --cflags --libs vulkan)
cc -std=c11 -O1 -g -Wall -Wextra -Werror gtk-composition.c gpu-import.c -o "$output/gtk-composition" $(pkg-config --cflags --libs gtk4 epoxy) -lm -ldl
printf '%s\n' "$output"
