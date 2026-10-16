#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
output=${PAX_PROBE_OUTPUT:-/tmp/pax-1038-linux-probes}
mkdir -p "$output"
# Every invocation has fresh evidence, including compilation failures or crashes.
run_output=$(mktemp -d "$output/preflight.XXXXXX")
printf 'Preflight output: %s\n' "$run_output" >&2
# Use the caller's actual desktop/display and ICD configuration. Never create a
# software display or select a software Vulkan driver to satisfy a hardware gate.
cc -std=c11 -O1 -g -Wall -Wextra -Werror vulkan-capabilities.c \
    -o "$run_output/vulkan-capabilities" $(pkg-config --cflags --libs vulkan gtk4)
status=0
timeout 30s "$run_output/vulkan-capabilities" >"$run_output/hardware-preflight.json" \
    2>"$run_output/hardware-preflight.log" || status=$?
if [[ "$status" != 0 && "$status" != 1 && "$status" != 2 ]]; then
    cat "$run_output/hardware-preflight.log" >&2
    printf 'Preflight process failed or timed out (status %s)\n' "$status" >&2
    exit 1
fi
python3 -I check-preflight-report.py "$run_output/hardware-preflight.json" "$status" || {
    cat "$run_output/hardware-preflight.log" >&2
    exit 1
}
cat "$run_output/hardware-preflight.json"
exit "$status"
