packer {
  required_version = ">= 1.11.0, < 2.0.0"
  required_plugins {
    amazon = {
      source  = "github.com/hashicorp/amazon"
      version = "1.8.3"
    }
  }
}

variable "account_id" {
  type = string
  validation {
    condition     = can(regex("^[0-9]{12}$", var.account_id))
    error_message = "Supply the approved account ID."
  }
}
variable "approval_reference" {
  type = string
  validation {
    condition     = length(trimspace(var.approval_reference)) > 0 && var.approval_reference != "APPROVAL_REQUIRED"
    error_message = "Image builds require explicit approval, separately from drafting."
  }
}
variable "guest_os" {
  type = string
  validation {
    condition     = contains(["windows", "linux"], var.guest_os)
    error_message = "Choose exactly one OS per invocation."
  }
}
variable "source_ami" {
  type = string
  validation {
    condition     = can(regex("^ami-[0-9a-f]{17}$", var.source_ami))
    error_message = "Pin an official x86_64 base AMI in Oregon."
  }
}
variable "lab_name" {
  type    = string
  default = "pax1038-gpu"
  validation {
    condition     = can(regex("^[a-z][a-z0-9-]{2,23}$", var.lab_name))
    error_message = "Use the Terraform lab name."
  }
}
variable "subnet_id" {
  type = string
}
variable "vpc_id" {
  type = string
}
variable "security_group_id" {
  type = string
}
variable "iam_instance_profile" {
  type = string
}
variable "expires_at" {
  type = string
  validation {
    condition     = can(regex("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$", var.expires_at))
    error_message = "Supply an explicit UTC deadline within four hours; verify the expiry service before building."
  }
}
variable "inputs_file" {
  type        = string
  description = "Absolute path to reviewed JSON inputs. Run tools/check_inputs.py first."
  validation {
    condition     = fileexists(var.inputs_file)
    error_message = "A reviewed input manifest must exist."
  }
}
