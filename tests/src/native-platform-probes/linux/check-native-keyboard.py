#!/usr/bin/env python3
"""Actual X11 canvas/native keyboard contract against Rust fixture readouts."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import time
from native_process import NativeProcess, add_launch_arguments


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("app", type=Path)
    add_launch_arguments(parser)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = output / "state.json"
    checks = []
    other = None
    log = (output / "app.log").open("w")
    app = NativeProcess(args, dict(os.environ, GSK_RENDERER="ngl", PAX_NATIVE_TEST_REPORT=str(report)), log, output)

    def check(condition, name):
        checks.append({"check": name, "passed": bool(condition)})
        if not condition:
            raise RuntimeError(name)
        print("PASS", name, flush=True)

    def state():
        if app.poll() is not None:
            raise RuntimeError(f"App exited {app.returncode}")
        try:
            return json.loads(report.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def texts(s):
        return [n.get("text", "") for n in s.get("nodes", []) if n["kind"] == "Text"]

    def counts(s):
        for text in texts(s):
            m = re.fullmatch(r"Keys: down=(\d+) up=(\d+) press=(\d+) repeats=(\d+)", text)
            if m:
                return tuple(map(int, m.groups()))
        return None

    def wait(predicate, name, seconds=15):
        deadline = time.monotonic()+seconds
        while time.monotonic() < deadline:
            s = state()
            if predicate(s):
                return s
            time.sleep(.05)
        raise RuntimeError("Timeout: "+name)

    def xdo(*args):
        return subprocess.check_output(["xdotool", *map(str, args)], text=True, timeout=10).strip()

    def expected_key(key, increment, name):
        before = counts(state())
        xdo("key", key)
        s = wait(lambda s: counts(s) and counts(s)[1] >= before[1]+increment[1], name)
        check(tuple(counts(s)[i]-before[i] for i in range(3)) == increment, name)
        return s

    def canvas_focus():
        xdo("mousemove", "--window", window, 10, 10)
        xdo("click", 1)
        time.sleep(.3)

    try:
        wait(lambda s: counts(s) is not None, "fixture keyboard readout")
        window = app.x11_window()
        xdo("windowfocus", "--sync", window)
        s = expected_key("a", (1, 1, 1), "standalone canvas initially receives KeyDown/KeyUp/KeyPress")
        check("KeyPress: key=a mods= repeat=false" in texts(s), "printable key and repeat flag reach Rust")
        expected_key("q", (1, 1, 0), "Rust KeyDown cancellation suppresses printable KeyPress")
        expected_key("Escape", (1, 1, 0), "Escape is normalized and cancellable")
        expected_key("Left", (1, 1, 0), "arrow key does not emit legacy KeyPress")
        expected_key("Return", (1, 1, 1), "Enter emits legacy KeyPress")
        before = counts(state())
        xdo("keydown", "ctrl")
        xdo("keydown", "a")
        s = wait(lambda s: "KeyDown: key=a mods=Control repeat=false" in texts(s), "Control modifier")
        check(counts(s)[2] == before[2], "Control shortcut suppresses KeyPress")
        xdo("keyup", "a", "ctrl")
        wait(lambda s: counts(s)[1] == before[1]+2, "Control releases")
        before = counts(state())
        xdo("keydown", "b")
        s = wait(lambda s: counts(s)[3] > before[3], "native X11 repeat", seconds=5)
        check(any(t == "KeyDown: key=b mods= repeat=true" for t in texts(s)), "held physical key emits native repeat flag")
        xdo("keyup", "b")
        wait(lambda s: counts(s)[1] == before[1]+1, "repeat key release")
        s = state()
        editor = min((n for n in s["nodes"] if n["kind"] == "Textbox"), key=lambda n:n["bounds"][1])
        x, y, w, h = editor["bounds"]
        before = counts(s)
        xdo("keydown", "c")
        wait(lambda s: counts(s)[0] > before[0], "held key before native focus")
        xdo("mousemove", "--window", window, round(x+w*.2), round(y+h*.5))
        xdo("click", 1)
        s = wait(lambda s: counts(s)[1] == before[1]+1, "focus transition releases held key")
        check("KeyUp: key=c mods= repeat=false" in texts(s), "moving focus to native editor releases held canvas key")
        xdo("keyup", "c")
        before = counts(state())
        xdo("key", "ctrl+a")
        xdo("type", "--clearmodifiers", "native q")
        wait(lambda s: "Rust received: native q" in texts(s), "native editor binding")
        check(counts(state()) == before, "native editing and shortcuts do not dispatch global canvas keys")
        canvas_focus()
        expected_key("d", (1, 1, 1), "clicking vector content restores canvas focus")
        before = counts(state())
        xdo("keydown", "ctrl")
        wait(lambda s: counts(s)[0] == before[0]+1, "Control held before deactivation")
        other = subprocess.Popen(["xmessage", "-title", "Pax keyboard focus probe", "-timeout", "20",
                                  "-buttons", "OK:0", "Native focus qualification"],
                                 stdout=log, stderr=log)
        other_window = xdo("search", "--sync", "--name", "Pax keyboard focus probe").splitlines()[-1]
        xdo("windowfocus", "--sync", other_window)
        s = wait(lambda s: counts(s)[1] == before[1]+1, "window deactivation releases held key")
        check("KeyUp: key=Control mods= repeat=false" in texts(s), "deactivation releases held modifiers with unknown-state modifiers cleared")
        xdo("keyup", "ctrl")
        other.terminate()
        other.wait(timeout=5)
        other = None
        xdo("windowfocus", "--sync", window)
        canvas_focus()
        expected_key("Tab", (1, 1, 0), "native Tab focus transition pairs canvas key release")
        app.terminate()
        check(app.wait(timeout=20) == 0, "keyboard fixture closes cleanly")
    finally:
        if other is not None and other.poll() is None:
            other.terminate()
            other.wait(timeout=5)
        for key in ("a", "b", "c", "ctrl"):
            subprocess.run(["xdotool", "keyup", key], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if app.poll() is None:
            app.terminate()
            try:
                app.wait(timeout=20)
            except subprocess.TimeoutExpired:
                app.kill()
                app.wait()
        log.close()
        (output / "checks.json").write_text(json.dumps({"checks": checks}, indent=2)+"\n")


if __name__ == "__main__":
    main()
