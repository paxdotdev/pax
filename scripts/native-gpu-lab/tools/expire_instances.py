"""AWS-only expiry guard. Importing this module never contacts AWS.

Stop, never terminate, only this lab's tagged running instances. Missing or
invalid deadlines fail closed. AMIs, disks, snapshots, and evidence are untouched.
"""

from datetime import datetime, timedelta, timezone
import os


def stop_reason(instance, lab_name, now):
    tags = {item["Key"]: item["Value"] for item in instance.get("Tags", [])}
    if tags.get("PaxGpuLab") != lab_name or tags.get("ManagedBy") != "pax-gpu-lab":
        return None
    if instance.get("State", {}).get("Name") != "running":
        return None
    try:
        deadline = datetime.strptime(tags["PaxExpiresAt"], "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
        launch = instance["LaunchTime"]
        if launch.tzinfo is None:
            raise ValueError("LaunchTime lacks timezone")
    except (KeyError, ValueError, TypeError, AttributeError):
        return "missing or invalid deadline"
    # The absolute deadline survives restarts; LaunchTime also caps each run.
    if now >= min(deadline, launch + timedelta(hours=4)):
        return "expired"
    return None


def expire(client, lab_name, now):
    stopped, failures = [], []
    pages = client.get_paginator("describe_instances").paginate(Filters=[
        {"Name": "tag:PaxGpuLab", "Values": [lab_name]},
        {"Name": "tag:ManagedBy", "Values": ["pax-gpu-lab"]},
        {"Name": "instance-state-name", "Values": ["running"]},
    ])
    for page in pages:
        for reservation in page.get("Reservations", []):
            for instance in reservation.get("Instances", []):
                reason = stop_reason(instance, lab_name, now)
                if reason is None:
                    continue
                instance_id = instance["InstanceId"]
                try:
                    client.stop_instances(InstanceIds=[instance_id])
                    stopped.append(instance_id)
                    print(f"Stop requested: {instance_id}: {reason}")
                except Exception as error:
                    # One transient failure must not skip the remaining guests.
                    failures.append(instance_id)
                    print(f"Stop failed: {instance_id}: {type(error).__name__}")
    if failures:
        raise RuntimeError(f"Could not stop {len(failures)} expired instance(s)")
    return {"stop_requested": stopped}


def handler(event, context):
    import boto3
    result = expire(boto3.client("ec2"), os.environ["LAB_NAME"], datetime.now(timezone.utc))
    # A launch preflight requires a recent successful invocation after deployment.
    print("pax_expiry_ok")
    return result
