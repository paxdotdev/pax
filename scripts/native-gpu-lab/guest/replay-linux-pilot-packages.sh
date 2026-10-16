#!/usr/bin/env bash
# Replays the approved archived official Ubuntu transaction on the same vendor AMI.
set -euo pipefail
[[ $EUID -eq 0 && $(uname -m) == x86_64 ]]
[[ $(cat /sys/class/dmi/id/sys_vendor) == 'Amazon EC2' ]]
source /etc/os-release
[[ $ID == ubuntu && $VERSION_ID == 24.04 ]]
root=${PAX_PILOT_ROOT:-/opt/pax-linux-desktop}
install -d "$root"
run=$(mktemp -d "$root/replay.XXXXXX")
exec > >(tee "$run/replay.log") 2>&1
export DEBIAN_FRONTEND=noninteractive NEEDRESTART_MODE=l LC_ALL=C
cloud-init status --wait
uname -r > "$run/kernel.before"
nvidia-smi --query-gpu=name,uuid,pci.bus_id,driver_version --format=csv,noheader > "$run/gpu.before"
cp /proc/driver/nvidia/version "$run/module.before"
systemctl stop apt-daily.timer apt-daily-upgrade.timer
systemctl mask --runtime apt-daily.service apt-daily-upgrade.service apt-daily.timer apt-daily-upgrade.timer
python3 - "$run" <<'PY'
import boto3,hashlib,pathlib,sys,tarfile,urllib.request
root=pathlib.Path(sys.argv[1]);base='http://169.254.169.254/latest/'
token=urllib.request.urlopen(urllib.request.Request(base+'api/token',method='PUT',headers={'X-aws-ec2-metadata-token-ttl-seconds':'60'}),timeout=5).read().decode()
ami=urllib.request.urlopen(urllib.request.Request(base+'meta-data/ami-id',headers={'X-aws-ec2-metadata-token':token}),timeout=5).read().decode()
if ami!='ami-09c24fc4cd6915a69':raise SystemExit('Unexpected vendor AMI '+ami)
sha='92dde5c97c69d878b4bd114764e1808e84406473bc9405b57207a63580d01a9d'
archive=root/'setup-transaction.tar.gz'
boto3.client('s3',region_name='us-west-2').download_file('pax1038-linux-artifacts-938065685265-us-west-2','inputs/'+sha+'/setup-transaction.tar.gz',str(archive))
if hashlib.sha256(archive.read_bytes()).hexdigest()!=sha:raise SystemExit('Archive hash mismatch')
with tarfile.open(archive) as bundle:bundle.extractall(root/'archive',filter='data')
print('Exact package archive',sha)
PY
mapfile -d '' -t packages < <(find "$run/archive" -path '*/cache/archives/*.deb' -print0)
[[ ${#packages[@]} -eq 228 ]]
# All package bytes come from the previously hashed, signed repository archive.
# dpkg consumes only these exact local archives; it never resolves or downloads
# additional package versions. An incomplete dependency set is a hard failure.
dpkg --install "${packages[@]}"
[[ -z $(dpkg --audit) ]]
uname -r > "$run/kernel.after"
nvidia-smi --query-gpu=name,uuid,pci.bus_id,driver_version --format=csv,noheader > "$run/gpu.after"
cp /proc/driver/nvidia/version "$run/module.after"
cmp "$run/kernel.before" "$run/kernel.after"
cmp "$run/gpu.before" "$run/gpu.after"
cmp "$run/module.before" "$run/module.after"
pkg-config --modversion gtk4 vulkan epoxy
id pax >/dev/null 2>&1 || useradd --create-home --shell /bin/bash pax
usermod -aG video,render pax
printf 'PAX_REPLAY_READY=%s\n' "$run"
