terraform {
  required_version = ">= 1.10.0, < 2.0.0"
  backend "s3" {}
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "6.66.0"
    }
    archive = {
      source  = "hashicorp/archive"
      version = "2.7.1"
    }
  }
}

provider "aws" {
  profile             = "pax"
  region              = "us-west-2"
  allowed_account_ids = [var.account_id]
  default_tags {
    tags = {
      Project   = "PAX-1038"
      PaxGpuLab = var.lab_name
      ManagedBy = "pax-gpu-lab"
      Approval  = var.approval_reference
    }
  }
}
