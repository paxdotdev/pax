# Native GPU lab on EC2 — provisioning runbook

This directory provisions the PAX-1038 qualification lab for Windows Server 2022
Desktop Experience and Ubuntu 24.04 x86-64 on NVIDIA T4 `g4dn.xlarge` instances
in Oregon. The October 15 pilot is authorized with separate $20 planning envelopes
for Windows and Linux, including Marketplace acceptance and artifact uploads.
Each guest has a four-hour stop deadline. These envelopes are not automatic
billing caps; later runs need their own recorded execution scope.

The current scope and review discussion live in
[PAX-1038](https://linear.app/paxdev/issue/PAX-1038/native-windows-and-linux-support).
This README documents the files and their operating contract. It does not add
Windows or Linux to Pax's supported application targets.

## Files and ownership

| Path | Responsibility |
| --- | --- |
| `terraform/bootstrap` | New private, encrypted, versioned S3 state bucket; local bootstrap state must be retained. |
| `terraform/lab` | Isolated VPC/subnet, IAM, security groups, artifact bucket, expiry service, and zero or one test VM. |
| `packer` | One OS image per invocation, using the lab's existing network and IAM profile. |
| `guest` | Guest-only setup scripts and a VS component selection; never run on the workstation. |
| `examples` | Deliberately incomplete templates; account-specific completed inputs stay ignored. |
| `pins` | Reviewed base AMI IDs, vendor artifact URLs/hashes and validation tool versions. |
| `tools` | Offline checks, a live build guard, Windows layout preparation, and the AWS expiry handler. |

Terraform owns the lab resources. It can launch the exact NVIDIA Marketplace
images in `pins/vendor-images.json` directly, or an image baked by this lab.
Packer owns its temporary builder/keypair and
creates AMIs and snapshots; it does **not** manage their subsequent retirement.
An image manifest records the AMI, base AMI and recipe hash. AMIs, snapshots,
builder volumes, and instances carry lab ownership tags. Do not import or reuse
the superseded blog Terraform configuration or existing production resources.

## Reproducibility boundary

Base AMIs are explicit IDs with official publisher, OS, architecture, and storage
filters. There is no `most_recent` image selection. For `image_source="baked"`,
Terraform accepts only this account's images with matching lab, OS and recipe
tags. For `image_source="vendor"`, it requires the exact OS-specific AMI ID,
owner, name and Marketplace product code in the reviewed pins. Marketplace's
shared owner account alone is insufficient to establish publisher identity.

Installer manifests require an exact version, URL/S3 key, and SHA256 for every
artifact. The scripts verify bytes before executing them. Fill the examples only
from independently verified official downloads or reviewed private mirrors.
Credential-bearing or signed URLs are forbidden in these files. The guest role
can read `inputs/` in the private artifact bucket and official NVIDIA driver/DCV
license objects, and write evidence under `runs/`; it cannot delete evidence or
modify infrastructure. Mirror objects should use content-addressed keys.

The optional Ubuntu baking recipe uses one explicit Canonical snapshot and the base AMI's exact kernel. `kernel_release="base-ami"` inherits that kernel from the pinned AMI;
setup records its actual release before installing the matching headers. The
requested manifest and resolved manifest are both retained, and the post-reboot
check rejects a changed kernel. An explicit kernel release is also accepted. Windows uses an archived VS 2022 offline layout, including its catalog
and channel manifest, with `--noWeb --noUpdateInstaller`; the archive must contain
all components in `guest/buildtools.vsconfig`. Rust is an exact release installed
through checksum-verified rustup; rustup verifies its downloaded components.
Automatic OS updates are disabled for the disposable test image so the driver
stack does not change during qualification. Refresh by rebuilding with reviewed
inputs; these are not unattended long-lived workstations.

The image stores the input manifest, setup log, installed-package inventory, OS,
kernel and GPU inventory. Test reports must additionally record the **actual**
driver, adapter identity, session type, resolution, source revision (including a
hash for any staged uncommitted patch), executable hashes, and probe logs.

This makes the software inputs and resulting AMI traceable. It does not promise
bit-identical image rebuilds, indefinitely available upstream downloads, identical
EC2 host placement, or identical performance. Repeated runs should launch the
same retained AMI. Archive reviewed inputs before their upstream retention ends.

## Access and sessions

Instances have ephemeral public IPv4 addresses for outbound SSM/download access.
There is no NAT gateway, load balancer, bastion, or open SSH/RDP rule. Runtime
ingress is closed by default. SSM provides command access and can forward TCP
8443 for DCV. An optional single-client `/32` enables direct DCV TCP and QUIC;
record whether a run used the tunnel or direct connection when evaluating streaming.

For an SSM-forwarded desktop, point the native DCV client at the listener's
explicit IPv4 address, for example `127.0.0.1:18443` when local port 18443 forwards
guest port 8443, and use TCP. A Session Manager listener can bind IPv4 loopback
only; `localhost` can resolve to IPv6 and fail before the
credential screen with a misleading authentication/connection error. Check the
local listener before changing credentials or guest access controls. SSM port
forwarding does not carry DCV's UDP/QUIC transport.

If the native client remains on its credential screen or clears the password
without an error, inspect the current client's log before retrying. A credential
prompt can outlive its authentication connection: the 2025 macOS client logged
`Missing username or password`, followed by a server-closed WebSocket, then kept
submitting against that closed connection. Quit and reopen the client to start a
fresh handshake, using the same verified server identity and existing credential.
That client's log is `~/.local/share/NICE/dcvviewer/log/viewer.log`; other client
versions may use a different location. A stale log from another version is not
evidence about the current connection.

Windows native input qualification requires a currently connected DCV desktop.
The existing idle timeout can disconnect it during long build waits; a subsequent
input preflight can then observe `SetCursorPos`/`GetCursorPos` returning `(0,0)`.
Preserve that failed precondition and reconnect before rerunning the checker.
Do not weaken the idle timeout, desktop-access checks, or security settings to
make an unattended input run pass.

Packer uses SSH on Linux and TLS/NTLM WinRM on Windows, through a separate builder
security group restricted to the operator's `/32`. The Windows builder uses a
temporary self-signed certificate, so Packer disables certificate verification
for that connection. It uses the EC2-generated Administrator password, not a
password in Terraform state or user data. The baked image removes the temporary
listener/certificate and disables WinRM at first boot. This cleanup still needs
an actual launch test. No automatic desktop login or reusable password is baked.

After launch, establish desktop credentials through an interactive SSM session,
or generate an ephemeral password inside the disposable guest and return it
encrypted to an operator-held public key. Keep the private key and decrypted
password in mode-600 local files, outside the repository and evidence archives.
Do not place passwords in Run Command strings or output, manifests, user data,
or Terraform state. The guest-generated/encrypted method was used for the pilot.
Ubuntu's `pax` account is password-locked until configured. On Windows, configure
the intended desktop account and install the retained, verified
`C:\PaxGpuLab\WindowsAppRuntimeInstall.exe` in that user's context before WinUI
tests. Rust is under `C:\PaxGpuLab`; MSVC is under `C:\BuildTools`. If using a
non-administrator development account, give it its own writable Cargo cache and
Rust toolchain rather than granting write access to the image's setup files.

SSM commands run outside the interactive desktop. They can build and collect
artifacts, but a successful SYSTEM/session-0 command cannot qualify native
composition. Launch the Windows interactive checker in the logged-in desktop.
The Linux recipe configures a 1920×1080 NVIDIA-backed **Xorg console** and DCV;
it does not install DCV-GL or claim Wayland qualification. Native Wayland and
DCV's newer Wayland support need a separate verified image/session configuration.

## Expiry and cleanup

The foundation provisions the expiry service **before** any Terraform test VM.
Before a Packer build, verify that foundation is applied and its schedule and
Lambda are healthy; Packer does not create the guard itself.

Every five minutes, an AWS-side job examines only instances carrying both this
lab's `PaxGpuLab` value and `ManagedBy=pax-gpu-lab`. It requests a stop at the
earlier of the explicit UTC deadline and four hours since launch. Missing or
invalid deadlines on owned running guests cause a stop. The absolute deadline
also prevents a restart from implicitly granting another four-hour session.
The role has tag-constrained stop permission and no terminate permission.

This is a backup stop mechanism, **not a hard billing cap**: scheduling delays,
API failures and stopping latency can extend runtime. Stopped disks, AMI
snapshots, S3 objects, and logs still incur storage charges. The guard does not
enforce a global concurrency limit. Keep builds and test runs sequential within
each lab. Separately approved concurrent OS pilots require distinct lab names,
artifact buckets, backend keys, and `TF_DATA_DIR` directories; never point two
active configurations at the same state.
`tools/build_image.py` rejects a build if an owned pending/running/stopped guest
already exists. This is an operational check, not an atomic cross-operator lock.

For an iteration lab, export evidence and stop the instance while retaining its
root volume and Terraform `machine` configuration. Verify EC2 reaches `stopped`;
a successful stop request is not that verification. Stopped instances do not
incur instance compute charges, but their provisioned EBS volumes and retained
S3/log data continue to incur storage charges. Do not set `machine = null` when
the intent is to retain the installed development environment.

Before resuming, record the new execution scope and budget, update
`machine.expires_at` in the lab's ignored tfvars to a fresh UTC deadline no more
than four hours ahead, and review/apply the saved Terraform plan. The plan should
only refresh the owned guest's deadline tag; stop if it proposes replacement or
unrelated changes. Verify the AWS expiry job is healthy and the new tag reached
that exact instance, then use EC2 `start-instances`. Terraform does not start an
already-created stopped guest. Wait for both EC2 running state and SSM Online,
and recheck the installed driver/kernel/session before qualification. The public
address and physical GPU UUID may change after stop/start. Rerun the hardware
preflight and select the newly verified device; do not reuse an old UUID from a
previous session. The old absolute deadline intentionally prevents an
unreviewed restart from granting another session.

EC2 reports no current public IPv4 association while an instance is stopped.
The instance resource ignores changes to `associate_public_ip_address` after
creation, preserving its explicit launch setting without replacing a retained
disk during a deadline refresh. The reviewed resume plan must still show zero
additions/destructions; never apply a replacement merely to refresh an expiry.
Keep the foundation's original approval provenance tag when only resuming, and
record the new session authorization/budget with the run evidence.

When retiring the environment, set `machine = null` and review/apply that plan
to terminate the instance and delete its root disk while retaining the foundation.
Packer normally removes its temporary builder, but inspect for
orphaned tagged instances, volumes, and temporary keypairs after interrupted
builds. An expired builder is stopped, not automatically deleted. Keep the guard
until all builders/test VMs are removed. Explicitly review retired AMIs and their
snapshots before deleting them. A full lab destroy refuses to erase a nonempty
artifact bucket; bootstrap separately prevents state-bucket destruction.

Verify both EC2's final `terminated` state and absence of the root volume; a
successful termination request is not that verification. Both October 15 vendor
guests stalled in `shutting-down`. After verifying exported evidence and allowing
six minutes for normal shutdown, the disposable guests required AWS's
[force-termination path](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/TroubleshootingInstancesShuttingDown.html).
Skipping OS shutdown is only appropriate after guest data is safely exported;
record the intervention and let the pending Terraform apply reconcile deletion.

## Execution sequence

Record approval and review each saved plan before applying it.

### Direct vendor AMI pilot

The initial hardware pilot can skip Packer entirely. NVIDIA's
[Windows Server 2022](https://aws.amazon.com/marketplace/pp/prodview-f4reygwmtxipu)
and [Ubuntu 24.04](https://aws.amazon.com/marketplace/pp/prodview-fkv4hqrlaflww)
vGPU 20.2 images include their graphics driver. Both list `g4dn.xlarge` in Oregon
and no additional software charge. Marketplace subscription/terms acceptance is
required; both subscriptions were accepted for the authorized October 15 pilot. These are NVIDIA
images distributed through AWS Marketplace, not images published by AWS itself.

Use steps 1–4 below to create state and the foundation, with builder ingress
closed (`builder_ipv4_cidr=null`). Verify the expiry function's recent successful
invocation before launching a guest. Then use the ignored
`lab-vendor-windows.tfvars.json` or `lab-vendor-linux.tfvars.json`, filling only
the real approval reference and a fresh deadline. Review a fresh plan against
the real backend before applying. The October 15 scratch previews add 27 lab
resources including one guest, with no change or destruction of existing resources;
bootstrap adds five separately. The foundation alone has 26 resources with
builder ingress closed.

For Windows checks, stage a freshly built x64 bundle and its source revision,
patch/per-file hashes, binary hashes and build logs. It need not be compiled on
the GPU VM. The October 15 pilot used a verified fresh Debug bundle containing
the WinUI app, Pax/WGPU bridge, interop diagnostic, hardware preflight and
offscreen test. Debug results do not establish release performance. First run
`python3 tools/check_sources.py windows.inputs.local.json --windows-runtime-only`
on the host to check the three small runtime downloads;
the unused VS archive need not exist in S3. Through SSM, copy the reviewed Windows input manifest to
`C:\Windows\Temp\pax-lab-inputs.json` and run:

```powershell
.\setup-windows.ps1 -RuntimeOnly
Restart-Computer
```

This mode installs only the verified VC runtime and DCV, and retains the WinAppSDK
runtime installer for the interactive user. It preserves the vendor driver and
does not download VS, Git or Rust. After reboot, set desktop credentials through
an interactive SSM session, register the WinAppSDK runtime as that user, and run
the hardware checker. Run the native interop diagnostic in the logged-in desktop.
The full WinUI interactive checker needs the matching x64 build. Confirm SSM is
online before continuing; the vendor Windows image took approximately ten
minutes to register on its first boot in this pilot. Runtime-only setup has now
been exercised on EC2. It imports PowerShell modules explicitly and waits on each
installer process with a deadline. After cancellation or timeout, inspect the
identified installer and its logs before retrying: cancelling an SSM command can
leave its installer holding downloaded files open. Development tools can be
added later if qualification passes.

The October 15 Windows pilot passed all 37 interactive checks with the real
Pax producer and the explicit native COM UIA selection mode. Selected and resized
captures also passed visual inspection. The managed PowerShell UIA text-range
path crashed on the pinned image; its failure remains separate evidence. See the
[Windows probe guide](../../tests/src/native-platform-probes/windows/README.md#ec2-server-2022-hardware-pilot)
for the qualified path and limits. An observed DCV stream over the SSM TCP tunnel
averaged 17.8 fps with 158.7 ms reported network latency; this single observation
supports remote inspection, not guest presentation-rate or performance claims.
Both pilot instances and their root volumes were removed after evidence export;
the private state/artifact buckets and lab foundations remain for subsequent runs.

For Linux, `guest/setup-linux-pilot.sh` preserves the vendor kernel and NVIDIA
driver. The Canonical snapshot endpoint was unavailable during preparation, so
the pilot explicitly resolves a minimal GTK/Xorg toolchain from signed official
live Noble repositories and archives the indexes, exact package versions, `.deb`
files and SHA256 hashes before installing with `--no-download`. This is an archived
live resolution, not a claim that the unavailable snapshot was reproduced. The
transaction guard rejects removals and kernel/NVIDIA changes, and setup verifies
that the running kernel, loaded module and GPU driver remain unchanged.
`guest/start-linux-pilot-xorg.sh` starts an authenticated NVIDIA Xorg console;
`PAX_PROBE_X11_USE_EXISTING_DISPLAY=1` makes the interactive checker use that
display while retaining its real input assertions. DCV and Wayland are separate
checks; neither is established by an Xorg-only run.

The October 15 Linux pilot passed real Xorg input and retained GPU composition
with GTK 4.14.5's `ngl` renderer. The legacy `gl` renderer repeated the last tile;
importing a producer fence into the DMA-BUF reservation object also failed a
forced-pending synchronization check. The opt-in explicit EGL fence path passed
that check with one GPU conversion into owned GL storage. See the
[Linux probe guide](../../tests/src/native-platform-probes/linux/README.md#nvidia-xorg-pilot-and-explicit-synchronization)
for its commands, evidence boundary, and remaining lifetime/performance limits.
The guest and root disk were removed after exporting the reviewed evidence.

This path pins the starting filesystem and records subsequent changes; it does
not create a reusable custom AMI. Terminate the guest after evidence export.
The optional baking path below is useful once a working environment is worth
retaining, rather than a prerequisite for learning whether the GPU gate passes.

### Optional custom image baking

1. Approve account/region, cost/time limits, image retention, and execution.
   Use the reviewed pins and completed ignored inputs; keep approval references
   unset until approval actually arrives. Recheck the operator IP, quotas and
   AMI availability. Regional offerings do not guarantee available capacity.
2. Use the versions in `pins/toolchain.json`. On this workstation AWS 2.38.1 and
   Terraform 1.16.2 are under `~/.local/bin`; the inherited PATH finds older
   copies. Packer 1.16.1 and Session Manager plugin 1.2.896.0 were unpacked in
   `/tmp/pax-1038-preflight`, without replacing system tools. Use explicit paths.
   Terraform's checked-in lockfiles contain the provider checksums obtained
   from HashiCorp. Packer pins its Amazon plugin to 1.8.3.
3. Initialize/validate/plan `terraform/bootstrap`, review a fresh plan, then
   apply to create only the state bucket. Preserve its local state securely.
4. Initialize/validate/plan `terraform/lab` with the S3 backend and `machine=null`.
   Review and apply the foundation. Configure builder ingress for the current
   operator `/32`; copy `packer_network` outputs into the JSON Packer inputs.
   The pre-approval foundation preview uses a scratch local backend because
   the future state bucket does not exist; it is not a saved executable plan.
5. Prepare the Windows offline-layout archive using `tools/prepare_vs_layout.ps1`
   on a Windows workstation. This downloads and verifies the selected components
   without installing VS. It checks the fixed-version bootstrapper's hash and
   Microsoft signature, verifies the layout, then emits a ZIP and `layout-pin.json`.
   Archive the whole layout, including its installer, catalog and channel. After
   approval, upload that ZIP under `inputs/<sha256>/vs-layout.zip` in the lab
   artifact bucket with `--metadata sha256=<sha256>`, then set its exact URI/hash
   in `windows.inputs.local.json`. `pins/vs-layout.json` records the locally
   completed and verified 17.14.42 archive; no upload has been performed.
   Do not start a Windows image build with an incomplete archive or a moving
   bootstrapper-only input. The layout preparation command may download a newer
   installer engine; the resulting archive hash pins those bytes for all builds.
6. Validate the completed image manifest offline, initialize/validate Packer,
   then use `tools/build_image.py` for exactly one OS. Before launching Packer,
   it verifies source availability, the account, network/profile ownership, restricted ingress, idle
   lab, deadline, active expiry schedule/function, and a recent successful expiry
   invocation after deployment. It defaults to checking only; `--execute` builds.
   The wrapper is required because Packer's Amazon builder has no Terraform-style
   `allowed_account_ids` option. Preserve its manifest and image inventories;
   inspect cleanup before switching OSes.
7. Put that image's exact AMI ID/recipe hash and a fresh UTC deadline in `machine`.
   Review a fresh Terraform plan, apply it, configure desktop access, and execute
   the hardware and interactive probes. Collect evidence, then retire the VM.

Commands from this directory (substitute the explicit validated executable paths):

```sh
# Local validation is authorized; it does not launch anything.
python3 tools/check_inputs.py linux linux.inputs.local.json
python3 tools/check_inputs.py windows windows.inputs.local.json
# Read-only network check. A custom Windows build requires the archive staged
# in S3; a Linux build requires all three snapshot InRelease endpoints to work.
python3 tools/check_sources.py linux.inputs.local.json --aws /absolute/path/to/aws
terraform -chdir=terraform/bootstrap init -backend=false
terraform -chdir=terraform/bootstrap validate
terraform -chdir=terraform/lab init -backend=false
terraform -chdir=terraform/lab validate
terraform -chdir=terraform/lab test
packer init packer
packer validate -var-file=windows.pkrvars.json packer

# After execution approval, fill approval references and use fresh plans.
terraform -chdir=terraform/bootstrap plan -var-file=../../bootstrap.tfvars.json -out=bootstrap.tfplan
# Review, then apply the saved plan within the approved scope.
terraform -chdir=terraform/lab init -reconfigure -backend-config=../../backend.hcl
terraform -chdir=terraform/lab plan -var-file=../../lab.tfvars.json -out=lab.tfplan
# Review, then apply the foundation plan before attempting a build.

python3 tools/build_image.py windows.pkrvars.json --aws /absolute/path/to/aws --packer /absolute/path/to/packer
# Add --execute only after approval and a successful live check.
```

The live check is deliberately expected to fail before the foundation is
provisioned. A validation success with placeholder network IDs establishes only
Packer schema correctness. It is not a live launch check.

Keep state, plans, credentials, vendor installers, generated manifests, and
screenshots out of Git. Do not use `-auto-approve`, `-force`, or Packer AMI
retirement overrides. The initial root volume is already encrypted; Packer does
not request an additional AMI encryption copy.

## Qualification and costs

Use the existing [Windows probe runbook](../../tests/src/native-platform-probes/windows/README.md)
and [Linux probe runbook](../../tests/src/native-platform-probes/linux/README.md).
Require hardware admission explicitly; never enable WARP/software diagnostics to
turn a failed hardware gate green. `nvidia-smi` only checks driver installation.
Server 2022 does not establish Windows 10 client coverage. Guest frame timing and
lossless captures must be assessed separately from DCV latency/compression.

The October 15 read-only Oregon quote was $0.710/hour for license-included Windows
and $0.526/hour for Linux `g4dn.xlarge`. Four test hours per OS total $4.944 in
compute. **Image builds are additional**: another four hours per OS would bring
that combined compute estimate to $9.888. EBS, AMI snapshots, IPv4, S3, logs,
streaming/egress and taxes are additional. The October 15 authorization provides
$20 per OS, and there is no automatic dollar ceiling. A direct vendor
Windows pilot capped at four hours needs $2.84 compute, with no image-build phase
or custom AMI snapshot retention. Provisioning, setup and testing share that same
four-hour window. Reprice the
complete build/test/retention schedule before subsequent runs.

## Validation and remaining runtime checks

The offline check is safe to run without AWS credentials:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools -p 'test_*.py' -v
bash -n guest/setup-linux.sh
bash -n guest/finish-linux.sh
```

Terraform format/schema validation and plan previews, both Packer OS schemas,
Windows PowerShell 5.1 parsing, Bash syntax, 28 offline Python checks and seven
mocked Terraform tests have been exercised. The current review evidence and unresolved input
preparation details are recorded in PAX-1038.

Both direct vendor guests have launched through Terraform and registered with
SSM; Windows runtime-only setup and the minimal Linux package archive/install
path have been exercised. Windows hardware-only Pax/WGPU admission and the
offscreen alpha/clipping/resize-lifetime test pass on T4/DX12. Linux Vulkan/GDK
admission passes on T4/Xorg. The bounded native composition checks also pass on
Windows with native COM UIA and on Linux with `ngl` and the explicit EGL fence
bridge; the probe guides preserve the failed controls and qualification limits.
Neither pilot establishes production scheduling or release performance.
Packer image builds, baked-image WinRM cleanup, and
offline VS installation remain untested; keep them separate from vendor-pilot
results. Permission simulation cannot prove every runtime
condition, organizational policy or future API result. No application support
claim follows from provisioning validation.

References: [S3 state and locking](https://developer.hashicorp.com/terraform/language/backend/s3),
[Packer EC2 images](https://developer.hashicorp.com/packer/integrations/hashicorp/amazon/latest/components/builder/ebs),
[Ubuntu snapshots](https://snapshot.ubuntu.com/),
[AWS NVIDIA GRID](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/nvidia-GRID-driver.html),
[Windows image generalization](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/sysprep-using-ec2launchv2.html).
