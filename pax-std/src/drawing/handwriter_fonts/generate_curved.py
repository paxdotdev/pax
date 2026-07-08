#!/usr/bin/env python3
"""Regenerate the `*Curved.svg` Handwriter font variants.

The EMS/Hershey SVG stroke fonts are mostly polylines. This script keeps the
original files as source assets and writes companion variants where continuous
line runs are lightly smoothed and emitted as cubic Beziers.
"""

from __future__ import annotations

import math
import re
from pathlib import Path


FONT_DIR = Path(__file__).resolve().parent
FONTS = [
    "EMSAllure",
    "EMSDelight",
    "EMSInvite",
    "EMSLeague",
    "EMSNeato",
    "EMSOsmotron",
    "EMSReadability",
    "EMSTech",
    "HersheySans1",
    "HersheyScript1",
]

DEFAULT_ANGLE_CUT = 120.0
ANGLE_CUTS = {
    "EMSOsmotron": 150.0,
    "EMSTech": 140.0,
}
SMOOTH_PASSES = 2
EPSILON = 1e-6

TOKEN_RE = re.compile(r"[MLCmlc]|[-+]?(?:\d*\.\d+|\d+\.?)(?:[eE][-+]?\d+)?")
D_ATTR_RE = re.compile(r'(?<![A-Za-z0-9_:-])d="([^"]*)"')


class Command:
    def __init__(self, kind: str, points: tuple[tuple[float, float], ...] = ()) -> None:
        self.kind = kind
        self.points = points


def is_command(token: str) -> bool:
    return len(token) == 1 and token in "MLCmlc"


def format_number(value: float) -> str:
    if abs(value) < 0.0005:
        value = 0.0
    text = f"{value:.3f}".rstrip("0").rstrip(".")
    return text or "0"


def distance(a: tuple[float, float], b: tuple[float, float]) -> float:
    return math.hypot(a[0] - b[0], a[1] - b[1])


def add(a: tuple[float, float], b: tuple[float, float]) -> tuple[float, float]:
    return (a[0] + b[0], a[1] + b[1])


def subtract(a: tuple[float, float], b: tuple[float, float]) -> tuple[float, float]:
    return (a[0] - b[0], a[1] - b[1])


def multiply(point: tuple[float, float], scalar: float) -> tuple[float, float]:
    return (point[0] * scalar, point[1] * scalar)


def smooth_point(
    previous: tuple[float, float],
    current: tuple[float, float],
    next_point: tuple[float, float],
) -> tuple[float, float]:
    return (
        previous[0] * 0.25 + current[0] * 0.5 + next_point[0] * 0.25,
        previous[1] * 0.25 + current[1] * 0.5 + next_point[1] * 0.25,
    )


def parse_path_data(data: str) -> list[Command]:
    tokens = TOKEN_RE.findall(data)
    output: list[Command] = []
    index = 0
    command = None
    current = (0.0, 0.0)

    def read_number() -> float:
        nonlocal index
        if index >= len(tokens) or is_command(tokens[index]):
            raise ValueError("expected number")
        value = float(tokens[index])
        index += 1
        return value

    def has_more_numbers() -> bool:
        return index < len(tokens) and not is_command(tokens[index])

    def read_point(relative: bool) -> tuple[float, float]:
        x = read_number()
        y = read_number()
        if relative:
            return (current[0] + x, current[1] + y)
        return (x, y)

    while index < len(tokens):
        if is_command(tokens[index]):
            command = tokens[index]
            index += 1
        if command is None:
            raise ValueError("expected command")

        relative = command.islower()
        normalized = command.upper()
        if normalized == "M":
            point = read_point(relative)
            output.append(Command("M", (point,)))
            current = point
            while has_more_numbers():
                point = read_point(relative)
                if distance(current, point) > EPSILON:
                    output.append(Command("L", (point,)))
                current = point
            command = "l" if relative else "L"
        elif normalized == "L":
            while has_more_numbers():
                point = read_point(relative)
                if distance(current, point) > EPSILON:
                    output.append(Command("L", (point,)))
                current = point
        elif normalized == "C":
            while has_more_numbers():
                control_1 = read_point(relative)
                control_2 = read_point(relative)
                point = read_point(relative)
                output.append(Command("C", (control_1, control_2, point)))
                current = point
        else:
            raise ValueError(f"unsupported path command: {command}")

    return output


def angle_degrees(
    previous: tuple[float, float],
    current: tuple[float, float],
    next_point: tuple[float, float],
) -> float:
    vector_1 = (previous[0] - current[0], previous[1] - current[1])
    vector_2 = (next_point[0] - current[0], next_point[1] - current[1])
    length_1 = math.hypot(*vector_1)
    length_2 = math.hypot(*vector_2)
    if length_1 < EPSILON or length_2 < EPSILON:
        return 180.0
    dot = (vector_1[0] * vector_2[0] + vector_1[1] * vector_2[1]) / (length_1 * length_2)
    return math.degrees(math.acos(max(-1.0, min(1.0, dot))))


def smooth_run(points: list[tuple[float, float]]) -> list[tuple[float, float]]:
    if len(points) < 4:
        return points
    output = points
    for _ in range(SMOOTH_PASSES):
        next_points = [output[0]]
        for index in range(1, len(output) - 1):
            next_points.append(smooth_point(output[index - 1], output[index], output[index + 1]))
        next_points.append(output[-1])
        output = next_points
    return output


def emit_run(points: list[tuple[float, float]], angle_cut: float) -> list[Command]:
    clean: list[tuple[float, float]] = []
    for point in points:
        if not clean or distance(clean[-1], point) > EPSILON:
            clean.append(point)
    if len(clean) < 2:
        return []

    cuts = [0]
    for index in range(1, len(clean) - 1):
        if angle_degrees(clean[index - 1], clean[index], clean[index + 1]) <= angle_cut:
            cuts.append(index)
    cuts.append(len(clean) - 1)

    output: list[Command] = []
    for start, end in zip(cuts, cuts[1:]):
        run = smooth_run(clean[start : end + 1])
        if len(run) == 2:
            output.append(Command("L", (run[1],)))
            continue
        for index in range(len(run) - 1):
            p0 = run[index - 1] if index else run[index]
            p1 = run[index]
            p2 = run[index + 1]
            p3 = run[index + 2] if index + 2 < len(run) else run[index + 1]
            control_1 = add(p1, multiply(subtract(p2, p0), 1.0 / 6.0))
            control_2 = subtract(p2, multiply(subtract(p3, p1), 1.0 / 6.0))
            output.append(Command("C", (control_1, control_2, p2)))
    return output


def curve_fit(commands: list[Command], angle_cut: float) -> list[Command]:
    output: list[Command] = []
    run: list[tuple[float, float]] = []

    def flush_run() -> None:
        nonlocal run
        output.extend(emit_run(run, angle_cut))
        run = []

    for command in commands:
        if command.kind == "M":
            flush_run()
            output.append(command)
            run = [command.points[0]]
        elif command.kind == "L":
            if not run:
                output.append(command)
                run = [command.points[0]]
            else:
                run.append(command.points[0])
        else:
            flush_run()
            output.append(command)
            run = [command.points[-1]]

    flush_run()
    return output


def serialize_path(commands: list[Command]) -> str:
    output: list[str] = []
    for command in commands:
        if command.kind == "M":
            point = command.points[0]
            output += ["M", format_number(point[0]), format_number(point[1])]
        elif command.kind == "L":
            point = command.points[0]
            output += ["L", format_number(point[0]), format_number(point[1])]
        elif command.kind == "C":
            control_1, control_2, point = command.points
            output += [
                "C",
                format_number(control_1[0]),
                format_number(control_1[1]),
                format_number(control_2[0]),
                format_number(control_2[1]),
                format_number(point[0]),
                format_number(point[1]),
            ]
    return " ".join(output)


def curve_fit_svg(svg: str, font_name: str, angle_cut: float) -> str:
    def replace_path(match: re.Match[str]) -> str:
        try:
            path = serialize_path(curve_fit(parse_path_data(match.group(1)), angle_cut))
        except ValueError:
            return match.group(0)
        return f'd="{path}"'

    output = D_ATTR_RE.sub(replace_path, svg)
    output = output.replace(f'<font id="{font_name}"', f'<font id="{font_name}Curved"')
    note = (
        "Curve-fitted Pax variant: straight-line glyph runs were converted to lightly "
        f"smoothed cubic Beziers for static smoothing; corner cutoff {angle_cut:g} degrees.\n"
    )
    return output.replace("</metadata>", note + "</metadata>", 1)


def main() -> None:
    for font_name in FONTS:
        angle_cut = ANGLE_CUTS.get(font_name, DEFAULT_ANGLE_CUT)
        source_path = FONT_DIR / f"{font_name}.svg"
        destination_path = FONT_DIR / f"{font_name}Curved.svg"
        destination_path.write_text(curve_fit_svg(source_path.read_text(), font_name, angle_cut))
        print(
            f"{font_name}: cutoff={angle_cut:g} "
            f"{source_path.stat().st_size} -> {destination_path.stat().st_size}"
        )


if __name__ == "__main__":
    main()
