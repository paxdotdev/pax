#!/usr/bin/env bash
set -euo pipefail
output=${PAX_PROBE_OUTPUT:-/tmp/pax-1038-linux-probes}
mkdir -p "$output"
probe_runtime=$(mktemp -d /tmp/pax-1038-wayland.XXXXXX)
chmod 700 "$probe_runtime"
export XDG_RUNTIME_DIR="$probe_runtime"
export WAYLAND_DISPLAY=pax-probe-wayland
unset PAX_PROBE_EXTERNAL_INPUT
weston --backend=headless --renderer=pixman --width=1024 --height=800 \
    --socket="$WAYLAND_DISPLAY" --idle-time=0 --no-config --log="$output/weston.log" &
compositor_pid=$!
cleanup() {
    kill "$compositor_pid" 2>/dev/null || true
    wait "$compositor_pid" 2>/dev/null || true
    rm -rf "$probe_runtime"
}
trap cleanup EXIT
for step in {1..100}; do
    if [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]]; then break; fi
    if ! kill -0 "$compositor_pid" 2>/dev/null; then cat "$output/weston.log"; exit 1; fi
    sleep 0.05
done
if [[ ! -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]]; then
    cat "$output/weston.log"
    printf '%s\n' 'Weston did not create the Wayland socket' >&2
    exit 1
fi
PAX_PROBE_SELF_TEST=1 PAX_PROBE_SCREENSHOT="$output/gtk-wayland.png" \
    GDK_BACKEND=wayland GSK_RENDERER=${GSK_RENDERER:-gl} \
    timeout 30s "$output/gtk-composition" >"$output/gtk-wayland.log" 2>&1 || {
        status=$?; cat "$output/gtk-wayland.log"; exit "$status";
    }
cat "$output/gtk-wayland.log"
