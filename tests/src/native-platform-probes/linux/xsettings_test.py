"""Temporary XSettings font-DPI source for an otherwise unowned test display.

Uses the real X11 settings protocol observed by GTK, including inside Flatpak.
This tests propagation through that protocol, not a desktop's settings UI.
Never replaces an existing settings manager and leaves no persistent settings.
"""
import ctypes
import ctypes.util
import math
import struct
import time


class _ClientData(ctypes.Union):
    _fields_ = [("b", ctypes.c_char * 20), ("s", ctypes.c_short * 10),
                ("l", ctypes.c_long * 5)]


class _ClientMessage(ctypes.Structure):
    _fields_ = [("type", ctypes.c_int), ("serial", ctypes.c_ulong),
                ("send_event", ctypes.c_int), ("display", ctypes.c_void_p),
                ("window", ctypes.c_ulong), ("message_type", ctypes.c_ulong),
                ("format", ctypes.c_int), ("data", _ClientData)]


class _PropertyEvent(ctypes.Structure):
    _fields_ = [("type", ctypes.c_int), ("serial", ctypes.c_ulong),
                ("send_event", ctypes.c_int), ("display", ctypes.c_void_p),
                ("window", ctypes.c_ulong), ("atom", ctypes.c_ulong),
                ("time", ctypes.c_ulong), ("state", ctypes.c_int)]


class _Event(ctypes.Union):
    _fields_ = [("client", _ClientMessage), ("property", _PropertyEvent),
                ("pad", ctypes.c_long * 24)]


class XSettingsFontDpi:
    """Own a fresh XSettings selection until close(); reject an existing owner."""

    def __init__(self, dpi=96):
        library = ctypes.util.find_library("X11")
        if not library:
            raise RuntimeError("XSettings qualification requires libX11")
        self.x = ctypes.CDLL(library)
        display, window, atom = ctypes.c_void_p, ctypes.c_ulong, ctypes.c_ulong
        signatures = {
            "XOpenDisplay": ([ctypes.c_char_p], display),
            "XCloseDisplay": ([display], ctypes.c_int),
            "XDefaultScreen": ([display], ctypes.c_int),
            "XRootWindow": ([display, ctypes.c_int], window),
            "XInternAtom": ([display, ctypes.c_char_p, ctypes.c_int], atom),
            "XCreateSimpleWindow": ([display, window, ctypes.c_int, ctypes.c_int,
                                    ctypes.c_uint, ctypes.c_uint, ctypes.c_uint,
                                    ctypes.c_ulong, ctypes.c_ulong], window),
            "XDestroyWindow": ([display, window], ctypes.c_int),
            "XGetSelectionOwner": ([display, atom], window),
            "XSetSelectionOwner": ([display, atom, window, ctypes.c_ulong], ctypes.c_int),
            "XGrabServer": ([display], ctypes.c_int),
            "XUngrabServer": ([display], ctypes.c_int),
            "XSync": ([display, ctypes.c_int], ctypes.c_int),
            "XSelectInput": ([display, window, ctypes.c_long], ctypes.c_int),
            "XCheckWindowEvent": ([display, window, ctypes.c_long,
                                   ctypes.POINTER(_Event)], ctypes.c_int),
            "XChangeProperty": ([display, window, atom, atom, ctypes.c_int,
                                 ctypes.c_int, ctypes.POINTER(ctypes.c_ubyte),
                                 ctypes.c_int], ctypes.c_int),
            "XSendEvent": ([display, window, ctypes.c_int, ctypes.c_long,
                            ctypes.POINTER(_Event)], ctypes.c_int),
        }
        for name, (arguments, result) in signatures.items():
            function = getattr(self.x, name)
            function.argtypes, function.restype = arguments, result
        self.display = self.x.XOpenDisplay(None)
        self.window = 0
        self.serial = 0
        if not self.display:
            raise RuntimeError("Cannot open the existing test X11 display")
        try:
            screen = self.x.XDefaultScreen(self.display)
            root = self.x.XRootWindow(self.display, screen)
            self.selection = self.x.XInternAtom(
                self.display, f"_XSETTINGS_S{screen}".encode(), 0)
            self.property = self.x.XInternAtom(self.display, b"_XSETTINGS_SETTINGS", 0)
            self.window = self.x.XCreateSimpleWindow(
                self.display, root, -10, -10, 1, 1, 0, 0, 0)
            self.x.XSelectInput(self.display, self.window, 1 << 22)
            self._publish(dpi)
            # ICCCM manager acquisition uses a real server timestamp. A property
            # notification on our own window supplies one without touching root.
            notification = _Event()
            deadline = time.monotonic() + 2
            while not self.x.XCheckWindowEvent(
                    self.display, self.window, 1 << 22, ctypes.byref(notification)):
                if time.monotonic() >= deadline:
                    raise RuntimeError("No X11 property timestamp for manager acquisition")
                time.sleep(.01)
            timestamp = notification.property.time
            # Check and claim atomically; never displace a desktop settings
            # manager that started between the initial inventory and this run.
            self.x.XGrabServer(self.display)
            try:
                owner = self.x.XGetSelectionOwner(self.display, self.selection)
                if owner:
                    raise RuntimeError(f"XSettings already has owner {owner}; refusing takeover")
                self.x.XSetSelectionOwner(self.display, self.selection, self.window, timestamp)
                if self.x.XGetSelectionOwner(self.display, self.selection) != self.window:
                    raise RuntimeError("Could not acquire the test XSettings selection")
            finally:
                self.x.XUngrabServer(self.display)
                self.x.XSync(self.display, 0)
            event = _Event()
            event.client.type = 33  # ClientMessage
            event.client.display = self.display
            event.client.window = root
            event.client.message_type = self.x.XInternAtom(self.display, b"MANAGER", 0)
            event.client.format = 32
            event.client.data.l[0] = timestamp
            event.client.data.l[1] = self.selection
            event.client.data.l[2] = self.window
            self.x.XSendEvent(self.display, root, 0, 1 << 17, ctypes.byref(event))
            self.x.XSync(self.display, 0)
        except BaseException:
            self.close()
            raise

    def _publish(self, dpi):
        if not math.isfinite(dpi) or not 24 <= dpi <= 384:
            raise ValueError("Test font DPI must be finite and between 24 and 384")
        self.serial += 1
        value = round(dpi * 1024)
        names = [b"Xft/DPI", b"Gdk/UnscaledDPI"]
        data = bytearray(struct.pack("<B3xII", 0, self.serial, len(names)))
        for name in names:
            data += struct.pack("<BBH", 0, 0, len(name))
            data += name + b"\0" * (-len(name) % 4)
            data += struct.pack("<Ii", self.serial, value)
        raw = (ctypes.c_ubyte * len(data)).from_buffer_copy(data)
        self.x.XChangeProperty(self.display, self.window, self.property,
                               self.property, 8, 0, raw, len(data))
        self.x.XSync(self.display, 0)
        self.dpi = dpi

    def set_dpi(self, dpi):
        self.check_owner()
        self._publish(dpi)

    def check_owner(self):
        """Call during test waits so a newly started desktop manager wins."""
        if not self.display:
            raise RuntimeError("XSettings test session is closed")
        if self.x.XGetSelectionOwner(self.display, self.selection) != self.window:
            self.close()
            raise RuntimeError("XSettings ownership changed; refusing to modify another manager")

    def close(self):
        if self.display:
            # Destroying only our window releases our selection, if still owned,
            # and tells GTK to restore its fallback settings.
            if self.window:
                self.x.XDestroyWindow(self.display, self.window)
            self.x.XCloseDisplay(self.display)
            self.display, self.window = None, 0

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()
