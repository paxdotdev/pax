#!/usr/bin/env bash
# Start a real NVIDIA Xorg console for the disposable pilot, not Xvfb/Xdcv.
set -euo pipefail
[[ $EUID -eq 0 && $(cat /sys/class/dmi/id/sys_vendor) == 'Amazon EC2' ]]
[[ $(nvidia-smi --query-gpu=driver_version --format=csv,noheader) == 595.91.07 ]]
id pax >/dev/null
if [[ -e /tmp/.X11-unix/X0 ]] || systemctl is-active --quiet pax-pilot-xorg.service; then
    echo 'An X server is already present; inspect and reuse its authenticated session instead of replacing it' >&2
    exit 1
fi
pilot_root=${PAX_PILOT_ROOT:-/opt/pax-linux-pilot}
install -d -m 755 "$pilot_root"
run_dir=$(mktemp -d "$pilot_root/xorg.XXXXXX")
chmod 755 "$run_dir"
nvidia-smi --query-gpu=name,uuid,pci.bus_id,driver_version --format=csv > "$run_dir/gpu.csv"
# EC2 also exposes a primary framebuffer. Bind the X screen to the actual NVIDIA
# PCI device rather than letting Xorg probe the framebuffer as its primary GPU.
bus_id=$(nvidia-smi --query-gpu=pci.bus_id --format=csv,noheader | python3 -c '
import re,sys
value=sys.stdin.read().strip()
m=re.fullmatch(r"([0-9A-Fa-f]+):([0-9A-Fa-f]+):([0-9A-Fa-f]+)\.([0-9A-Fa-f]+)",value)
if not m: raise SystemExit("Expected exactly one NVIDIA PCI address")
domain,bus,device,function=(int(v,16) for v in m.groups())
print(f"PCI:{bus}@{domain}:{device}:{function}")')
cat > "$run_dir/xorg.conf" <<EOF
Section "ServerLayout"
    Identifier "PaxPilot"
    Screen 0 "PaxScreen"
EndSection
Section "Device"
    Identifier "PaxGPU"
    Driver "nvidia"
    BusID "$bus_id"
    Option "AllowEmptyInitialConfiguration" "True"
EndSection
Section "Screen"
    Identifier "PaxScreen"
    Device "PaxGPU"
    DefaultDepth 24
    Option "MetaModes" "1920x1080 +0+0"
    SubSection "Display"
        Depth 24
        Virtual 1920 1080
    EndSubSection
EndSection
EOF
touch "$run_dir/Xauthority"
chmod 600 "$run_dir/Xauthority"
cookie=$(python3 -c 'import secrets; print(secrets.token_hex(16))')
xauth -f "$run_dir/Xauthority" add :0 MIT-MAGIC-COOKIE-1 "$cookie"
unset cookie
chown pax:pax "$run_dir/Xauthority"
systemd-run --unit=pax-pilot-xorg --collect /usr/lib/xorg/Xorg :0 \
    -config "$run_dir/xorg.conf" -auth "$run_dir/Xauthority" -nolisten tcp \
    -noreset -logfile "$run_dir/Xorg.log"
for attempt in {1..100}; do
    if runuser -u pax -- env DISPLAY=:0 XAUTHORITY="$run_dir/Xauthority" xdpyinfo > "$run_dir/xdpyinfo.txt" 2>&1; then break; fi
    if ! systemctl is-active --quiet pax-pilot-xorg.service; then
        journalctl -u pax-pilot-xorg.service --no-pager
        exit 1
    fi
    sleep 0.1
done
runuser -u pax -- env DISPLAY=:0 XAUTHORITY="$run_dir/Xauthority" glxinfo -B > "$run_dir/glxinfo.txt"
grep -q 'OpenGL vendor string: NVIDIA' "$run_dir/glxinfo.txt"
printf 'DISPLAY=:0\nXAUTHORITY=%q\n' "$run_dir/Xauthority" > "$run_dir/session.env"
cat "$run_dir/glxinfo.txt"
printf 'PILOT_XORG_DIR=%s\n' "$run_dir"
