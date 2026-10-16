#!/usr/bin/env bash
# Packer guest only. Never source or execute this on the workstation.
set -euo pipefail
[[ $EUID -eq 0 && $(uname -m) == x86_64 ]]
[[ $(cat /sys/class/dmi/id/sys_vendor) == 'Amazon EC2' ]]
source /etc/os-release
[[ $ID == ubuntu && $VERSION_ID == 24.04 ]]
python3 /tmp/pax-lab-check-inputs.py linux /tmp/pax-lab-inputs.json

root=/opt/pax-lab
install -d -m 755 "$root" "$root/downloads"
install -m 644 /tmp/pax-lab-inputs.json "$root/inputs.json"
install -m 644 /tmp/pax-lab-inputs.json "$root/inputs.requested.json"
exec > >(tee "$root/setup.log") 2>&1
value() {
    python3 - "$root/inputs.json" "$1" <<'PY'
import json, sys
value = json.load(open(sys.argv[1]))
for key in sys.argv[2].split('.'):
    value = value[key]
print(value)
PY
}
# An explicit AMI ID already pins its kernel. Record that resolved value before
# installing anything; no moving kernel meta-package or dist-upgrade is used.
if [[ $(value kernel_release) == base-ami ]]; then
    python3 - "$root/inputs.json" <<'PY'
import json, pathlib, platform, re, sys
path = pathlib.Path(sys.argv[1])
data = json.loads(path.read_text())
kernel = platform.release()
if not re.fullmatch(r'[0-9][0-9A-Za-z.+-]*-aws', kernel):
    raise SystemExit('Expected an AWS kernel in the pinned AMI')
data['kernel_release'] = kernel
path.write_text(json.dumps(data, indent=2) + '\n')
PY
fi
[[ $(uname -r) == "$(value kernel_release)" ]]
kernel_gcc_major=$(python3 - <<'PY'
import re
match = re.search(r'\bgcc-(\d+)\b', open('/proc/version').read())
if not match:
    raise SystemExit('Cannot identify the pinned AWS kernel GCC major version')
print(match.group(1))
PY
)

# Restrict the disposable image to official, date-pinned Ubuntu repositories.
# No dist-upgrade: the kernel must remain the one recorded in the manifest.
cloud-init status --wait
systemctl disable --now apt-daily.timer apt-daily-upgrade.timer
systemctl mask apt-daily.service apt-daily-upgrade.service
# Let a previously started update finish before changing its repository inputs.
for attempt in {1..60}; do
    if ! systemctl is-active --quiet apt-daily.service && ! systemctl is-active --quiet apt-daily-upgrade.service; then
        break
    fi
    sleep 5
done
if systemctl is-active --quiet apt-daily.service || systemctl is-active --quiet apt-daily-upgrade.service; then
    echo 'Automatic package update did not finish within five minutes' >&2
    exit 1
fi
install -d "$root/original-apt"
cp -a /etc/apt/sources.list.d "$root/original-apt/"
rm -f /etc/apt/sources.list.d/*.list /etc/apt/sources.list.d/*.sources
printf '' > /etc/apt/sources.list
cat > /etc/apt/sources.list.d/pax-lab.sources <<EOF
Types: deb
URIs: https://archive.ubuntu.com/ubuntu
Suites: noble noble-updates noble-security
Components: main restricted universe multiverse
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg
Snapshot: $(value ubuntu_snapshot)
EOF
export DEBIAN_FRONTEND=noninteractive
# A partial snapshot fetch must never reuse stale indexes from the base image.
apt-get -o APT::Update::Error-Mode=any update
apt-get install -y --no-install-recommends \
    ca-certificates curl awscli git build-essential pkg-config libssl-dev \
    python3 unzip dkms "gcc-$kernel_gcc_major" "linux-headers-$(uname -r)" \
    libgtk-4-dev libvulkan-dev vulkan-tools libepoxy-dev \
    libgl1-mesa-dev libegl1-mesa-dev libdrm-dev xvfb xauth xdotool weston \
    gnome-shell gnome-session gdm3 xserver-xorg-core xfonts-base dbus-x11 mesa-utils

fetch() {
    local id=$1 extension=$2 uri output
    uri=$(value "artifacts.$id.uri")
    output="$root/downloads/$id.$extension"
    if [[ $uri == s3://* ]]; then
        aws s3 cp "$uri" "$output" --region us-west-2 --only-show-errors
    else
        curl --fail --location --proto '=https' --proto-redir '=https' --retry 3 "$uri" -o "$output"
    fi
    printf '%s  %s\n' "$(value "artifacts.$id.sha256")" "$output" | sha256sum --check --status
}
fetch rustup bin
fetch grid run
fetch dcv tgz

# GRID graphics driver, never the compute-only/TCC package. GSP must be disabled
# for this G4dn configuration per AWS's current GRID installation instructions.
printf 'blacklist nouveau\noptions nouveau modeset=0\n' > /etc/modprobe.d/pax-nouveau.conf
printf 'options nvidia NVreg_EnableGpuFirmware=0\n' > /etc/modprobe.d/pax-nvidia.conf
printf 'options nvidia_drm modeset=1\n' >> /etc/modprobe.d/pax-nvidia.conf
update-initramfs -u
systemctl stop display-manager.service || true
# The open module requires GSP; AWS requires GSP off for this T4 setup.
export CC="gcc-$kernel_gcc_major"
bash "$root/downloads/grid.run" --silent --dkms --disable-nouveau --kernel-module-type=proprietary
install -d "$root/downloads/dcv"
tar -xzf "$root/downloads/dcv.tgz" -C "$root/downloads/dcv" --no-same-owner
dcv_packages=("$root"/downloads/dcv/*/nice-dcv-server_*_amd64.ubuntu2404.deb)
[[ ${#dcv_packages[@]} -eq 1 && -f ${dcv_packages[0]} ]]
apt-get install -y "${dcv_packages[0]}"
# The browser viewer is a separate package in DCV 2026.0.
dcv_web=("$root"/downloads/dcv/*/nice-dcv-web-viewer_*_amd64.ubuntu2404.deb)
[[ ${#dcv_web[@]} -eq 1 && -f ${dcv_web[0]} ]]
apt-get install -y "${dcv_web[0]}"

id pax >/dev/null 2>&1 || useradd --create-home --shell /bin/bash pax
usermod -aG video,render pax
usermod -aG video dcv
chmod 755 "$root/downloads/rustup.bin"
install -d -o pax -g pax /home/pax/.cargo /home/pax/.rustup
runuser -u pax -- env HOME=/home/pax CARGO_HOME=/home/pax/.cargo RUSTUP_HOME=/home/pax/.rustup \
    "$root/downloads/rustup.bin" -y --profile minimal --default-toolchain "$(value rust_version)"
runuser -u pax -- env HOME=/home/pax CARGO_HOME=/home/pax/.cargo RUSTUP_HOME=/home/pax/.rustup \
    /home/pax/.cargo/bin/rustc --version | tee "$root/rust.txt"
pkg-config --atleast-version=4.14 gtk4

# Console session on the NVIDIA-backed Xorg desktop; never Xdcv/DCV-GL for
# hardware qualification. Native Wayland is a separate, still-unqualified lane.
nvidia-xconfig --preserve-busid --enable-all-gpus --allow-empty-initial-configuration --virtual=1920x1080
sed -i 's/^#\?WaylandEnable=.*/WaylandEnable=false/' /etc/gdm3/custom.conf
systemctl set-default graphical.target
cat > /etc/dcv/dcv.conf <<'EOF'
[license]
[log]
[session-management]
create-session=true
[session-management/automatic-console-session]
owner="pax"
[connectivity]
web-port=8443
enable-quic-frontend=true
[security]
authentication="system"
EOF
systemctl enable dcvserver
dpkg-query -W -f='${binary:Package}\t${Version}\n' > "$root/packages.tsv"
uname -a > "$root/kernel.txt"
# Password stays locked in the image; set it interactively after launch.
rm -rf "$root/downloads"
rm -f /tmp/pax-lab-inputs.json /tmp/pax-lab-check-inputs.py
