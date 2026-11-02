#!/usr/bin/env python3
"""Exercise the actual generated native ABI without a display or GPU."""
import ctypes
import json
from pathlib import Path
import sys
import time

library = ctypes.CDLL(sys.argv[1])
library.cfg_program_recipe.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
library.cfg_program_recipe.restype = ctypes.c_size_t
size = library.cfg_program_recipe(None, 0)
assert 0 < size < 1024 * 1024
buffer = ctypes.create_string_buffer(size)
assert library.cfg_program_recipe(buffer, size) == size
recipe = json.loads(buffer.raw)
exports = recipe["nativeExports"]
assert set(exports) == {"abiVersion", "programId", "init", "initWithData", "initEntry"}
assert len(set(exports.values())) == 5
for name in exports.values():
    assert getattr(library, name), name
for generic in ["pax_cartridge_abi_version", "pax_cartridge_program_id",
                "pax_init", "pax_init_with_data", "pax_init_entry"]:
    assert not hasattr(library, generic), f"Unqualified program export: {generic}"

abi = getattr(library, exports["abiVersion"])
abi.argtypes = []
abi.restype = ctypes.c_uint32
assert abi() == (3 if sys.platform == "darwin" else 4)
identity = getattr(library, exports["programId"])
identity.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
identity.restype = ctypes.c_size_t
assert identity(None, 0) == 64
buffer = ctypes.create_string_buffer(64)
assert identity(buffer, 64) == 64
assert buffer.raw.decode() == recipe["programId"]

initialize = getattr(library, exports["initEntry"])
initialize.argtypes = [ctypes.c_float, ctypes.c_float, ctypes.c_char_p,
                       ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t]
initialize.restype = ctypes.c_void_p
library.cfg_initializer_count.argtypes = [ctypes.c_bool]
library.cfg_initializer_count.restype = ctypes.c_size_t
library.cfg_dependency_initializer_count.argtypes = []
library.cfg_dependency_initializer_count.restype = ctypes.c_size_t


def counts():
    return (library.cfg_initializer_count(False), library.cfg_initializer_count(True),
            library.cfg_dependency_initializer_count())


assert counts() == (0, 0, 0), "Loading a library must not initialize any program"
for entry in [b"", b"crate::Missing", b"crate::Wrapper"]:
    assert not initialize(600, 400, entry, len(entry), None, 0)
assert counts() == (0, 0, 0), "Rejected selection must not run an initializer"
engines = [initialize(600, 400, entry, len(entry), None, 0)
           for entry in [b"crate::App", b"crate::Shell"]]
assert all(engines) and engines[0] != engines[1]
assert counts() == (1, 1, 0), "Reusing a runnable component must not initialize its program"

if sys.platform == "darwin":
    library.pax_dealloc_engine.argtypes = [ctypes.c_void_p]
    library.pax_dealloc_engine.restype = None
    for engine in engines:
        library.pax_dealloc_engine(engine)
else:
    library.pax_desktop_begin_close.argtypes = [ctypes.c_void_p]
    library.pax_desktop_begin_close.restype = None
    library.pax_desktop_finish_close.argtypes = [ctypes.c_void_p]
    library.pax_desktop_finish_close.restype = ctypes.c_bool
    for engine in engines:
        library.pax_desktop_begin_close(engine)
        deadline = time.monotonic() + 5
        while not library.pax_desktop_finish_close(engine):
            assert time.monotonic() < deadline, "Engine did not drain"
            time.sleep(0.01)
cartridge_counts = counts()
host_counts = None
if len(sys.argv) > 2:
    assert sys.platform == "darwin", "The optional SDK probe covers the macOS host"
    # Optional production SDK check, still without mounting UI or starting a GPU.
    host = ctypes.CDLL(sys.argv[2])
    host.pax_host_configure.argtypes = [ctypes.c_char_p, ctypes.c_size_t]
    host.pax_host_configure.restype = ctypes.c_int32
    host.pax_host_prepare_with_data.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    host.pax_host_prepare_with_data.restype = ctypes.c_void_p
    for name in ["pax_host_close", "pax_host_release"]:
        getattr(host, name).argtypes = [ctypes.c_void_p]
        getattr(host, name).restype = None
    selected = dict(recipe, cartridge=str(Path(sys.argv[1]).resolve()), entry="crate::App")

    def prepare(configuration):
        encoded = json.dumps(configuration).encode()
        assert host.pax_host_configure(encoded, len(encoded)) == 0
        return host.pax_host_prepare_with_data(None, 0)

    def close(session):
        host.pax_host_close(session)
        host.pax_host_release(session)

    session = prepare(selected)
    assert session, "Production native host must resolve the selected exports"
    assert counts() == (2, 1, 0)
    close(session)
    assert not prepare(dict(selected, programId="0" * 64)), "Reject a stale program"
    assert not prepare(dict(selected, nativeExports=dict(exports, initEntry="pax_init_entry"))), "Never fall back to generic entrypoints"
    assert counts() == (2, 1, 0), "Rejected metadata must not run an initializer"
    session = prepare(dict(selected, entry="crate::Shell"))
    assert session, "Recover after rejected metadata without reusing the old API"
    assert counts() == (2, 2, 0)
    close(session)
    host_counts = counts()

print(json.dumps({"result": "pass", "nativeExports": exports,
                  "programId": recipe["programId"], "initializerCounts": cartridge_counts,
                  "hostInitializerCounts": host_counts}))
