locals {
  # Include the scripts and builder configuration, not only installer versions.
  recipe_sha256 = sha256(join("\n", [
    var.source_ami, var.guest_os, file(var.inputs_file),
    file("${abspath(path.root)}/variables.pkr.hcl"), file("${abspath(path.root)}/images.pkr.hcl"),
    file("${abspath(path.root)}/../guest/setup-linux.sh"),
    file("${abspath(path.root)}/../guest/finish-linux.sh"),
    file("${abspath(path.root)}/../guest/setup-windows.ps1"),
    file("${abspath(path.root)}/../guest/windows-winrm.ps1"),
    file("${abspath(path.root)}/../guest/finish-windows.ps1"),
    file("${abspath(path.root)}/../guest/buildtools.vsconfig"),
    file("${abspath(path.root)}/../tools/check_inputs.py")
  ]))
  image_tags = {
    Project         = "PAX-1038", PaxGpuLab = var.lab_name, ManagedBy = "pax-gpu-lab"
    PaxRecipeSHA256 = local.recipe_sha256, PaxGuestOS = var.guest_os
    Approval        = var.approval_reference
  }
  run_tags = merge(local.image_tags, {
    PaxExpiresAt = var.expires_at, PaxPurpose = "image-build"
  })
}

source "amazon-ebs" "linux" {
  profile = "pax"
  region  = "us-west-2"
  source_ami_filter {
    filters = {
      image-id         = var.source_ami, architecture = "x86_64"
      name             = "ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-*"
      root-device-type = "ebs", virtualization-type = "hvm"
    }
    owners      = ["099720109477"]
    most_recent = false
  }
  ami_name                    = "${var.lab_name}-linux-${substr(local.recipe_sha256, 0, 12)}-{{timestamp}}"
  instance_type               = "g4dn.xlarge"
  subnet_id                   = var.subnet_id
  vpc_id                      = var.vpc_id
  security_group_id           = var.security_group_id
  iam_instance_profile        = var.iam_instance_profile
  associate_public_ip_address = true
  communicator                = "ssh"
  ssh_interface               = "public_ip"
  ssh_username                = "ubuntu"
  ssh_timeout                 = "15m"
  ssh_clear_authorized_keys   = true
  imds_support                = "v2.0"
  metadata_options {
    http_tokens = "required"
  }
  launch_block_device_mappings {
    device_name           = "/dev/sda1"
    volume_type           = "gp3"
    volume_size           = 60
    encrypted             = true
    delete_on_termination = true
  }
  tags            = local.image_tags
  snapshot_tags   = local.image_tags
  run_tags        = local.run_tags
  run_volume_tags = local.run_tags
}

source "amazon-ebs" "windows" {
  profile = "pax"
  region  = "us-west-2"
  source_ami_filter {
    filters = {
      image-id         = var.source_ami, architecture = "x86_64"
      name             = "Windows_Server-2022-English-Full-Base-*"
      root-device-type = "ebs", virtualization-type = "hvm"
    }
    owners      = ["amazon"]
    most_recent = false
  }
  ami_name                    = "${var.lab_name}-windows-${substr(local.recipe_sha256, 0, 12)}-{{timestamp}}"
  instance_type               = "g4dn.xlarge"
  subnet_id                   = var.subnet_id
  vpc_id                      = var.vpc_id
  security_group_id           = var.security_group_id
  iam_instance_profile        = var.iam_instance_profile
  associate_public_ip_address = true
  communicator                = "winrm"
  ssh_interface               = "public_ip"
  winrm_username              = "Administrator"
  winrm_use_ssl               = true
  winrm_use_ntlm              = true
  # The short-lived builder has a self-signed certificate and /32 ingress.
  winrm_insecure           = true
  winrm_timeout            = "20m"
  windows_password_timeout = "20m"
  user_data                = "<powershell>\n${file("${abspath(path.root)}/../guest/windows-winrm.ps1")}\n</powershell>"
  imds_support             = "v2.0"
  metadata_options {
    http_tokens = "required"
  }
  launch_block_device_mappings {
    device_name           = "/dev/sda1"
    volume_type           = "gp3"
    volume_size           = 100
    encrypted             = true
    delete_on_termination = true
  }
  tags            = local.image_tags
  snapshot_tags   = local.image_tags
  run_tags        = local.run_tags
  run_volume_tags = local.run_tags
}

build {
  name    = "native-gpu"
  sources = ["source.amazon-ebs.${var.guest_os}"]

  provisioner "file" {
    only        = ["amazon-ebs.windows"]
    source      = "${abspath(path.root)}/../guest/buildtools.vsconfig"
    destination = "C:/Windows/Temp/pax-buildtools.vsconfig"
  }
  provisioner "file" {
    source      = var.inputs_file
    destination = var.guest_os == "linux" ? "/tmp/pax-lab-inputs.json" : "C:/Windows/Temp/pax-lab-inputs.json"
  }
  provisioner "file" {
    only        = ["amazon-ebs.linux"]
    source      = "${abspath(path.root)}/../tools/check_inputs.py"
    destination = "/tmp/pax-lab-check-inputs.py"
  }
  provisioner "shell" {
    only            = ["amazon-ebs.linux"]
    script          = "${abspath(path.root)}/../guest/setup-linux.sh"
    execute_command = "sudo bash '{{ .Path }}'"
    timeout         = "90m"
  }
  provisioner "shell" {
    only              = ["amazon-ebs.linux"]
    inline            = ["sudo reboot"]
    expect_disconnect = true
  }
  provisioner "shell" {
    only            = ["amazon-ebs.linux"]
    pause_before    = "30s"
    script          = "${abspath(path.root)}/../guest/finish-linux.sh"
    execute_command = "sudo bash '{{ .Path }}'"
  }
  provisioner "powershell" {
    only    = ["amazon-ebs.windows"]
    script  = "${abspath(path.root)}/../guest/setup-windows.ps1"
    timeout = "90m"
  }
  provisioner "windows-restart" {
    only            = ["amazon-ebs.windows"]
    restart_timeout = "20m"
  }
  provisioner "powershell" {
    only   = ["amazon-ebs.windows"]
    script = "${abspath(path.root)}/../guest/finish-windows.ps1"
  }
  provisioner "powershell" {
    only = ["amazon-ebs.windows"]
    inline = [
      "& 'C:/Program Files/Amazon/EC2Launch/ec2launch.exe' reset --block",
      "if ($LASTEXITCODE -ne 0) { throw 'EC2Launch reset failed' }",
      "& 'C:/Program Files/Amazon/EC2Launch/ec2launch.exe' sysprep --shutdown --block"
    ]
  }
  post-processor "manifest" {
    output     = "${abspath(path.root)}/../packer-manifest.json"
    strip_path = true
    custom_data = {
      recipe_sha256 = local.recipe_sha256
      source_ami    = var.source_ami
      guest_os      = var.guest_os
      account_id    = var.account_id
      lab_name      = var.lab_name
    }
  }
}
