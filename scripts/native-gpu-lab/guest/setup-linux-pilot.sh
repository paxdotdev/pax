#!/usr/bin/env bash
# Disposable NVIDIA vendor-AMI guest only. Preserves the installed driver/kernel.
set -euo pipefail
[[ $EUID -eq 0 && $(uname -m) == x86_64 ]]
[[ $(cat /sys/class/dmi/id/sys_vendor) == 'Amazon EC2' ]]
source /etc/os-release
[[ $ID == ubuntu && $VERSION_ID == 24.04 ]]
script_dir=$(cd "$(dirname "$0")" && pwd)
pilot_root=${PAX_PILOT_ROOT:-/opt/pax-linux-pilot}
install -d -m 755 "$pilot_root"
run_dir=$(mktemp -d "$pilot_root/setup.XXXXXX")
printf 'PILOT_SETUP_DIR=%s\n' "$run_dir"
exec > >(tee "$run_dir/setup.log") 2>&1
export LC_ALL=C DEBIAN_FRONTEND=noninteractive NEEDRESTART_MODE=l
python3 - "$run_dir" <<'PY'
import json, pathlib, sys, urllib.request
root = pathlib.Path(sys.argv[1])
base = 'http://169.254.169.254/latest/'
request = urllib.request.Request(base + 'api/token', method='PUT', headers={'X-aws-ec2-metadata-token-ttl-seconds': '60'})
token = urllib.request.urlopen(request, timeout=5).read().decode()
def read(path):
    return urllib.request.urlopen(urllib.request.Request(base + path, headers={'X-aws-ec2-metadata-token': token}), timeout=5).read()
ami = read('meta-data/ami-id').decode()
if ami != 'ami-09c24fc4cd6915a69':
    raise SystemExit('Unexpected AMI: ' + ami)
(root / 'instance-identity.json').write_bytes(read('dynamic/instance-identity/document'))
PY
cloud-init status --wait
uname -r > "$run_dir/kernel.before"
nvidia-smi --query-gpu=name,uuid,pci.bus_id,driver_version --format=csv,noheader > "$run_dir/gpu.before.csv"
python3 - "$run_dir/gpu.before.csv" <<'PY'
import csv,sys
rows = list(csv.reader(open(sys.argv[1])))
if not rows or any(row[-1].strip() != '595.91.07' for row in rows):
    raise SystemExit('Expected the AMI vendor driver 595.91.07')
PY
dpkg-query -W -f='${binary:Package}\t${Version}\n' > "$run_dir/packages.before.tsv"
cp /etc/os-release "$run_dir/os-release"
cp /proc/driver/nvidia/version "$run_dir/nvidia-module.before"
# Stop scheduled changes after any already-running transaction finishes.
systemctl stop apt-daily.timer apt-daily-upgrade.timer
for attempt in {1..60}; do
    if ! systemctl is-active --quiet apt-daily.service && ! systemctl is-active --quiet apt-daily-upgrade.service; then break; fi
    sleep 2
done
if systemctl is-active --quiet apt-daily.service || systemctl is-active --quiet apt-daily-upgrade.service; then
    echo 'An automatic package transaction is still running' >&2
    exit 1
fi
systemctl mask --runtime apt-daily.service apt-daily-upgrade.service apt-daily.timer apt-daily-upgrade.timer
install -d "$run_dir/lists/partial" "$run_dir/cache/archives/partial"
cat > "$run_dir/ubuntu.sources" <<'EOF'
Types: deb
URIs: https://archive.ubuntu.com/ubuntu
Suites: noble noble-updates
Components: main universe
Architectures: amd64
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg

Types: deb
URIs: https://security.ubuntu.com/ubuntu
Suites: noble-security
Components: main universe
Architectures: amd64
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg
EOF
apt_options=(-o "Dir::Etc::sourcelist=$run_dir/ubuntu.sources" -o Dir::Etc::sourceparts=-
    -o "Dir::State::lists=$run_dir/lists" -o "Dir::Cache=$run_dir/cache"
    -o Acquire::Languages=none -o Acquire::Retries=2 -o Acquire::https::Timeout=30
    -o Acquire::IndexTargets::deb::DEP-11::DefaultEnabled=false
    -o Acquire::IndexTargets::deb::CNF::DefaultEnabled=false
    -o APT::Update::Error-Mode=any -o Binary::apt-get::APT::Keep-Downloaded-Packages=true)
# This deliberately uses live official indexes. All exact resolved versions,
# signed indexes, and downloaded package bytes remain in this invocation folder.
apt-get "${apt_options[@]}" update
packages=(build-essential pkg-config libgtk-4-dev libvulkan-dev vulkan-tools libepoxy-dev
    xserver-xorg-core xauth xdotool x11-utils dbus-x11 mesa-utils fonts-dejavu-core)
printf '%s\n' "${packages[@]}" > "$run_dir/requested-packages.txt"
apt-cache "${apt_options[@]}" policy "${packages[@]}" > "$run_dir/requested-policy.txt"
apt-get "${apt_options[@]}" --simulate --no-install-recommends --no-remove install "${packages[@]}" > "$run_dir/apt-plan.txt"
python3 "$script_dir/linux-pilot-packages.py" plan "$run_dir/apt-plan.txt" "$run_dir/transaction.json" > "$run_dir/exact-packages.txt"
mapfile -t exact_packages < "$run_dir/exact-packages.txt"
if [[ ${#exact_packages[@]} -gt 0 ]]; then
    apt-get "${apt_options[@]}" --download-only --no-install-recommends --no-remove -y install "${exact_packages[@]}"
fi
python3 "$script_dir/linux-pilot-packages.py" archive "$run_dir/transaction.json" "$run_dir/cache/archives"
# The second resolution uses the same frozen index files and exact transaction.
# --no-download forbids an installation from silently acquiring new inputs.
if [[ ${#exact_packages[@]} -gt 0 ]]; then
    apt-get "${apt_options[@]}" --no-download --no-install-recommends --no-remove -y install "${exact_packages[@]}"
fi
uname -r > "$run_dir/kernel.after"
nvidia-smi --query-gpu=name,uuid,pci.bus_id,driver_version --format=csv,noheader > "$run_dir/gpu.after.csv"
cp /proc/driver/nvidia/version "$run_dir/nvidia-module.after"
cmp "$run_dir/kernel.before" "$run_dir/kernel.after"
cmp "$run_dir/gpu.before.csv" "$run_dir/gpu.after.csv"
cmp "$run_dir/nvidia-module.before" "$run_dir/nvidia-module.after"
dpkg-query -W -f='${binary:Package}\t${Version}\n' > "$run_dir/packages.after.tsv"
pkg-config --atleast-version=4.14 gtk4
pkg-config --modversion gtk4 vulkan epoxy > "$run_dir/probe-libraries.txt"
id pax >/dev/null 2>&1 || useradd --create-home --shell /bin/bash pax
usermod -aG video,render pax
printf 'READY: %s (live repository transaction archived; no snapshot reproducibility claim)\n' "$run_dir"
