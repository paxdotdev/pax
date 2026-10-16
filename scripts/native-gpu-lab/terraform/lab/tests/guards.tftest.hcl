mock_provider "aws" {}
mock_provider "archive" {}

variables {
  account_id           = "123456789012"
  approval_reference   = "offline-test-only"
  artifact_bucket_name = "pax-offline-test-artifacts"
  availability_zone    = "us-west-2a"
}

run "foundation_has_no_guest_or_runtime_ingress" {
  command = plan
  assert {
    condition     = length(aws_instance.machine) == 0 && length(aws_vpc_security_group_ingress_rule.dcv) == 0 && length(aws_vpc_security_group_ingress_rule.builder) == 0
    error_message = "The default foundation must not launch a VM or open inbound access."
  }
}

run "reject_broad_builder_ingress" {
  command = plan
  variables {
    builder_ipv4_cidr = "0.0.0.0/0"
  }
  expect_failures = [var.builder_ipv4_cidr]
}

run "reject_broad_runtime_ingress" {
  command = plan
  variables {
    dcv_ipv4_cidr = "10.0.0.0/8"
  }
  expect_failures = [var.dcv_ipv4_cidr]
}

run "reject_expired_guest" {
  command = plan
  variables {
    machine = {
      os            = "windows"
      ami_id        = "ami-00000000000000000"
      recipe_sha256 = "abababababababababababababababababababababababababababababababab"
      expires_at    = "2020-01-01T00:00:00Z"
    }
  }
  expect_failures = [aws_instance.machine]
}

run "bounded_windows_guest" {
  command = plan
  variables {
    machine = {
      os            = "windows"
      ami_id        = "ami-00000000000000000"
      recipe_sha256 = "abababababababababababababababababababababababababababababababab"
      expires_at    = timeadd(timestamp(), "2h")
    }
  }
  assert {
    condition     = length(aws_instance.machine) == 1 && aws_instance.machine[0].instance_type == "g4dn.xlarge" && aws_instance.machine[0].root_block_device[0].encrypted && aws_instance.machine[0].metadata_options[0].http_tokens == "required"
    error_message = "A guest must use the bounded GPU type, encrypted root disk and IMDSv2."
  }
}

run "pinned_vendor_windows_guest" {
  command = plan
  variables {
    machine = {
      os           = "windows"
      image_source = "vendor"
      ami_id       = "ami-0421bb7b41b9c7828"
      expires_at   = timeadd(timestamp(), "2h")
    }
  }
  assert {
    condition     = length(aws_instance.machine) == 1 && aws_instance.machine[0].tags.PaxImageSource == "vendor" && aws_instance.machine[0].root_block_device[0].encrypted
    error_message = "A vendor image must preserve the same bounded, encrypted runtime contract."
  }
}

run "reject_unreviewed_vendor_image" {
  command = plan
  variables {
    machine = {
      os           = "windows"
      image_source = "vendor"
      ami_id       = "ami-00000000000000000"
      expires_at   = timeadd(timestamp(), "2h")
    }
  }
  expect_failures = [data.aws_ami.machine]
}
