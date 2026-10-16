#!/usr/bin/env python3
"""Check the live lab before Packer; build only with an explicit --execute flag.

The caller must already have approval. This guard verifies technical prerequisites;
an approval string is a run record, not a substitute for human authorization.
"""

import argparse
from datetime import datetime, timedelta, timezone
import json
from pathlib import Path
import subprocess

from check_inputs import validate
from check_sources import check_sources


def validate_request(values, now):
    approval = values.get("approval_reference", "")
    if not approval.strip() or any(x in approval.upper() for x in ("REQUIRED", "PREFLIGHT", "PREVIEW")):
        raise ValueError("Execution approval has not been recorded")
    deadline = datetime.strptime(values["expires_at"], "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    if not now + timedelta(minutes=30) < deadline <= now + timedelta(hours=4):
        raise ValueError("Build deadline must be more than 30 minutes and at most four hours away")
    manifest = Path(values["inputs_file"])
    if not manifest.is_absolute():
        raise ValueError("inputs_file must be an absolute path")
    validate(json.loads(manifest.read_text()), values["guest_os"])


def owned(resource, lab):
    tags = resource.get("Tags", [])
    if isinstance(tags, list):
        tags = {t["Key"]: t["Value"] for t in tags}
    return tags.get("PaxGpuLab") == lab and tags.get("ManagedBy") == "pax-gpu-lab"


def check_lab(values, aws, now):
    account, lab = values["account_id"], values.get("lab_name", "pax1038-gpu")
    if aws("sts", "get-caller-identity")["Account"] != account:
        raise ValueError("AWS profile account differs from approved account")
    reservations = aws("ec2", "describe-instances", "--filters",
                       f"Name=tag:PaxGpuLab,Values={lab}",
                       "Name=tag:ManagedBy,Values=pax-gpu-lab",
                       "Name=instance-state-name,Values=pending,running,stopping,stopped")["Reservations"]
    if any(r.get("Instances") for r in reservations):
        raise ValueError("An owned guest already exists; finish/clean up before another build")
    vpc = aws("ec2", "describe-vpcs", "--vpc-ids", values["vpc_id"])["Vpcs"][0]
    subnet = aws("ec2", "describe-subnets", "--subnet-ids", values["subnet_id"])["Subnets"][0]
    group = aws("ec2", "describe-security-groups", "--group-ids", values["security_group_id"])["SecurityGroups"][0]
    for resource in (vpc, subnet, group):
        if not owned(resource, lab) or resource["OwnerId"] != account:
            raise ValueError("Network ownership mismatch")
    if subnet["VpcId"] != vpc["VpcId"] or group["VpcId"] != vpc["VpcId"]:
        raise ValueError("Builder network does not share the selected VPC")
    port = 22 if values["guest_os"] == "linux" else 5986
    permissions = group.get("IpPermissions", [])
    if not permissions or any(p.get("IpProtocol") != "tcp" or p.get("FromPort") != p.get("ToPort")
                             or p.get("FromPort") not in (22, 5986) or p.get("Ipv6Ranges")
                             or p.get("UserIdGroupPairs") or p.get("PrefixListIds")
                             or len(p.get("IpRanges", [])) != 1
                             or not p["IpRanges"][0]["CidrIp"].endswith("/32") for p in permissions):
        raise ValueError("Builder ingress must contain only single-client SSH/WinRM rules")
    if not any(p["FromPort"] == port for p in permissions):
        raise ValueError("Required builder port is closed")
    profile = aws("iam", "get-instance-profile", "--instance-profile-name", values["iam_instance_profile"])["InstanceProfile"]
    if not owned(profile, lab) or profile["Arn"].split(":")[4] != account:
        raise ValueError("Guest instance profile ownership mismatch")
    fn = aws("lambda", "get-function", "--function-name", f"{lab}-expiry")
    config = fn["Configuration"]
    if not owned(fn, lab) or config["State"] != "Active" or config["LastUpdateStatus"] != "Successful":
        raise ValueError("Expiry function is not healthy and owned")
    schedule = aws("scheduler", "get-schedule", "--group-name", f"{lab}-expiry", "--name", "stop-expired-guests")
    if schedule["State"] != "ENABLED" or schedule["ScheduleExpression"] != "rate(5 minutes)" or schedule["Target"]["Arn"] != config["FunctionArn"]:
        raise ValueError("Expiry schedule is missing or targets the wrong function")
    deployed = datetime.fromisoformat(config["LastModified"].replace("Z", "+00:00"))
    since = max(deployed, now - timedelta(minutes=10))
    events = aws("logs", "filter-log-events", "--log-group-name", f"/aws/lambda/{lab}-expiry",
                 "--start-time", str(int(since.timestamp() * 1000)), "--filter-pattern", '"pax_expiry_ok"')
    if not events.get("events"):
        raise ValueError("Expiry function has no successful invocation in the last ten minutes after deployment")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("variables", type=Path, help="JSON .pkrvars.json, never a shell script")
    parser.add_argument("--aws", default="aws")
    parser.add_argument("--packer", default="packer")
    parser.add_argument("--execute", action="store_true", help="Build after all checks; requires prior human approval")
    args = parser.parse_args()
    values = json.loads(args.variables.read_text())
    now = datetime.now(timezone.utc)
    validate_request(values, now)

    def aws(*command):
        return json.loads(subprocess.check_output(
            [args.aws, "--profile", "pax", "--region", "us-west-2", "--no-cli-pager", "--output", "json", *command], text=True))

    check_lab(values, aws, now)
    check_sources(json.loads(Path(values["inputs_file"]).read_text()), aws)
    print("Live account, idle lab, network ownership, sources and expiry checks passed.")
    if args.execute:
        validate_request(values, datetime.now(timezone.utc))
        subprocess.run([args.packer, "build", "-parallel-builds=1", "-on-error=cleanup",
                        f"-var-file={args.variables.resolve()}",
                        str(Path(__file__).resolve().parents[1] / "packer")], check=True)
    else:
        print("Check only: no image build started.")


if __name__ == "__main__":
    main()
