packer {
  required_plugins {
    parallels = {
      source  = "github.com/hashicorp/parallels"
      version = "~> 1.2"
    }
  }
}

variable "vm_name" {
  type    = string
  default = "pax-ubuntu-first-touch"
}

variable "iso_url" {
  type    = string
  default = "https://cdimage.ubuntu.com/releases/26.04/release/ubuntu-26.04-live-server-arm64.iso"
}

variable "iso_checksum" {
  type    = string
  default = "sha256:c9aa567e6560b2eddae3af03fc686002e35b6fee96f97fd5df3271e846439fdd"
}

variable "ssh_username" {
  type    = string
  default = "pax"
}

variable "ssh_password" {
  type      = string
  sensitive = true
}

variable "ssh_password_hash" {
  type = string
}

variable "output_directory" {
  type    = string
  default = "/Users/zack/Parallels/pax-ubuntu-first-touch.pvm"
}

variable "wasm_pack_version" {
  type    = string
  default = "0.15.0"
}

source "parallels-iso" "ubuntu" {
  vm_name          = var.vm_name
  guest_os_type    = "ubuntu"
  output_directory = var.output_directory

  iso_url      = var.iso_url
  iso_checksum = var.iso_checksum

  cpus      = 4
  memory    = 6144
  disk_size = 65536
  disk_type = "expand"

  startup_view    = "headless"
  on_window_close = "keep-running"

  parallels_tools_mode = "disable"

  http_content = {
    "/meta-data" = "instance-id: ${var.vm_name}\nlocal-hostname: ${var.vm_name}\n"
    "/user-data" = templatefile("${path.root}/../guest/ubuntu-autoinstall-user-data.pkrtpl", {
      hostname      = var.vm_name
      username      = var.ssh_username
      password_hash = var.ssh_password_hash
    })
  }

  boot_wait = "10s"
  boot_command = [
    "c<wait>",
    "linux /casper/vmlinuz autoinstall ds=nocloud-net\\;s=http://{{ .HTTPIP }}:{{ .HTTPPort }}/ ---<enter><wait>",
    "initrd /casper/initrd<enter><wait>",
    "boot<enter>"
  ]

  ssh_username = var.ssh_username
  ssh_password = var.ssh_password
  ssh_timeout  = "45m"

  shutdown_command = "echo '${var.ssh_password}' | sudo -S shutdown -P now"
  shutdown_timeout = "10m"
}

build {
  sources = ["source.parallels-iso.ubuntu"]

  provisioner "shell" {
    execute_command = "echo '${var.ssh_password}' | {{ .Vars }} sudo -S -E sh -eux '{{ .Path }}'"
    inline = [
      "set -eux",
      "apt-get update",
      "DEBIAN_FRONTEND=noninteractive apt-get install -y ca-certificates curl git build-essential pkg-config libssl-dev python3 unzip xvfb nodejs npm libglib2.0-dev libcairo2-dev libpango1.0-dev",
      "apt-get clean",
      "rm -rf /var/lib/apt/lists/*",
      "printf 'pax-first-touch ubuntu system prerequisites ready\\n'"
    ]
  }

  provisioner "shell" {
    script = "${path.root}/../guest/ubuntu-workstation-prereqs.sh"
    environment_vars = [
      "PAX_FIRST_TOUCH_WASM_PACK_VERSION=${var.wasm_pack_version}"
    ]
  }
}
