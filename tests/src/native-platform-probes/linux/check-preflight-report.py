#!/usr/bin/env python3
"""Validate evidence and guard against treating advertised capabilities as proof."""
import copy
import json
import sys

BASELINE = "physical_gpu_dma_buf_modifier_bidirectional_sync_fd_for_future_reuse"
UNTESTED = (
    "wgpu_pax_rendering_tested", "gdk_import_tested", "same_physical_device_import_tested",
    "composition_tested", "synchronization_tested", "foreign_queue_ownership_tested",
    "dma_buf_reservation_fences_tested", "hardware_performance_tested",
)


def require(condition, message):
    # This gate must remain effective when PYTHONOPTIMIZE is set by the caller.
    if not condition:
        raise ValueError(message)


def image_candidate(image, display_formats):
    return (
        image["tiling"] == "drm_modifier"
        and image["gdk_modifier_match"] is True
        and (image["fourcc"], image["modifier"]) in display_formats
        and image["query_result"] == 0
        and image["exportable"] is True
        and image["export"]["attempted"] is True
        and image["export"]["succeeded"] is True
        and image["export"]["stage"] == "fd_exported_and_closed"
        and image["export"]["vk_result"] == 0
        and image["export"]["actual_modifier"] == image["modifier"]
        and image["export"]["allocation_bytes"] > 0
        and image["export"]["contents_written"] is False
    )


def device_candidate(device, display_open, display_formats):
    sync = device["sync_fd"]
    # Vulkan's EXPORTABLE=1, IMPORTABLE=2, SYNC_FD handle=16. Extension strings
    # alone cannot satisfy this future bidirectional reuse baseline.
    return (
        display_open is True
        and device["device_class"] in ("integrated", "discrete")
        and device["physical_gpu_candidate"] is True
        and any(image_candidate(image, display_formats) for image in device["images"])
        and sync["queried"] is True and sync["exportable"] is True and sync["importable"] is True
        and sync["features"] & 3 == 3 and sync["compatible_handle_types"] & 16 != 0
    )


def validate(report, process_status):
    require(report["schema_version"] == 1, "unsupported schema")
    require(report["probe"] == "pax-linux-hardware-preflight", "wrong probe")
    require(report["baseline"] == BASELINE, "unknown eligibility baseline")
    require(all(report["qualification"][field] is False for field in UNTESTED), "preflight cannot qualify composition or synchronization")
    gdk = report["gdk"]
    display_formats = {(item["fourcc"], item["modifier"]) for item in gdk["dmabuf_formats"]}
    eligible = False
    for device in report["vulkan"]["devices"]:
        for field in ("device_uuid", "driver_uuid"):
            value = device[field]
            require(len(value) == 32 and all(c in "0123456789abcdef" for c in value), f"invalid {field}")
        require(device["sync_fd"]["handoff_tested"] is False, "properties are not a semaphore handoff test")
        expected = device_candidate(device, gdk["display_open"], display_formats)
        require(device["preflight_eligible"] is expected, "device eligibility exceeds recorded evidence")
        eligible |= expected
    eligible = eligible and not report["operational_error"]
    require(report["preflight_eligible"] is eligible, "aggregate eligibility exceeds recorded evidence")
    expected_status = 1 if report["operational_error"] else 0 if eligible else 2
    require(report["exit_status"] == expected_status == process_status, "exit status disagrees with evidence")


def self_test():
    image = {"tiling": "drm_modifier", "gdk_modifier_match": True, "fourcc": 875708993,
             "modifier": "0x0000000000000000", "query_result": 0, "exportable": True,
             "export": {"attempted": True, "succeeded": True, "stage": "fd_exported_and_closed",
                        "vk_result": 0, "actual_modifier": "0x0000000000000000",
                        "allocation_bytes": 16384, "contents_written": False}}
    device = {"device_class": "discrete", "physical_gpu_candidate": True,
              "device_uuid": "1" * 32, "driver_uuid": "2" * 32,
              "images": [image], "preflight_eligible": True,
              "sync_fd": {"queried": True, "exportable": True, "importable": True,
                          "features": 3, "compatible_handle_types": 16, "handoff_tested": False}}
    report = {"schema_version": 1, "probe": "pax-linux-hardware-preflight", "baseline": BASELINE,
              "gdk": {"display_open": True, "dmabuf_formats": [{"fourcc": image["fourcc"], "modifier": image["modifier"]}]},
              "vulkan": {"devices": [device]}, "qualification": dict.fromkeys(UNTESTED, False),
              "operational_error": False, "preflight_eligible": True, "exit_status": 0}
    validate(report, 0)
    # Each mutation must invalidate a claimed success, even if extension strings
    # or other fields still look plausible. These are policy fixtures, not runs.
    cases = [
        lambda r: r["vulkan"]["devices"][0].update(device_class="cpu"),
        lambda r: r["vulkan"]["devices"][0].update(device_class="virtual"),
        lambda r: r["gdk"].update(display_open=False),
        lambda r: r["gdk"].update(dmabuf_formats=[]),
        lambda r: r["vulkan"]["devices"][0]["images"][0]["export"].update(succeeded=False),
        lambda r: r["vulkan"]["devices"][0]["images"][0]["export"].update(actual_modifier="0x0000000000000001"),
        lambda r: r["vulkan"]["devices"][0]["images"][0].update(tiling="linear"),
        lambda r: r["vulkan"]["devices"][0]["sync_fd"].update(features=0),
        lambda r: r["vulkan"]["devices"][0]["sync_fd"].update(importable=False),
        lambda r: r["qualification"].update(synchronization_tested=True),
    ]
    for change in cases:
        invalid = copy.deepcopy(report)
        change(invalid)
        try:
            validate(invalid, 0)
        except ValueError:
            pass
        else:
            raise ValueError("false-positive hardware gate was accepted")
    cpu = copy.deepcopy(report)
    cpu["vulkan"]["devices"][0].update(device_class="cpu", physical_gpu_candidate=False, preflight_eligible=False)
    cpu.update(preflight_eligible=False, exit_status=2)
    validate(cpu, 2)
    failure = copy.deepcopy(report)
    failure.update(operational_error=True, preflight_eligible=False, exit_status=1)
    validate(failure, 1)
    print("PASS preflight schema, eligible policy fixture, ten false-positive guards, CPU exit2, operational exit1")


if __name__ == "__main__":
    try:
        require(not sys.flags.optimize, "run the evidence validator without Python optimization")
        if sys.argv[1:] == ["--self-test"]:
            self_test()
        else:
            with open(sys.argv[1], encoding="utf-8") as source:
                validate(json.load(source), int(sys.argv[2]))
    except (KeyError, IndexError, OSError, TypeError, ValueError) as error:
        print(f"Invalid preflight report: {error}", file=sys.stderr)
        sys.exit(1)
