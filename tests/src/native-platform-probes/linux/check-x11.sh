#!/usr/bin/env bash
set -euo pipefail
output=${PAX_PROBE_OUTPUT:-/tmp/pax-1038-linux-probes}
mkdir -p "$output"
export PAX_PROBE_OUTPUT="$output"
export PAX_PROBE_SELF_TEST=1
export PAX_PROBE_EXTERNAL_INPUT=1
export PAX_PROBE_SCREENSHOT="$output/gtk-x11.png"
export GDK_BACKEND=x11
export GSK_RENDERER=${GSK_RENDERER:-gl}
exec xvfb-run -a bash -c '
set -euo pipefail
timeout 30s "$PAX_PROBE_OUTPUT/gtk-composition" >"$PAX_PROBE_OUTPUT/gtk-x11.log" 2>&1 &
probe_pid=$!
trap '\''kill "$probe_pid" 2>/dev/null || true'\'' EXIT
for step in {1..100}; do
    if grep -q INPUT_READY "$PAX_PROBE_OUTPUT/gtk-x11.log"; then break; fi
    if ! kill -0 "$probe_pid" 2>/dev/null; then cat "$PAX_PROBE_OUTPUT/gtk-x11.log"; exit 1; fi
    sleep 0.05
done
if ! grep -q INPUT_READY "$PAX_PROBE_OUTPUT/gtk-x11.log"; then
    cat "$PAX_PROBE_OUTPUT/gtk-x11.log"
    printf "%s\n" "GTK probe did not become ready for keyboard input" >&2
    exit 1
fi
window_id=$(xdotool search --name "PAX-1038 GTK native composition probe" | head -1)
read -r hole_x hole_y fraction_x fraction_y < <(sed -n "s/^INPUT_POINTS //p" "$PAX_PROBE_OUTPUT/gtk-x11.log" | head -1)
xdotool windowfocus --sync "$window_id"
xdotool mousemove --sync --window "$window_id" "$hole_x" "$hole_y"
xdotool click --clearmodifiers 1
xdotool mousemove --sync --window "$window_id" "$fraction_x" "$fraction_y"
xdotool click --clearmodifiers 1
xdotool key --clearmodifiers ctrl+a
xdotool type --clearmodifiers --delay 6 "Pax native edit retained"
xdotool key --clearmodifiers ctrl+a
status=0
wait "$probe_pid" || status=$?
cat "$PAX_PROBE_OUTPUT/gtk-x11.log"
exit "$status"
'
