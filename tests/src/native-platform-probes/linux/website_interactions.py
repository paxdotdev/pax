"""Real-input checks for the unchanged website, separate from timing phases.

All movement uses native input. Reported positions identify targets and verify
outcomes; the probe never writes engine state or native scroll adjustments.
"""
import hashlib
import re
import time


class WebsiteInteractions:
    def __init__(self, *, state, wait, fresh, xdo, pointer_owner, key,
                 save, screenshot, window, outer_id, clipboard):
        self.state, self.wait, self.fresh = state, wait, fresh
        self.xdo, self.pointer_owner, self.key = xdo, pointer_owner, key
        self.save, self.screenshot = save, screenshot
        self.window, self.outer_id, self.clipboard = window, outer_id, clipboard
        self.results = []
        self.inputs = []

    def node(self, node_id, value=None):
        return next(n for n in (value or self.state()).get("nodes", []) if n["id"] == node_id)

    def outer(self, value=None):
        return self.node(self.outer_id, value)

    def nodes(self, value=None):
        return (value or self.state()).get("nodes", [])

    def visible(self, n, clip=None):
        if len(n.get("bounds", [])) != 4:
            return False
        x, y, w, h = n["bounds"]
        cx, cy, cw, ch = clip or self.state()["sceneBoundsInSurface"]
        return w > 0 and h > 0 and x < cx + cw - 3 and x + w > cx + 3 and y < cy + ch - 3 and y + h > cy + 3

    def text(self, text, *, value=None, clip=None):
        return [n for n in self.nodes(value) if n.get("text") == text
                and self.visible(n, clip)]

    def point(self, x, y):
        self.xdo("mousemove", "--window", self.window, round(x), round(y))
        self.pointer_owner()

    def click(self, n, clip=None):
        x, y, w, h = self.node(n["id"])["bounds"]
        cx, cy, cw, ch = clip or self.state()["sceneBoundsInSurface"]
        left, top = max(x, cx + 3), max(y, cy + 3)
        right, bottom = min(x + w, cx + cw - 3), min(y + h, cy + ch - 3)
        if right <= left or bottom <= top:
            raise RuntimeError(f"Interaction target is clipped: {n.get('text', n['id'])}")
        self.point((left + right) / 2, (top + bottom) / 2)
        self.record_input("click", node=n["id"], text=n.get("text", "")[:96],
                          point=[(left + right) / 2, (top + bottom) / 2])
        self.xdo("click", 1)
        return self.fresh()

    def wheel_at(self, bounds, button, count=1):
        x, y, w, h = bounds
        self.point(x + w / 2, y + h / 2)
        self.record_input("wheel", button=button, count=count, point=[x + w / 2, y + h / 2])
        self.xdo("click", "--repeat", count, "--delay", 35, button)
        time.sleep(.2)
        return self.fresh()

    def outer_wheel(self, button, count=1):
        x, y, w, h = self.outer()["bounds"]
        return self.wheel_at([x, y, w * .04, h], button, count)

    def seek(self, selector, description, target_y=200, timeout=70):
        """Bring an observed target near the top using the outer native wheel."""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            value = self.state()
            candidates = selector(value)
            if candidates:
                n = candidates[0]
                y = n["bounds"][1] + n["bounds"][3] / 2
                if abs(y - target_y) < 55 and self.visible(n, self.outer(value)["bounds"]):
                    return n
                direction = 5 if y > target_y else 4
                count = min(6, max(1, int(abs(y - target_y) / 90)))
            else:
                direction, count = 5, 4
            before = self.outer(value).get("scrollY", 0)
            self.outer_wheel(direction, count)
            after = self.outer().get("scrollY", 0)
            if abs(after - before) < 1 and candidates and self.visible(candidates[0], self.outer()["bounds"]):
                return candidates[0]
        raise RuntimeError("Could not reach native target: " + description)

    def seek_text(self, text, target_y=200):
        return self.seek(lambda s: [n for n in s.get("nodes", []) if n.get("text") == text
                                    and len(n.get("bounds", [])) == 4], text, target_y)

    def check(self, condition, description, **evidence):
        self.results.append({"check": description, "passed": bool(condition), **evidence})
        self.save("website-interactions.json", self.results)
        if not condition:
            raise RuntimeError("Website interaction failed: " + description)

    def record_input(self, kind, **values):
        root = next((n for n in self.nodes() if n["id"] == self.outer_id), None)
        self.inputs.append({"kind": kind, "monotonic": time.monotonic(),
                            "outerY": root.get("scrollY", 0) if root else None, **values})
        self.save("website-inputs.json", self.inputs)

    def run(self, cases):
        for name in cases:
            try:
                getattr(self, name.replace("-", "_"))()
                self.save("interaction-" + name + "-state.json", self.state())
                self.screenshot("interaction-" + name)
            except Exception:
                self.save("interaction-" + name + "-failure-state.json", self.state())
                self.screenshot("interaction-" + name + "-failure")
                raise

    def calculator(self):
        heading = self.seek_text("PAX-83+", 180)
        hx, hy, _, _ = heading["bounds"]
        # The title identifies the one calculator instance; do not select an
        # identically named label from another embedded example behind it.
        region = [hx - 18, hy, 355, 730]

        def button(label):
            candidates = self.text(label, clip=region)
            if not candidates:
                raise RuntimeError("No visible calculator key: " + label)
            # + and − also occur in the upper zoom controls.
            return max(candidates, key=lambda n: n["bounds"][1])

        self.click(button("CALCULATE"))
        self.click(button("clear"))
        for label in ("7", "+", "8", "enter"):
            self.click(button(label))
        result = self.wait(lambda s: any(n.get("text", "").strip() == "15"
                                        and self.visible(n, region)
                                        and n["bounds"][1] < hy + 280 for n in self.nodes(s)),
                           "calculator renders 7 + 8 = 15 from actual key clicks")
        self.check(True, "embedded calculator arithmetic", expression="7+8", result="15")
        self.save("calculator-arithmetic-state.json", result)
        self.click(button("GRAPH"))
        graph = self.wait(lambda s: any(n.get("text", "").startswith(("FUNCTION  /  RAD", "POLAR  /  RAD"))
                               and self.visible(n, region) for n in self.nodes(s)),
                  "calculator graph mode")
        self.save("calculator-graph-state.json", graph)
        self.check(not any(n.get("text") == "NO RESOLVED CURVE" and self.visible(n, region)
                           for n in self.nodes(graph)), "calculator initial graph resolves its default curve")

        def graph_range(value):
            for n in self.nodes(value):
                if not self.visible(n, region):
                    continue
                match = re.fullmatch(r"x (-?[\d.]+) … (-?[\d.]+)   y (-?[\d.]+) … (-?[\d.]+)",
                                     n.get("text", ""))
                if match:
                    return tuple(float(v) for v in match.groups())
            raise RuntimeError("Calculator graph range is not reported")

        initial_range = graph_range(graph)
        self.check(initial_range[0] < 0 < initial_range[1]
                   and initial_range[2] < 0 < initial_range[3],
                   "calculator starts with a centered graph", range=initial_range)
        self.screenshot("website-calculator-centered-graph")
        plot = next(n for n in self.nodes(graph) if n["kind"] == "Scroller"
                    and self.visible(n, region) and n.get("scrollUpper", 0) > 1000
                    and 30 < n["bounds"][3] < 300 and n["bounds"][2] > 200)
        outer_before = self.outer().get("scrollY", 0)
        initial_y = plot.get("scrollY", 0)
        self.wheel_at(plot["bounds"], 5)
        self.wait(lambda s: self.node(plot["id"], s).get("scrollY", 0) > initial_y + 1,
                  "fresh calculator graph responds to native wheel")
        time.sleep(.4)
        wheeled = self.fresh()
        self.check(self.node(plot["id"], wheeled).get("scrollY", 0) > initial_y + 1
                   and abs(self.outer(wheeled).get("scrollY", 0) - outer_before) <= 1,
                   "graph wheel retains native movement while its parent stays stationary",
                   initialY=initial_y, afterY=self.node(plot["id"], wheeled).get("scrollY", 0))
        self.save("calculator-graph-wheel-state.json", wheeled)
        self.wheel_at(self.node(plot["id"])["bounds"], 4)
        self.wait(lambda s: abs(self.node(plot["id"], s).get("scrollY", 0) - initial_y) <= 1,
                  "opposite native wheel restores graph position")
        self.check(True, "calculator graph wheel roundtrip preserves its page position")
        # The upper + is the authored zoom control; the lower one is arithmetic.
        self.click(min(self.text("+", clip=region), key=lambda n: n["bounds"][1]))
        zoomed = self.wait(lambda s: self.node(plot["id"], s).get("scrollUpper", 0)
                          > plot["scrollUpper"] * 1.5, "calculator zoom updates graph extents")
        zoomed_range = graph_range(zoomed)
        before_span = initial_range[1] - initial_range[0]
        after_span = zoomed_range[1] - zoomed_range[0]
        self.check(abs(after_span * 2 - before_span) <= .3
                   and abs(sum(zoomed_range[:2]) - sum(initial_range[:2])) <= .3
                   and abs(sum(zoomed_range[2:]) - sum(initial_range[2:])) <= .3,
                   "calculator zoom preserves world center and halves the visible range",
                   beforeRange=initial_range, afterRange=zoomed_range)
        plot = self.node(plot["id"])
        px, py, pw, ph = plot["bounds"]
        x0, y0 = px + pw / 2, py + ph / 2
        before_x, before_y = plot.get("scrollX", 0), plot.get("scrollY", 0)
        self.point(x0, y0)
        self.record_input("drag", node=plot["id"], start=[x0, y0], end=[x0 - 24, y0 - 12])
        self.xdo("mousedown", 1)
        try:
            time.sleep(.1)
            for step in range(1, 7):
                self.point(x0 - 4 * step, y0 - 2 * step)
                time.sleep(.04)
        finally:
            self.xdo("mouseup", 1)
        panned = self.wait(lambda s: self.node(plot["id"], s).get("scrollX", 0) > before_x + 5
                          and self.node(plot["id"], s).get("scrollY", 0) > before_y + 5,
                          "native pointer drag pans the calculator graph on both axes")
        self.check(abs(self.outer(panned).get("scrollY", 0) - outer_before) <= 1,
                   "calculator graph drag changes both native offsets without moving its parent",
                   beforeOffset=[before_x, before_y],
                   afterOffset=[self.node(plot["id"], panned).get("scrollX", 0),
                                self.node(plot["id"], panned).get("scrollY", 0)])
        self.save("calculator-graph-panned-state.json", panned)
        self.screenshot("website-calculator-panned-graph")
        self.click(button("CALCULATE"))
        self.wait(lambda s: any(n.get("text") == "CALCULATE  /  RAD"
                               and self.visible(n, region) for n in self.nodes(s)),
                  "calculator returns to calculation mode")
        self.check(True, "embedded calculator graph and calculate mode lifecycle")

    def paxflix(self):
        launch = self.seek_text("Open Paxflix full screen ↗", 300)
        before_ids = {n["id"] for n in self.nodes()}
        outer_before = self.outer().get("scrollY", 0)
        self.click(launch)
        opened = self.wait(lambda s: any(n.get("text") == "PAXFLIX / LIVE DEMO" for n in self.nodes(s)),
                           "full-screen Paxflix opens")
        new_ids = {n["id"] for n in self.nodes(opened)} - before_ids

        def new_text(label, value=None):
            return [n for n in self.text(label, value=value) if n["id"] in new_ids]

        catalog = max((n for n in self.nodes(opened) if n["id"] in new_ids and n["kind"] == "Scroller"),
                      key=lambda n: n["bounds"][2] * n["bounds"][3])
        self.check(catalog["bounds"][2] > 1000, "Paxflix full-screen catalog uses viewport width")
        self.screenshot("website-paxflix-open")
        self.click(new_text("+   More info")[0])
        self.wait(lambda s: bool(self.text("Paxflix / FEATURE", value=s)), "Paxflix movie detail opens")
        detail = max(self.text("Paxflix / FEATURE"), key=lambda n: n["id"])
        self.click(max(self.text("Play"), key=lambda n: n["id"]))
        self.wait(lambda s: bool(self.text("You found your next watch. Playback is a demo.", value=s)),
                  "Paxflix native click updates movie feedback")
        self.check(True, "Paxflix movie detail and playback-demo feedback")
        self.screenshot("website-paxflix-detail")
        self.click(max(self.text("×"), key=lambda n: n["id"]))
        self.wait(lambda s: all(n["id"] != detail["id"] for n in self.nodes(s)), "Paxflix movie detail retires")
        self.wheel_at(self.node(catalog["id"])["bounds"], 5, 3)
        moved = self.wait(lambda s: self.node(catalog["id"], s).get("scrollY", 0) > 50,
                          "Paxflix catalog scrolls independently")
        self.check(abs(self.outer(moved).get("scrollY", 0) - outer_before) <= 1,
                   "full-screen catalog scroll leaves website position unchanged")
        # A shelf belongs to this newly mounted demo and is clipped by its
        # catalog. The old inline demos' report nodes can lie at similar y values.
        cb = self.node(catalog["id"])["bounds"]
        shelves = [n for n in self.nodes() if n["id"] in new_ids and n["kind"] == "Scroller"
                   and n["id"] != catalog["id"] and self.visible(n, cb)
                   and n["bounds"][3] < 350]
        shelf = max(shelves, key=lambda n: max(0, min(n["bounds"][1] + n["bounds"][3], cb[1] + cb[3])
                                                      - max(n["bounds"][1], cb[1])))
        sx, sy, sw, sh = shelf["bounds"]
        clipped = [sx, max(sy, cb[1] + 5), sw,
                   min(sy + sh, cb[1] + cb[3] - 5) - max(sy, cb[1] + 5)]
        catalog_y = self.node(catalog["id"]).get("scrollY", 0)
        shelf_x = shelf.get("scrollX", 0)
        self.wheel_at(clipped, 7, 2)
        moved = self.wait(lambda s: self.node(shelf["id"], s).get("scrollX", 0) > shelf_x + 5,
                          "Paxflix shelf scrolls horizontally")
        self.check(abs(self.node(catalog["id"], moved).get("scrollY", 0) - catalog_y) <= 1
                   and abs(self.outer(moved).get("scrollY", 0) - outer_before) <= 1,
                   "Paxflix shelf wheel preserves catalog and website vertical positions")
        self.click(self.text("× Close demo")[0])
        self.wait(lambda s: not ({n["id"] for n in self.nodes(s)} & new_ids), "full-screen Paxflix subtree retires")
        self.check(abs(self.outer().get("scrollY", 0) - outer_before) <= 1,
                   "full-screen Paxflix closes and restores the website position")

    def gallery(self):
        title = self.seek_text("Meet your creative toolkit", 180)
        # The gallery's full-width rail is the only below-title horizontal
        # scroller with a current offset spanning several card widths.
        rail = min((n for n in self.nodes() if n["kind"] == "Scroller"
                    and n.get("scrollX", 0) > 1000 and n["bounds"][2] > 1000
                    and n["bounds"][1] > title["bounds"][1]), key=lambda n: n["bounds"][1])
        rail_id = rail["id"]
        self.seek(lambda s: [self.node(rail_id, s)], "gallery rail", 460)
        self.point(20, 90)
        a = self.node(rail_id).get("scrollX", 0)
        moving = self.wait(lambda s: self.node(rail_id, s).get("scrollX", 0) > a + 8,
                           "gallery automatic horizontal motion", 15)
        self.check(True, "gallery automatic motion", startX=a,
                   endX=self.node(rail_id, moving).get("scrollX", 0))
        rail = self.node(rail_id)
        x, y, w, h = rail["bounds"]
        self.point(x + w * .4, y + h * .5)
        time.sleep(.5)
        paused = self.node(rail_id).get("scrollX", 0)
        time.sleep(1)
        self.fresh()
        self.check(abs(self.node(rail_id).get("scrollX", 0) - paused) <= 1,
                   "gallery pauses on native mouse hover", startX=paused,
                   endX=self.node(rail_id).get("scrollX", 0))
        outer_y = self.outer().get("scrollY", 0)
        self.wheel_at(rail["bounds"], 7, 2)
        self.wait(lambda s: self.node(rail_id, s).get("scrollX", 0) > paused + 5,
                  "gallery horizontal wheel")
        self.check(abs(self.outer().get("scrollY", 0) - outer_y) <= 1,
                   "gallery horizontal wheel leaves parent stationary")
        self.wheel_at(self.node(rail_id)["bounds"], 5)
        self.wait(lambda s: self.outer(s).get("scrollY", 0) > outer_y + 5,
                  "gallery vertical wheel propagates to the website")
        self.check(True, "gallery vertical wheel propagates on its disabled axis")
        self.point(20, 90)
        resumed = self.node(rail_id).get("scrollX", 0)
        self.wait(lambda s: self.node(rail_id, s).get("scrollX", 0) > resumed + 8,
                  "gallery resumes after leaving", 15)
        self.check(True, "gallery resumes after hover ends")

    def resize(self):
        self.seek_text("A declarative language", 200)
        for width, height in ((640, 900), (1280, 960)):
            self.xdo("windowsize", "--sync", self.window, width, height)
            value = self.wait(lambda s: len(s.get("sceneBoundsInSurface", [])) == 4
                              and abs(s["sceneBoundsInSurface"][2] - width) <= 1
                              and abs(s["sceneBoundsInSurface"][3] - height) <= 1,
                              "native website resize")
            time.sleep(1)
            value = self.fresh()
            nav = "Get started →" if width < 760 else "Get started"
            self.check(bool(self.text(nav, value=value)), "website responsive navigation", width=width, height=height)
            root = self.outer(value)
            self.check(abs(root["bounds"][2] - width) <= 1 and root["bounds"][3] > height - 100,
                       "website scroller follows native resize", width=width, height=height)
            before = root.get("scrollY", 0)
            self.outer_wheel(5, 2)
            self.wait(lambda s: self.outer(s).get("scrollY", 0) > before + 5,
                      "website remains scrollable after resize")
            self.save(f"website-resize-{width}-state.json", self.state())
            self.screenshot(f"website-resize-{width}")
        self.check(True, "native narrow/wide resize preserves live scrolling")

    def path_studies(self):
        headings = sorted((n for n in self.nodes() if n.get("text") == "PATH DRAWING"),
                          key=lambda n: n["bounds"][1])
        self.check(len(headings) == 2, "both authored path studies are mounted", count=len(headings))
        for index, heading in enumerate(headings):
            self.seek(lambda s: [self.node(heading["id"], s)], "path study heading", 170)
            hx, hy, hw, _ = self.node(heading["id"])["bounds"]
            sliders = [n for n in self.nodes() if n["kind"] == "Slider"
                       and abs(n["bounds"][0] - hx) < 2 and abs(n["bounds"][2] - hw) < 2
                       and hy < n["bounds"][1] < hy + 600
                       and n.get("min") == 0 and n.get("max") == 1]
            self.check(len(sliders) == 1, "path study owns one native scrubber", study=index)
            slider_id = sliders[0]["id"]
            self.seek(lambda s: [self.node(slider_id, s)], "path study scrubber", 400)
            sx, sy, sw, sh = self.node(slider_id)["bounds"]
            toggles = [n for n in self.nodes() if n.get("text") in ("PAUSE  Ⅱ", "PLAY  →")
                       and abs(n["bounds"][0] - sx) < 2 and abs(n["bounds"][2] - sw) < 2
                       and sy + sh < n["bounds"][1] < sy + sh + 50]
            self.check(len(toggles) == 1, "path study owns one playback toggle", study=index)
            toggle_id = toggles[0]["id"]
            if self.node(toggle_id).get("text") == "PAUSE  Ⅱ":
                self.click(self.node(toggle_id))
            self.wait(lambda s: self.node(toggle_id, s).get("text") == "PLAY  →",
                      "path playback pauses")
            paused = self.node(slider_id)["value"]
            time.sleep(.8)
            value = self.fresh()
            self.check(abs(self.node(slider_id, value)["value"] - paused) < .002,
                       "paused path keeps its bound native slider value", study=index, value=paused)
            self.click(self.node(toggle_id))
            self.wait(lambda s: self.node(toggle_id, s).get("text") == "PAUSE  Ⅱ",
                      "path playback resumes")
            start = self.node(slider_id)["value"]
            self.wait(lambda s: abs(self.node(slider_id, s)["value"] - start) > .025,
                      "playing path advances its bound native slider")
            self.check(True, "playing path advances its bound native slider", study=index)
            # Focus the genuine GtkScale, then use its native range keyboard
            # actions. The application's scrub handler must also pause playback.
            before_events = self.node(slider_id).get("controlEvents", 0)
            self.click(self.node(slider_id))
            self.key("Home")
            value = self.wait(lambda s: abs(self.node(slider_id, s)["value"]) < .002
                              and self.node(toggle_id, s).get("text") == "PLAY  →",
                              "native scrub to start pauses the path")
            self.check(self.node(slider_id, value).get("controlEvents", 0) > before_events,
                       "native path scrub invokes its Rust change handler", study=index)
            self.key("End")
            value = self.wait(lambda s: abs(self.node(slider_id, s)["value"] - 1) < .002,
                              "native path scrub reaches its end")
            time.sleep(.8)
            value = self.fresh()
            self.check(abs(self.node(slider_id, value)["value"] - 1) < .002
                       and self.node(toggle_id, value).get("text") == "PLAY  →",
                       "scrubbed path remains paused at its end", study=index)
            self.save(f"website-path-study-{index}-state.json", value)
            self.screenshot(f"website-path-study-{index}")
            self.click(self.node(toggle_id))
            self.wait(lambda s: self.node(toggle_id, s).get("text") == "PAUSE  Ⅱ"
                      and self.node(slider_id, s)["value"] < .95,
                      "path resumes from the scrubbed endpoint")
            self.check(True, "path resumes from its scrubbed endpoint", study=index)

    def source_tabs(self):
        drawers = sorted((n for n in self.nodes() if n.get("text") == "View Source"),
                         key=lambda n: n["bounds"][1])
        self.check(len(drawers) == 6, "website exposes all six section source drawers", count=len(drawers))
        receipts = []
        for index, toggle in enumerate(drawers):
            self.seek(lambda s: [self.node(toggle["id"], s)], "section source drawer", 130)
            old_ids = {n["id"] for n in self.nodes()}
            self.click(toggle)
            value = self.wait(lambda s: any(n["id"] not in old_ids and n.get("richTextRunCount", 0) > 1
                                            for n in self.nodes(s)), "section source markup renders")
            time.sleep(.5)
            value = self.fresh()
            added = [n for n in self.nodes(value) if n["id"] not in old_ids]
            strips = [n for n in added if n["kind"] == "Scroller" and abs(n["bounds"][3] - 30) < 2]
            if len(strips) != 1:
                raise RuntimeError(f"Expected one newly mounted source tab strip, got {len(strips)}")
            strip_id = strips[0]["id"]
            bx, by, bw, bh = strips[0]["bounds"]
            tabs = sorted((n for n in added if n["kind"] == "Text" and n.get("text")
                           and by <= n["bounds"][1] < by + bh and n["bounds"][3] <= bh),
                          key=lambda n: n["bounds"][0])
            if not tabs:
                raise RuntimeError("Source drawer has no reported source tabs")
            close = next(n for n in added if n.get("text") == "x" and n["bounds"][1] < by)
            for tab in tabs:
                for _ in range(90):
                    strip = self.node(strip_id)
                    bx, by, bw, bh = strip["bounds"]
                    target = self.node(tab["id"])
                    tx, ty, tw, th = target["bounds"]
                    overlap = min(tx + tw, bx + bw - 5) - max(tx, bx + 5)
                    if overlap >= 35:
                        break
                    self.wheel_at(strip["bounds"], 7 if tx > bx else 6)
                else:
                    raise RuntimeError("Could not scroll to source tab " + tab["text"])
                self.click(target, strip["bounds"])
                selected = self.wait(lambda s: any(n["id"] not in old_ids and n["id"] != tab["id"]
                                                   and n.get("text") == tab["text"]
                                                   and n["bounds"][1] < by for n in self.nodes(s)),
                                     "source tab becomes active: " + tab["text"])
                rich = next(n for n in self.nodes(selected) if n["id"] not in old_ids
                            and n.get("richTextRunCount", 0) > 0 and n.get("selectable"))
                text = rich["text"]
                if "<pre data-pax-code-markup" in text:
                    raise RuntimeError("Source tab displays generated HTML instead of native rich text")
                if tab["id"] == tabs[0]["id"]:
                    self.save(f"website-source-drawer-{index}-state.json", selected)
                    self.screenshot(f"website-source-drawer-{index}")
                # Only the tab strip has moved. Click the first rendered line
                # inside the stationary code viewport before native selection.
                code = next(n for n in added if n["kind"] == "Scroller" and n["id"] != strip_id
                            and abs(n["bounds"][0] - bx) < 2 and n["bounds"][3] > 30)
                code = self.node(code["id"])
                rx, ry, _, _ = rich["bounds"]
                cx, cy, cw, ch = code["bounds"]
                line = rich["textLayout"]["lineLogicalBounds"][0]
                px, py = max(cx + 5, rx + line[0] + 4), ry + line[1] + line[3] / 2
                if not cx < px < cx + cw or not cy < py < min(cy + ch, self.state()["sceneBoundsInSurface"][3]):
                    raise RuntimeError("Source tab's first line is not visible: " + tab["text"])
                self.point(px, py)
                self.xdo("click", 1)
                self.key("ctrl+a")
                self.wait(lambda s: self.node(rich["id"], s).get("selectionStart") == 0
                          and self.node(rich["id"], s).get("selectionEnd") == len(text),
                          "select all active source text")
                self.key("ctrl+c")
                copied, owner = self.clipboard()
                encoded = text.encode("utf-8")
                receipt = {"drawer": index, "tab": tab["text"], "bytes": len(encoded),
                           "sha256": hashlib.sha256(encoded).hexdigest(), "clipboardOwner": owner,
                           "exactUTF8Match": copied == encoded,
                           "nonASCIICharacters": sum(ord(c) > 127 for c in text)}
                receipts.append(receipt)
                self.save("website-source-tabs.json", receipts)
                self.check(copied == encoded, "source tab exact native copy", drawer=index, tab=tab["text"])
            self.click(close)
            self.wait(lambda s: all(n["id"] != strip_id for n in self.nodes(s)), "source drawer closes")
            self.check(True, "all tabs in section source drawer copied and closed", drawer=index, count=len(tabs))
        self.check(True, "all six website source drawers and every discovered tab", tabs=len(receipts))

    def route(self):
        notes = self.seek_text("PAX NOTES →", 400)
        old_scrollers = {n["id"] for n in self.nodes() if n["kind"] == "Scroller"}
        navigation_started = time.monotonic()
        self.click(notes)
        missing = self.wait(lambda s: bool(self.text("404 / WRONG TURN", value=s)),
                            "the authored /blog route reaches the site's native fallback")
        self.save("website-route-first-not-found-state.json", missing)
        first_pending = old_scrollers & {n["id"] for n in self.nodes(missing)}
        first_frame = missing["frames"]
        started = time.monotonic()
        retirement = []

        def retired(value):
            # A missing/partial diagnostic report must never count as deletion.
            if value.get("frames", -1) < first_frame or not isinstance(value.get("nodes"), list):
                return False
            pending = [n for n in value["nodes"] if n["id"] in old_scrollers]
            if not retirement or retirement[-1]["frames"] != value["frames"]:
                retirement.append({"frames": value["frames"], "elapsedSeconds": time.monotonic() - started,
                                   "navigationElapsedSeconds": time.monotonic() - navigation_started,
                                   "nodeCount": len(value["nodes"]),
                                   "pending": [{"id": n["id"], "bounds": n.get("bounds")} for n in pending]})
                self.save("website-route-retirement.json", retirement)
            return not pending and any(n.get("text") == "404 / WRONG TURN" for n in value["nodes"])

        # Gallery studies author exit timelines. New route controls may mount
        # before outgoing descendants finish; require actual node retirement.
        missing = self.wait(retired, "native route retires all previous home scrollers")
        self.check(True, "native route navigation retires the previous home scrollers",
                   pendingAtFirstFallbackReport=len(first_pending),
                   navigationToRetirementSeconds=time.monotonic() - navigation_started,
                   retirementWaitSeconds=time.monotonic() - started,
                   retirementFrameDelta=missing["frames"] - first_frame)
        self.save("website-route-not-found-state.json", missing)
        self.click(self.text("Return home  →")[0])
        restored = self.wait(lambda s: any(n["kind"] == "Scroller" for n in self.nodes(s))
                             and not self.text("404 / WRONG TURN", value=s),
                             "native Return home remounts the website")
        root = max((n for n in self.nodes(restored) if n["kind"] == "Scroller"),
                   key=lambda n: n["bounds"][2] * n["bounds"][3])
        self.outer_id = root["id"]
        self.check(self.outer_id not in old_scrollers and abs(root.get("scrollY", 0)) <= 1,
                   "native route roundtrip remounts home at its initial position")
        self.outer_wheel(5, 2)
        self.wait(lambda s: self.outer(s).get("scrollY", 0) > 5,
                  "remounted home remains scrollable")
        self.check(True, "owned native /blog fallback and Return home route roundtrip")
