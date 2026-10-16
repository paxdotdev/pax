#!/usr/bin/env python3
"""Verify the actual Debug CLI/template reload path without restarting its GTK app."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("cli", type=Path)
    parser.add_argument("project", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    project = args.project.resolve()
    template = project / "src/lib.pax"
    original = template.read_bytes()
    label = b"Outer native scroller"
    replacement = b"Live Linux reload verified"
    if original.count(label) != 1:
        raise RuntimeError("Fixture must contain exactly one reload label")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = output / "state.json"
    environment = dict(os.environ, GSK_RENDERER="ngl", PAX_NATIVE_TEST_REPORT=str(report))
    checks = []
    app_pid = None
    log = (output / "cli.log").open("w")
    cli = subprocess.Popen([str(args.cli.resolve()), "run", "--path", str(project),
                            "--target", "linux", "--hot-reload=pax"],
                           env=environment, stdout=log, stderr=log, start_new_session=True)

    def wait_label(text, timeout=30):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if cli.poll() is not None:
                raise RuntimeError(f"CLI exited {cli.returncode}; inspect cli.log")
            try:
                state = json.loads(report.read_text())
                if any(n.get("text") == text for n in state.get("nodes", [])):
                    return
            except (FileNotFoundError, json.JSONDecodeError):
                pass
            time.sleep(.1)
        raise RuntimeError("Native reload timeout: " + text)

    def check(condition, name):
        checks.append({"check": name, "passed": bool(condition)})
        if not condition:
            raise RuntimeError(name)
        print("PASS", name, flush=True)

    def xdo(*arguments):
        return subprocess.check_output(["xdotool", *map(str, arguments)], text=True).strip()

    try:
        wait_label(label.decode(), timeout=600)
        window = xdo("search", "--name", "Pax native desktop acceptance").splitlines()[-1]
        app_pid = int(xdo("getwindowpid", window))
        check(app_pid != cli.pid, "CLI launched the native application with its design server")
        template.write_bytes(original.replace(label, replacement))
        wait_label(replacement.decode())
        check(int(xdo("getwindowpid", window)) == app_pid,
              "saved Pax template updates actual native Text without replacing the application")
        template.write_bytes(original)
        wait_label(label.decode())
        check(int(xdo("getwindowpid", window)) == app_pid,
              "restoring the template updates the same native application")
        os.kill(app_pid, signal.SIGTERM)
        check(cli.wait(timeout=30) == 0, "normal native close lets CLI reap its design server")
    finally:
        template.write_bytes(original)
        if cli.poll() is None:
            os.killpg(cli.pid, signal.SIGTERM)
            try:
                cli.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(cli.pid, signal.SIGKILL)
                cli.wait()
        log.close()
        (output / "checks.json").write_text(json.dumps({
            "checks": checks, "appPid": app_pid,
            "templateSha256": hashlib.sha256(original).hexdigest(),
            "restoredSha256": hashlib.sha256(template.read_bytes()).hexdigest(),
        }, indent=2) + "\n")


if __name__ == "__main__":
    main()
