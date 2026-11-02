"""Launch native acceptance fixtures, preserving identity across Flatpak namespaces."""
import configparser
import ctypes
import ctypes.util
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time


def add_launch_arguments(parser):
    parser.add_argument("--flatpak-app", help="Installed user Flatpak application ID")
    parser.add_argument("--flatpak-branch", default="pilot")


def _children(pid):
    try:
        return [int(v) for v in Path(f"/proc/{pid}/task/{pid}/children").read_text().split()]
    except FileNotFoundError:
        return []


def _descendants(pid):
    found, pending = set(), [pid]
    while pending:
        current = pending.pop()
        if current not in found:
            found.add(current)
            pending.extend(_children(current))
    return found


def _stop_unidentified_launch(process):
    # A failed identity probe cannot safely signal a guessed sandbox child.
    # Flatpak is launched in its own session so cleanup is limited to this run.
    if process.poll() is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=5)
    except ProcessLookupError:
        process.wait(timeout=5)


def _x11_client_pid(window):
    # XRes reports the server-side host PID when a sandbox omits _NET_WM_PID.
    # Use the public libXRes API; private Xlib request functions can deadlock.
    library = ctypes.util.find_library("XRes")
    if not library:
        raise RuntimeError("Flatpak X11 checks require libXRes (Ubuntu: libxres1)")

    class ClientSpec(ctypes.Structure):
        _fields_ = [("client", ctypes.c_ulong), ("mask", ctypes.c_uint)]

    class ClientValue(ctypes.Structure):
        _fields_ = [("spec", ClientSpec), ("length", ctypes.c_long), ("value", ctypes.c_void_p)]

    x = ctypes.CDLL(ctypes.util.find_library("X11"))
    r = ctypes.CDLL(library)
    x.XOpenDisplay.argtypes = [ctypes.c_char_p]
    x.XOpenDisplay.restype = ctypes.c_void_p
    x.XCloseDisplay.argtypes = [ctypes.c_void_p]
    r.XResQueryVersion.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_int)]
    r.XResQueryClientIds.argtypes = [ctypes.c_void_p, ctypes.c_long, ctypes.POINTER(ClientSpec),
                                    ctypes.POINTER(ctypes.c_long), ctypes.POINTER(ctypes.POINTER(ClientValue))]
    r.XResGetClientPid.argtypes = [ctypes.POINTER(ClientValue)]
    r.XResGetClientPid.restype = ctypes.c_int
    r.XResClientIdsDestroy.argtypes = [ctypes.c_long, ctypes.POINTER(ClientValue)]
    display = x.XOpenDisplay(None)
    if not display:
        raise RuntimeError("Cannot open X11 display for process identity")
    values = ctypes.POINTER(ClientValue)()
    count = ctypes.c_long()
    try:
        major, minor = ctypes.c_int(), ctypes.c_int()
        if not r.XResQueryVersion(display, ctypes.byref(major), ctypes.byref(minor)) or (major.value, minor.value) < (1, 2):
            raise RuntimeError("XRes 1.2 is required for sandbox window identity")
        spec = ClientSpec(int(window), 2)
        status = r.XResQueryClientIds(display, 1, ctypes.byref(spec), ctypes.byref(count), ctypes.byref(values))
        if status != 0 or not values or count.value != 1:
            raise RuntimeError(f"Expected one XRes identity, got {count.value}")
        pid = r.XResGetClientPid(ctypes.byref(values[0]))
        if pid <= 0:
            raise RuntimeError("XRes did not expose the window's host PID")
        return pid
    finally:
        if values:
            r.XResClientIdsDestroy(count, values)
        x.XCloseDisplay(display)


class NativeProcess:
    def __init__(self, args, environment, log, output):
        self.flatpak = bool(args.flatpak_app)
        self.identity = {}
        self.pid = None
        self._expected_executable = None
        if not self.flatpak:
            self.process = subprocess.Popen([str(args.app.resolve())], env=environment, stdout=log, stderr=log)
            self.pid = self.process.pid
            return
        command = ["flatpak", "run", "--user", f"--branch={args.flatpak_branch}",
                   f"--filesystem={output}"]
        # Only the fresh fixture evidence directory is shared; shipping metadata
        # has no filesystem permission. Forward explicit test/runtime variables.
        allowed = {"PAX_RENDER_TIMINGS", "PAX_GPU_DEVICE_UUID", "PAX_GPU_MEMORY_DIAGNOSTICS", "PAX_PROPERTY_MEMORY_DIAGNOSTICS", "PAX_TEXT_LAYOUT_LOG", "PAX_NAVIGATION_LOG", "GDK_BACKEND", "GSK_RENDERER", "GTK_A11Y", "GTK_IM_MODULE", "GTK_USE_PORTAL"}
        for name, value in environment.items():
            if name in allowed or name.startswith("PAX_NATIVE_TEST_"):
                command.append(f"--env={name}={value}")
        with (output / "flatpak-instance").open("w") as instance:
            command += [f"--instance-id-fd={instance.fileno()}", args.flatpak_app]
            self.process = subprocess.Popen(command, env=environment, stdout=log, stderr=log,
                                            pass_fds=(instance.fileno(),), start_new_session=True)
        try:
            self._identify_flatpak(args, output)
        except BaseException:
            _stop_unidentified_launch(self.process)
            raise

    def _identify_flatpak(self, args, output):
        deadline = time.monotonic() + 15
        expected = f"/app/lib/pax/{args.app.name}"
        while time.monotonic() < deadline and self.process.poll() is None:
            for pid in _descendants(self.process.pid):
                try:
                    argv = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
                    # Shell launchers may use ./name after cd /app/lib/pax.
                    if not argv or argv[0].decode() not in (expected, f"./{args.app.name}"):
                        continue
                    own_exe = Path(f"/proc/{pid}/exe")
                    installed_exe = Path(f"/proc/{pid}/root{expected}")
                    if not os.path.samefile(own_exe, installed_exe):
                        continue
                    self.pid = pid
                    self._expected_executable = own_exe.stat()
                    self.identity = {"application": args.flatpak_app, "branch": args.flatpak_branch,
                                     "launcherPid": self.process.pid, "applicationHostPid": pid,
                                     "instance": (output / "flatpak-instance").read_text().strip()}
                    (output / "process-identity.json").write_text(json.dumps(self.identity, indent=2)+"\n")
                    self.output = output
                    return
                except (FileNotFoundError, ProcessLookupError):
                    continue
            time.sleep(.05)
        raise RuntimeError("Could not identify the launched Flatpak application child; inspect app.log")

    @property
    def returncode(self):
        return self.process.returncode

    def poll(self):
        return self.process.poll()

    def wait(self, timeout=None):
        return self.process.wait(timeout=timeout)

    def _signal(self, value):
        if self.flatpak:
            try:
                current = Path(f"/proc/{self.pid}/exe").stat()
            except FileNotFoundError:
                return
            if (current.st_dev, current.st_ino) != (self._expected_executable.st_dev, self._expected_executable.st_ino):
                raise RuntimeError("Sandbox process identity changed before signal")
            os.kill(self.pid, value)
        else:
            self.process.send_signal(value)

    def send_signal(self, value):
        self._signal(value)

    def terminate(self):
        self._signal(signal.SIGTERM)

    def kill(self):
        self._signal(signal.SIGKILL)

    def x11_window(self, title="Pax native desktop acceptance", only_visible=True):
        # GTK can give an unmapped leader the same title and PID as its surface.
        # Input belongs to the visible surface, still verified by PID or XRes.
        command = ["xdotool", "search"]
        if only_visible:
            command.append("--onlyvisible")
        if not self.flatpak:
            command += ["--pid", str(self.pid)]
        command += ["--name", "^" + re.escape(title) + "$"]
        windows = subprocess.check_output(command, text=True).splitlines()
        matches = [w for w in windows if not self.flatpak or _x11_client_pid(w) == self.pid]
        if len(matches) != 1:
            raise RuntimeError(f"Expected one native window owned by application PID {self.pid}, got {matches}")
        if self.flatpak:
            self.identity["x11Window"] = matches[0]
            self.identity["xresHostPid"] = self.pid
            (self.output / "process-identity.json").write_text(json.dumps(self.identity, indent=2)+"\n")
        return matches[0]

    def matches_accessible(self, accessible):
        pid = accessible.get_process_id()
        if pid == self.pid:
            return True
        if not self.flatpak:
            return False
        # AT-SPI sees the D-Bus proxy's host PID, not PID 2 inside the sandbox.
        try:
            command = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")[0]
        except FileNotFoundError:
            return False
        if Path(os.fsdecode(command)).name != "xdg-dbus-proxy":
            return False
        # Flatpak wraps its D-Bus proxy in a mount namespace carrying the same
        # instance metadata. The proxy may be reparented, so ancestry alone is
        # insufficient; bind it to this launched instance and application ID.
        info = configparser.ConfigParser(interpolation=None)
        try:
            with Path(f"/proc/{pid}/root/.flatpak-info").open() as stream:
                info.read_file(stream)
        except (OSError, configparser.Error):
            return False
        if (info.get("Application", "name", fallback=None) != self.identity["application"] or
                info.get("Instance", "instance-id", fallback=None) != self.identity["instance"]):
            return False
        self.identity["atspiProxyHostPid"] = pid
        (self.output / "process-identity.json").write_text(json.dumps(self.identity, indent=2)+"\n")
        return True
