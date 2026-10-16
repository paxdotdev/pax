terraform {
  required_version = ">= 1.10.0, < 2.0.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "6.66.0"
    }
  }
}

variable "account_id" {
  type = string
  validation {
    condition     = can(regex("^[0-9]{12}$", var.account_id))
    error_message = "Supply the approved AWS account ID."
  }
}
variable "state_bucket_name" {
  type        = string
  description = "New, globally unique bucket; never an existing production bucket."
}
variable "approval_reference" {
  type = string
  validation {
    condition     = length(trimspace(var.approval_reference)) > 0 && var.approval_reference != "APPROVAL_REQUIRED"
    error_message = "Record Zack's explicit execution approval before provisioning."
  }
}

provider "aws" {
  profile             = "pax"
  region              = "us-west-2"
  allowed_account_ids = [var.account_id]
  default_tags {
    tags = { Project = "PAX-1038", Purpose = "native-gpu-lab-state" }
  }
}

# Bootstrap uses local state, retained outside Git. It deliberately cannot be
# destroyed together with the lab whose remote state it will hold.
resource "aws_s3_bucket" "state" {
  bucket        = var.state_bucket_name
  force_destroy = false
  tags          = { Approval = var.approval_reference }
  lifecycle {
    prevent_destroy = true
  }
}
resource "aws_s3_bucket_public_access_block" "state" {
  bucket                  = aws_s3_bucket.state.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}
resource "aws_s3_bucket_versioning" "state" {
  bucket = aws_s3_bucket.state.id
  versioning_configuration {
    status = "Enabled"
  }
}
resource "aws_s3_bucket_server_side_encryption_configuration" "state" {
  bucket = aws_s3_bucket.state.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}
resource "aws_s3_bucket_policy" "tls" {
  bucket = aws_s3_bucket.state.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Deny", Principal = "*", Action = "s3:*"
      Resource  = [aws_s3_bucket.state.arn, "${aws_s3_bucket.state.arn}/*"]
      Condition = { Bool = { "aws:SecureTransport" = "false" } }
    }]
  })
}
output "state_bucket" {
  value = aws_s3_bucket.state.id
}
