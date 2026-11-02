"""Drive the installed GTK settings portal in an isolated acceptance session."""
import json
import os
from pathlib import Path
import time


class PortalFontScale:
    def __init__(self, output):
        from gi.repository import Gio, GLib
        self.Gio, self.GLib = Gio, GLib
        self.output = Path(output)
        config = Path(os.environ.get("XDG_CONFIG_HOME", "")).resolve()
        # Never change the logged-in user's normal dconf database.
        if not config.is_relative_to(self.output.parent.resolve()) or not config.is_dir():
            raise RuntimeError("Portal qualification requires a scoped XDG_CONFIG_HOME beside the evidence directory")
        self.bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        self.settings = Gio.Settings.new("org.gnome.desktop.interface")
        self.original = self.settings.get_user_value("text-scaling-factor")
        self.events = []
        self.closed = False
        # Activate/read the real portal before binding its process identity.
        self.read()
        self.owners = {name: self.owner(name) for name in [
            "org.freedesktop.portal.Desktop", "org.freedesktop.impl.portal.desktop.gtk"]}
        self.identities = {}
        for name, owner in self.owners.items():
            pid = self.call("org.freedesktop.DBus", "/org/freedesktop/DBus",
                            "org.freedesktop.DBus", "GetConnectionUnixProcessID",
                            "(s)", (owner,), "(u)")[0]
            self.identities[name] = {"owner": owner, "pid": pid,
                                    "command": Path(f"/proc/{pid}/cmdline").read_bytes().replace(b"\0", b" ").decode()}
        try:
            self.set_dpi(96)
        except BaseException:
            self.close()
            raise

    def call(self, name, path, interface, method, signature, values, result):
        return self.bus.call_sync(
            name, path, interface, method, self.GLib.Variant(signature, values),
            self.GLib.VariantType.new(result), self.Gio.DBusCallFlags.NONE, 5000, None).unpack()

    def owner(self, name):
        return self.call("org.freedesktop.DBus", "/org/freedesktop/DBus",
                         "org.freedesktop.DBus", "GetNameOwner", "(s)", (name,), "(s)")[0]

    def check_owner(self):
        for name, owner in self.owners.items():
            if self.owner(name) != owner:
                raise RuntimeError(f"Settings portal provider changed during qualification: {name}")

    def read(self):
        value = self.call("org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop",
                          "org.freedesktop.portal.Settings", "ReadOne", "(ss)",
                          ("org.gnome.desktop.interface", "text-scaling-factor"), "(v)")[0]
        while hasattr(value, "unpack"):
            value = value.unpack()
        return float(value)

    def set_dpi(self, dpi):
        self.check_owner()
        scale = dpi / 96
        if not self.settings.set_double("text-scaling-factor", scale):
            raise RuntimeError("The isolated desktop font-scale setting is not writable")
        self.Gio.Settings.sync()
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            observed = self.read()
            if abs(observed - scale) < .001:
                self.events.append({"requestedDpi": dpi, "requestedScale": scale,
                                    "portalScale": observed, "monotonic": time.monotonic()})
                self.save()
                return
            time.sleep(.05)
        raise RuntimeError(f"Real settings portal did not publish scale {scale}")

    def save(self):
        (self.output / "portal-settings.json").write_text(json.dumps({
            "providers": self.identities, "events": self.events, "restored": self.closed,
            "originalUserValue": self.original.unpack() if self.original is not None else None,
        }, indent=2) + "\n")

    def close(self):
        if self.closed:
            return
        if self.original is None:
            self.settings.reset("text-scaling-factor")
        else:
            self.settings.set_value("text-scaling-factor", self.original)
        self.Gio.Settings.sync()
        expected = self.settings.get_double("text-scaling-factor")
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            observed = self.read()
            if abs(observed - expected) < .001:
                self.closed = True
                self.events.append({"restoredScale": expected, "portalScale": observed,
                                    "monotonic": time.monotonic()})
                self.save()
                return
            time.sleep(.05)
        self.save()
        raise RuntimeError("Settings portal did not confirm restored scoped font scale")
