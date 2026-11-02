#!/usr/bin/env python3
"""Qualify real IBus composition and AT-SPI on the native desktop fixture.

Run in the same D-Bus session as an IBus daemon, with libpinyin selected,
GTK_IM_MODULE=ibus and GTK_A11Y=atspi. Uses actual X11 keyboard/mouse input;
AT-SPI accesses the live GTK widget tree, never the host's diagnostic model.
"""
import argparse
import json
import os
from pathlib import Path
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
    log = (output / "app.log").open("w")
    environment = dict(os.environ, GSK_RENDERER="ngl", PAX_NATIVE_TEST_REPORT=str(report))
    app = NativeProcess(args, environment, log, output)

    def check(condition, name):
        checks.append({"check": name, "passed": bool(condition)})
        if not condition:
            raise RuntimeError(name)
        print("PASS", name, flush=True)

    def state():
        if app.poll() is not None:
            raise RuntimeError(f"Application exited {app.returncode}")
        try:
            return json.loads(report.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def wait(predicate, name, seconds=15):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = state()
            if predicate(value):
                return value
            time.sleep(.05)
        raise RuntimeError("Timeout: " + name)

    def node(value, kind, text=None):
        return next((n for n in value.get("nodes", []) if n["kind"] == kind
                     and (text is None or n.get("text") == text)), None)

    def xdo(*args):
        return subprocess.check_output(["xdotool", *map(str, args)], text=True).strip()

    try:
        import pyatspi
        value = wait(lambda s: node(s, "Button", "Increment"), "native startup")
        window = app.x11_window()
        xdo("windowfocus", "--sync", window)
        editor = min((n for n in value["nodes"] if n["kind"] == "Textbox"), key=lambda n: n["bounds"][1])
        editor_id = editor["id"]
        x, y, w, h = editor["bounds"]
        xdo("mousemove", "--window", window, round(x+w*.2), round(y+h*.5))
        xdo("click", 1)
        xdo("key", "ctrl+a", "BackSpace")
        wait(lambda s: node(s, "Text", "Rust received: "), "clear editor")
        engine = subprocess.check_output(["ibus", "engine"], text=True).strip()
        check(engine == "libpinyin", "real IBus libpinyin engine selected")
        xdo("type", "--clearmodifiers", "--delay", 120, "nihao")
        time.sleep(.75)
        value = state()
        current = next(n for n in value["nodes"] if n["id"] == editor_id)
        check(current["text"] == "", "uncommitted IBus preedit does not leak into Rust bound text")
        xdo("key", "space")
        value = wait(lambda s: node(s, "Text", "Rust received: 你好"), "IBus candidate commit")
        check(True, "actual CJK candidate commit reaches Rust through native GTK input")
        xdo("type", "--clearmodifiers", "--delay", 120, "zhong")
        xdo("key", "Escape")
        time.sleep(.5)
        check(node(state(), "Text", "Rust received: 你好") is not None,
              "cancelling IBus preedit preserves committed bound text")
        xdo("key", "ctrl+a")
        time.sleep(.4)
        desktop = pyatspi.Registry.getDesktop(0)
        native_app = None
        deadline = time.monotonic() + 10
        while native_app is None and time.monotonic() < deadline:
            for candidate in desktop:
                if app.matches_accessible(candidate):
                    native_app = candidate
                    break
            time.sleep(.1)
        check(native_app is not None, "external AT-SPI client discovers the actual application")
        tree = []
        controls = []

        def visit(accessible, depth=0):
            if depth > 24 or len(tree) >= 512:
                return
            record = {"role": accessible.getRoleName(), "name": accessible.name, "depth": depth}
            try:
                text = accessible.queryText()
                record["characters"] = text.characterCount
                record["text"] = text.getText(0, text.characterCount)
                record["selections"] = [list(text.getSelection(i)) for i in range(text.getNSelections())]
            except NotImplementedError:
                pass
            tree.append(record)
            controls.append((accessible, record))
            for child in accessible:
                visit(child, depth+1)

        visit(native_app)
        (output / "atspi-tree.json").write_text(json.dumps(tree, ensure_ascii=False, indent=2)+"\n")
        entries = [(a, r) for a, r in controls if r.get("text") == "你好" and r["role"] in ("text", "entry")]
        check(bool(entries), "AT-SPI exposes actual committed Unicode editor text")
        check(any([0, 2] in r.get("selections", []) for _, r in entries),
              "AT-SPI exposes native Unicode selection bounds")
        button = next((a for a, r in controls if r["role"] in ("push button", "button") and r["name"] == "Increment"), None)
        check(button is not None, "AT-SPI exposes named native button role")
        action = button.queryAction()
        check(action.nActions > 0 and action.doAction(0), "AT-SPI invokes native button action")
        wait(lambda s: node(s, "Text", "Count: 1"), "accessible action Rust handler")
        check(True, "accessible button action reaches Rust")
        # GTK 4.14's AT-SPI Component.grabFocus is unavailable here. Exercise
        # actual toolkit keyboard traversal and observe its exported focus state.
        for _ in range(20):
            if button.getState().contains(pyatspi.STATE_FOCUSED):
                break
            xdo("key", "Tab")
            time.sleep(.1)
        check(button.getState().contains(pyatspi.STATE_FOCUSED),
              "native Tab traversal focuses the button and AT-SPI exposes focus")
        xdo("key", "space")
        wait(lambda s: node(s, "Text", "Count: 2"), "keyboard button activation")
        check(True, "Space activates the focused native button and Rust handler")
        app.terminate()
        check(app.wait(timeout=20) == 0, "input/accessibility session closes cleanly")
    finally:
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
