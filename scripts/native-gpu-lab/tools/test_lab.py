"""Offline tests for cost-control ownership/deadlines and input admission."""

import copy
from datetime import datetime, timedelta, timezone
import json
from pathlib import Path
import unittest
from unittest.mock import MagicMock

from check_inputs import validate
from check_sources import check_sources
from build_image import check_lab, owned, validate_request
from expire_instances import expire, stop_reason

NOW = datetime(2026, 10, 15, 12, tzinfo=timezone.utc)


def instance(deadline="2026-10-15T12:00:00Z", age=1):
    return {
        "InstanceId": "i-test",
        "State": {"Name": "running"},
        "LaunchTime": NOW - timedelta(hours=age),
        "Tags": [
            {"Key": "PaxGpuLab", "Value": "test-lab"},
            {"Key": "ManagedBy", "Value": "pax-gpu-lab"},
            {"Key": "PaxExpiresAt", "Value": deadline},
        ],
    }


class ExpiryTests(unittest.TestCase):
    def test_live_instance_is_untouched(self):
        self.assertIsNone(stop_reason(instance("2026-10-15T13:00:00Z"), "test-lab", NOW))

    def test_deadline_is_inclusive(self):
        self.assertEqual(stop_reason(instance(), "test-lab", NOW), "expired")

    def test_other_lab_and_untagged_instances_are_untouched(self):
        self.assertIsNone(stop_reason(instance(), "other-lab", NOW))
        item = instance()
        item["Tags"] = item["Tags"][1:]
        self.assertIsNone(stop_reason(item, "test-lab", NOW))

    def test_ownership_requires_both_tags(self):
        item = instance()
        item["Tags"][1]["Value"] = "unrelated-tool"
        self.assertIsNone(stop_reason(item, "test-lab", NOW))

    def test_invalid_deadline_fails_closed_only_for_owned_instances(self):
        for deadline in ["", "tomorrow", "2026-10-15T14:00:00+02:00"]:
            self.assertEqual(stop_reason(instance(deadline), "test-lab", NOW), "missing or invalid deadline")
        item = instance()
        item["Tags"].pop()
        self.assertEqual(stop_reason(item, "test-lab", NOW), "missing or invalid deadline")

    def test_far_future_deadline_cannot_bypass_four_hour_limit(self):
        self.assertEqual(stop_reason(instance("2030-01-01T00:00:00Z", age=4), "test-lab", NOW), "expired")

    def test_restart_does_not_extend_absolute_deadline(self):
        self.assertEqual(stop_reason(instance(age=0), "test-lab", NOW), "expired")

    def test_stopped_instances_are_not_modified(self):
        item = instance()
        item["State"]["Name"] = "stopped"
        self.assertIsNone(stop_reason(item, "test-lab", NOW))

    def test_pagination_and_failure_isolation(self):
        first, second, third = instance(), instance(), instance()
        second["InstanceId"], third["InstanceId"] = "i-other", "i-third"
        second["Tags"][0]["Value"] = "other-lab"

        class FakeEC2:
            def __init__(self):
                self.calls = []

            def get_paginator(self, name):
                assert name == "describe_instances"
                return self

            def paginate(self, **kwargs):
                assert len(kwargs["Filters"]) == 3
                yield {"Reservations": [{"Instances": [first, second]}]}
                yield {"Reservations": [{"Instances": [third]}]}

            def stop_instances(self, InstanceIds):
                self.calls += InstanceIds
                if InstanceIds == ["i-test"]:
                    raise RuntimeError("simulated AWS failure")

        fake = FakeEC2()
        with self.assertRaises(RuntimeError):
            expire(fake, "test-lab", NOW)
        self.assertEqual(fake.calls, ["i-test", "i-third"])


class InputTests(unittest.TestCase):
    def setUp(self):
        self.data = {
            "schema": 1, "guest_os": "linux", "rust_version": "1.96.0",
            "ubuntu_snapshot": "20261014T000000Z", "kernel_release": "6.8.0-1000-aws",
            "artifacts": {name: {
                "version": "1.2.3", "uri": f"https://example.com/1.2.3/{name}", "sha256": "ab" * 32,
            } for name in ["rustup", "grid", "dcv"]},
        }

    def test_complete_manifest(self):
        self.assertIs(validate(self.data, "linux"), self.data)

    def test_kernel_can_be_inherited_from_the_explicit_ami(self):
        self.data["kernel_release"] = "base-ami"
        self.assertIs(validate(self.data, "linux"), self.data)

    def test_examples_cannot_be_used_as_ready_inputs(self):
        root = Path(__file__).resolve().parents[1]
        for guest in ["linux", "windows"]:
            with self.assertRaises(ValueError):
                validate(json.loads((root / "examples" / f"{guest}.inputs.example.json").read_text()), guest)

    def test_rejects_moving_credentials_and_missing_hashes(self):
        for key, value in [
            ("uri", "https://example.com/latest/driver"),
            ("uri", "https://user:password@example.com/driver"),
            ("uri", "https://example.com/driver?token=secret"),
            ("uri", "file:///tmp/driver"),
            ("sha256", "0" * 64), ("sha256", "REVIEW_REQUIRED"),
            ("version", "latest"),
        ]:
            data = copy.deepcopy(self.data)
            data["artifacts"]["grid"][key] = value
            with self.assertRaises(ValueError, msg=(key, value)):
                validate(data, "linux")

    def test_kernel_and_os_must_be_explicit(self):
        with self.assertRaises(ValueError):
            validate(self.data, "windows")
        self.data["kernel_release"] = "latest"
        with self.assertRaises(ValueError):
            validate(self.data, "linux")


class SourceAvailabilityTests(unittest.TestCase):
    def setUp(self):
        self.data = {
            "schema": 1, "guest_os": "windows", "rust_version": "1.96.0",
            "artifacts": {name: {
                "version": "1.2.3", "uri": f"https://example.com/1.2.3/{name}", "sha256": "ab" * 32,
            } for name in ("rustup", "grid", "dcv", "vs_layout", "git", "vc_redist", "winappsdk")},
        }
        self.data["artifacts"]["vs_layout"]["uri"] = "s3://example/inputs/verified/vs-layout.zip"
        self.aws = MagicMock(return_value={"ContentLength": 123, "Metadata": {"sha256": "ab" * 32}})
        self.http = MagicMock()
        response = self.http.return_value.__enter__.return_value
        response.url = "https://example.com/download"
        response.headers = {"Content-Length": "123"}

    def test_uploaded_archive_is_required(self):
        self.aws.side_effect = RuntimeError("NoSuchKey")
        with self.assertRaisesRegex(RuntimeError, "NoSuchKey"):
            check_sources(self.data, self.aws, self.http)

    def test_archive_digest_metadata_must_match(self):
        self.aws.return_value["Metadata"]["sha256"] = "cd" * 32
        with self.assertRaisesRegex(ValueError, "SHA256 metadata"):
            check_sources(self.data, self.aws, self.http)

    def test_available_sources(self):
        check_sources(self.data, self.aws, self.http)
        self.assertEqual(self.aws.call_args.args[:2], ("s3api", "head-object"))

    def test_vendor_runtime_needs_no_vs_archive_or_driver_download(self):
        check_sources(self.data, self.aws, self.http, windows_runtime_only=True)
        self.aws.assert_not_called()
        self.assertEqual({call.args[0].full_url.rsplit("/", 1)[-1] for call in self.http.call_args_list},
                         {"vc_redist", "dcv", "winappsdk"})

    def test_snapshot_outage_stops_before_artifact_checks(self):
        self.data.update(guest_os="linux", ubuntu_snapshot="20261014T000000Z", kernel_release="base-ami")
        self.data["artifacts"] = {k: v for k, v in self.data["artifacts"].items() if k in ("rustup", "grid", "dcv")}
        self.http.side_effect = OSError("502 Bad Gateway")
        with self.assertRaisesRegex(ValueError, "do not launch"):
            check_sources(self.data, self.aws, self.http)
        self.aws.assert_not_called()


class BuildGuardTests(unittest.TestCase):
    def setUp(self):
        self.values = {
            "account_id": "123456789012", "lab_name": "test-lab", "guest_os": "linux",
            "vpc_id": "vpc-test", "subnet_id": "subnet-test", "security_group_id": "sg-test",
            "iam_instance_profile": "test-profile",
        }
        tags = {"PaxGpuLab": "test-lab", "ManagedBy": "pax-gpu-lab"}
        resource = {"Tags": tags, "OwnerId": "123456789012", "VpcId": "vpc-test"}
        arn = "arn:aws:lambda:us-west-2:123456789012:function:test-lab-expiry"
        self.responses = {
            ("sts", "get-caller-identity"): {"Account": "123456789012"},
            ("ec2", "describe-instances"): {"Reservations": []},
            ("ec2", "describe-vpcs"): {"Vpcs": [resource]},
            ("ec2", "describe-subnets"): {"Subnets": [resource]},
            ("ec2", "describe-security-groups"): {"SecurityGroups": [dict(resource, IpPermissions=[
                {"IpProtocol": "tcp", "FromPort": 22, "ToPort": 22, "IpRanges": [{"CidrIp": "192.0.2.1/32"}]}])]},
            ("iam", "get-instance-profile"): {"InstanceProfile": {"Tags": tags, "Arn": "arn:aws:iam::123456789012:instance-profile/test"}},
            ("lambda", "get-function"): {"Tags": tags, "Configuration": {
                "State": "Active", "LastUpdateStatus": "Successful", "FunctionArn": arn,
                "LastModified": "2026-10-15T11:59:00.000+0000"}},
            ("scheduler", "get-schedule"): {"State": "ENABLED", "ScheduleExpression": "rate(5 minutes)", "Target": {"Arn": arn}},
            ("logs", "filter-log-events"): {"events": [{"message": "pax_expiry_ok"}]},
        }
        self.calls = []

    def aws(self, *command):
        self.calls.append(command)
        return self.responses[command[:2]]

    def test_healthy_foundation(self):
        check_lab(self.values, self.aws, NOW)
        log_call = self.calls[-1]
        self.assertEqual(log_call[log_call.index("--start-time") + 1], str(int((NOW - timedelta(minutes=1)).timestamp() * 1000)))

    def test_wrong_account_fails_before_other_calls(self):
        self.responses[("sts", "get-caller-identity")]["Account"] = "999999999999"
        with self.assertRaisesRegex(ValueError, "account"):
            check_lab(self.values, self.aws, NOW)
        self.assertEqual(len(self.calls), 1)

    def test_existing_guests_prevent_another_build(self):
        self.responses[("ec2", "describe-instances")]["Reservations"] = [{"Instances": [{"State": {"Name": "stopped"}}]}]
        with self.assertRaisesRegex(ValueError, "already exists"):
            check_lab(self.values, self.aws, NOW)

    def test_broad_ingress_is_rejected(self):
        self.responses[("ec2", "describe-security-groups")]["SecurityGroups"][0]["IpPermissions"][0]["IpRanges"][0]["CidrIp"] = "0.0.0.0/0"
        with self.assertRaisesRegex(ValueError, "ingress"):
            check_lab(self.values, self.aws, NOW)

    def test_network_with_wrong_ownership_is_rejected(self):
        self.responses[("ec2", "describe-vpcs")]["Vpcs"][0]["OwnerId"] = "999999999999"
        with self.assertRaisesRegex(ValueError, "ownership"):
            check_lab(self.values, self.aws, NOW)

    def test_wrong_expiry_target_is_rejected(self):
        self.responses[("scheduler", "get-schedule")]["Target"]["Arn"] = "another-function"
        with self.assertRaisesRegex(ValueError, "schedule"):
            check_lab(self.values, self.aws, NOW)

    def test_expiry_requires_recent_success(self):
        self.responses[("logs", "filter-log-events")]["events"] = []
        with self.assertRaisesRegex(ValueError, "successful invocation"):
            check_lab(self.values, self.aws, NOW)

    def test_ownership_requires_both_tags(self):
        self.assertFalse(owned({"Tags": {"PaxGpuLab": "test-lab"}}, "test-lab"))

    def test_deadline_and_approval_fail_before_manifest_reads(self):
        for approval in ["", "APPROVAL_REQUIRED", "PREFLIGHT_ONLY_NO_APPLY_APPROVAL"]:
            with self.assertRaisesRegex(ValueError, "approval"):
                validate_request(dict(self.values, approval_reference=approval), NOW)
        for deadline in ["2026-10-15T11:59:00Z", "2026-10-15T12:30:00Z", "2026-10-15T16:00:01Z"]:
            with self.assertRaisesRegex(ValueError, "deadline"):
                validate_request(dict(self.values, approval_reference="approved-in-chat", expires_at=deadline), NOW)


if __name__ == "__main__":
    unittest.main()
