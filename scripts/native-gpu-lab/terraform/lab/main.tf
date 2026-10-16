resource "aws_vpc" "lab" {
  cidr_block           = "10.103.0.0/16"
  enable_dns_support   = true
  enable_dns_hostnames = true
  tags                 = { Name = var.lab_name }
}
resource "aws_subnet" "lab" {
  vpc_id                  = aws_vpc.lab.id
  cidr_block              = "10.103.1.0/24"
  availability_zone       = var.availability_zone
  map_public_ip_on_launch = false
  tags                    = { Name = var.lab_name }
}
resource "aws_internet_gateway" "lab" {
  vpc_id = aws_vpc.lab.id
}
resource "aws_route_table" "lab" {
  vpc_id = aws_vpc.lab.id
  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.lab.id
  }
}
resource "aws_route_table_association" "lab" {
  subnet_id      = aws_subnet.lab.id
  route_table_id = aws_route_table.lab.id
}
resource "aws_security_group" "runtime" {
  name_prefix = "${var.lab_name}-runtime-"
  description = "SSM outbound access; optional operator-only DCV"
  vpc_id      = aws_vpc.lab.id
}
resource "aws_security_group" "builder" {
  name_prefix = "${var.lab_name}-builder-"
  description = "Temporary Packer communication from one operator IP"
  vpc_id      = aws_vpc.lab.id
}
resource "aws_vpc_security_group_egress_rule" "outbound" {
  for_each          = { runtime = aws_security_group.runtime.id, builder = aws_security_group.builder.id }
  security_group_id = each.value
  ip_protocol       = "-1"
  cidr_ipv4         = "0.0.0.0/0"
}
resource "aws_vpc_security_group_ingress_rule" "builder" {
  for_each          = var.builder_ipv4_cidr == null ? {} : { ssh = 22, winrm_tls = 5986 }
  security_group_id = aws_security_group.builder.id
  ip_protocol       = "tcp"
  from_port         = each.value
  to_port           = each.value
  cidr_ipv4         = var.builder_ipv4_cidr
}
resource "aws_vpc_security_group_ingress_rule" "dcv" {
  for_each          = var.dcv_ipv4_cidr == null ? toset([]) : toset(["tcp", "udp"])
  security_group_id = aws_security_group.runtime.id
  ip_protocol       = each.value
  from_port         = 8443
  to_port           = 8443
  cidr_ipv4         = var.dcv_ipv4_cidr
}

resource "aws_s3_bucket" "artifacts" {
  bucket        = var.artifact_bucket_name
  force_destroy = false
}
resource "aws_s3_bucket_public_access_block" "artifacts" {
  bucket                  = aws_s3_bucket.artifacts.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}
resource "aws_s3_bucket_versioning" "artifacts" {
  bucket = aws_s3_bucket.artifacts.id
  versioning_configuration {
    status = "Enabled"
  }
}
resource "aws_s3_bucket_server_side_encryption_configuration" "artifacts" {
  bucket = aws_s3_bucket.artifacts.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}
resource "aws_s3_bucket_policy" "artifact_tls" {
  bucket = aws_s3_bucket.artifacts.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Deny", Principal = "*", Action = "s3:*"
      Resource  = [aws_s3_bucket.artifacts.arn, "${aws_s3_bucket.artifacts.arn}/*"]
      Condition = { Bool = { "aws:SecureTransport" = "false" } }
    }]
  })
}

resource "aws_iam_role" "guest" {
  name_prefix = "${var.lab_name}-guest-"
  assume_role_policy = jsonencode({
    Version   = "2012-10-17"
    Statement = [{ Effect = "Allow", Principal = { Service = "ec2.amazonaws.com" }, Action = "sts:AssumeRole" }]
  })
}
resource "aws_iam_role_policy_attachment" "ssm" {
  role       = aws_iam_role.guest.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonSSMManagedInstanceCore"
}
resource "aws_iam_role_policy" "guest_artifacts" {
  role = aws_iam_role.guest.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      { Effect = "Allow", Action = ["s3:GetObject", "s3:GetObjectVersion"], Resource = "${aws_s3_bucket.artifacts.arn}/inputs/*" },
      { Effect = "Allow", Action = ["s3:PutObject"], Resource = "${aws_s3_bucket.artifacts.arn}/runs/*" },
      # Vendor downloads are read-only; manifests still require exact keys/hashes.
      { Effect = "Allow", Action = ["s3:GetObject"], Resource = [
        "arn:aws:s3:::ec2-windows-nvidia-drivers/*", "arn:aws:s3:::ec2-linux-nvidia-drivers/*",
        "arn:aws:s3:::dcv-license.us-west-2/*"
      ] }
    ]
  })
}
resource "aws_iam_instance_profile" "guest" {
  name_prefix = "${var.lab_name}-"
  role        = aws_iam_role.guest.name
}

locals {
  vendor_images = jsondecode(file("${path.module}/../../pins/vendor-images.json"))
  vendor_image  = try(var.machine.image_source == "vendor", false)
}

data "aws_ami" "machine" {
  count  = var.machine == null ? 0 : 1
  owners = [local.vendor_image ? local.vendor_images[var.machine.os].owner_id : var.account_id]
  filter {
    name   = "image-id"
    values = [var.machine.ami_id]
  }
  dynamic "filter" {
    for_each = local.vendor_image ? {
      name         = local.vendor_images[var.machine.os].name
      product-code = local.vendor_images[var.machine.os].product_code
      } : {
      "tag:PaxGpuLab"       = var.lab_name
      "tag:PaxRecipeSHA256" = var.machine.recipe_sha256
      "tag:PaxGuestOS"      = var.machine.os
    }
    content {
      name   = filter.key
      values = [filter.value]
    }
  }
  filter {
    name   = "architecture"
    values = ["x86_64"]
  }
  lifecycle {
    precondition {
      condition     = !local.vendor_image || var.machine.ami_id == local.vendor_images[var.machine.os].ami_id
      error_message = "Vendor AMIs must match the exact reviewed OS-specific pin, not merely a Marketplace publisher account."
    }
  }
}
resource "aws_instance" "machine" {
  count                                = var.machine == null ? 0 : 1
  ami                                  = data.aws_ami.machine[0].id
  instance_type                        = "g4dn.xlarge"
  subnet_id                            = aws_subnet.lab.id
  vpc_security_group_ids               = [aws_security_group.runtime.id]
  associate_public_ip_address          = true
  iam_instance_profile                 = aws_iam_instance_profile.guest.name
  instance_initiated_shutdown_behavior = "stop"
  metadata_options {
    http_endpoint               = "enabled"
    http_tokens                 = "required"
    http_put_response_hop_limit = 1
    instance_metadata_tags      = "disabled"
  }
  root_block_device {
    volume_type           = "gp3"
    volume_size           = var.machine.os == "windows" ? 100 : 60
    encrypted             = true
    delete_on_termination = true
    tags                  = { PaxGpuLab = var.lab_name, ManagedBy = "pax-gpu-lab" }
  }
  tags = {
    Name            = "${var.lab_name}-${var.machine.os}"
    PaxExpiresAt    = var.machine.expires_at
    PaxGuestOS      = var.machine.os
    PaxRecipeSHA256 = var.machine.recipe_sha256
    PaxImageSource  = var.machine.image_source
    PaxSourceAMI    = var.machine.ami_id
    PaxPurpose      = "qualification"
  }
  depends_on = [aws_scheduler_schedule.expiry, aws_route_table_association.lab, aws_iam_role_policy_attachment.ssm]
  lifecycle {
    precondition {
      condition = try(
        timecmp(var.machine.expires_at, plantimestamp()) > 0 &&
        timecmp(var.machine.expires_at, timeadd(plantimestamp(), "4h")) <= 0,
        false
      )
      error_message = "Expiry must be in the next four hours. Regenerate a stale plan; do not extend a run implicitly."
    }
  }
}

output "packer_network" {
  value = {
    vpc_id               = aws_vpc.lab.id
    subnet_id            = aws_subnet.lab.id
    security_group_id    = aws_security_group.builder.id
    iam_instance_profile = aws_iam_instance_profile.guest.name
    expiry_schedule      = aws_scheduler_schedule.expiry.arn
  }
}
output "machine_id" {
  value = one(aws_instance.machine[*].id)
}
output "artifact_bucket" {
  value = aws_s3_bucket.artifacts.id
}
