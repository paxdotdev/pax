variable "account_id" {
  type = string
  validation {
    condition     = can(regex("^[0-9]{12}$", var.account_id))
    error_message = "Supply the approved AWS account ID."
  }
}
variable "approval_reference" {
  type = string
  validation {
    condition     = length(trimspace(var.approval_reference)) > 0 && var.approval_reference != "APPROVAL_REQUIRED"
    error_message = "Record explicit execution approval; drafting is not approval to apply."
  }
}
variable "lab_name" {
  type    = string
  default = "pax1038-gpu"
  validation {
    condition     = can(regex("^[a-z][a-z0-9-]{2,23}$", var.lab_name))
    error_message = "Use a short lowercase lab name."
  }
}
variable "artifact_bucket_name" {
  type        = string
  description = "New private bucket for inputs/ and runs/; contents block destroy."
}
variable "availability_zone" {
  type        = string
  description = "Explicit us-west-2 AZ, checked for g4dn capacity before approval."
  validation {
    condition     = can(regex("^us-west-2[a-z]$", var.availability_zone))
    error_message = "This draft targets Oregon only."
  }
}
variable "builder_ipv4_cidr" {
  type        = string
  default     = null
  nullable    = true
  description = "Operator public IPv4 /32 for Packer SSH/WinRM; null closes builder ingress."
  validation {
    condition     = var.builder_ipv4_cidr == null ? true : can(cidrnetmask(var.builder_ipv4_cidr)) && endswith(var.builder_ipv4_cidr, "/32")
    error_message = "Supply a single IPv4 /32, never a broad inbound range."
  }
}
variable "dcv_ipv4_cidr" {
  type        = string
  default     = null
  nullable    = true
  description = "Optional /32 for direct DCV TCP+QUIC; default uses an SSM TCP tunnel."
  validation {
    condition     = var.dcv_ipv4_cidr == null ? true : can(cidrnetmask(var.dcv_ipv4_cidr)) && endswith(var.dcv_ipv4_cidr, "/32")
    error_message = "Supply a single IPv4 /32."
  }
}
variable "machine" {
  type = object({
    os            = string
    ami_id        = string
    image_source  = optional(string, "baked")
    recipe_sha256 = optional(string)
    expires_at    = string
  })
  default     = null
  nullable    = true
  description = "At most one test VM. Choose a lab-baked AMI or an exact reviewed NVIDIA vendor AMI. null creates only infrastructure."
  validation {
    condition = var.machine == null ? true : (
      contains(["windows", "linux"], var.machine.os) &&
      can(regex("^ami-[0-9a-f]{17}$", var.machine.ami_id)) &&
      contains(["baked", "vendor"], var.machine.image_source) &&
      (var.machine.image_source == "vendor" ? var.machine.recipe_sha256 == null : can(regex("^[0-9a-f]{64}$", var.machine.recipe_sha256))) &&
      can(regex("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$", var.machine.expires_at))
    )
    error_message = "Supply OS, explicit AMI, source (baked/vendor), and UTC expiry; recipe SHA256 is required only for baked images."
  }
}
