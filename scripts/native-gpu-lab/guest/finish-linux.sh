#!/usr/bin/env bash
set -euo pipefail
[[ $EUID -eq 0 && $(cat /sys/class/dmi/id/sys_vendor) == 'Amazon EC2' ]]
root=/opt/pax-lab
nvidia-smi --query-gpu=name,driver_version --format=csv,noheader | tee "$root/gpu.txt"
python3 - "$root/inputs.json" <<'PY'
import json, subprocess, sys
inputs = json.load(open(sys.argv[1]))
if subprocess.check_output(['uname', '-r'], text=True).strip() != inputs['kernel_release']:
    raise SystemExit('kernel changed')
versions = subprocess.check_output(['nvidia-smi', '--query-gpu=driver_version', '--format=csv,noheader'], text=True).splitlines()
if not versions or any(v.strip() != inputs['artifacts']['grid']['version'] for v in versions):
    raise SystemExit('driver version mismatch')
PY
systemctl is-active dcvserver
rm -f /root/.bash_history /home/ubuntu/.bash_history
cloud-init clean --logs --machine-id
rm -f /etc/ssh/ssh_host_*
