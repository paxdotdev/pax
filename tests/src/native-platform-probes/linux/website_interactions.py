"""Real-input checks for the unchanged website, separate from timing phases.

All movement uses native input. Reported positions identify targets and verify
outcomes; the probe never writes engine state or native scroll adjustments.
"""
import hashlib
import re
import time


class WebsiteInteractions:
    def __init__(self, *, state, wait, fresh, xdo, pointer_owner, key,
                 save, screenshot, window, outer_id, clipboard, source_sections=None):
        self.state, self.wait, self.fresh = state, wait, fresh
        self.xdo, self.pointer_owner, self.key = xdo, pointer_owner, key
        self.save, self.screenshot = save, screenshot
        self.window, self.outer_id, self.clipboard = window, outer_id, clipboard
        self.source_sections = source_sections
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
        # Wheel and drawer animations can move a target after seek() sees it.
        # Wait for observed geometry to settle before using X11 coordinates.
        observed = {"bounds": None, "since": time.monotonic(), "frame": None}

        def stationary(value):
            bounds = self.node(n["id"], value)["bounds"]
            if observed["bounds"] is None or any(abs(a - b) > .5 for a, b in zip(bounds, observed["bounds"])):
                observed.update(bounds=bounds, since=time.monotonic(), frame=value.get("frames"))
            return time.monotonic() - observed["since"] >= .35 and value.get("frames") != observed["frame"]

        value = self.wait(stationary, "native click target geometry settles", 5)
        x, y, w, h = self.node(n["id"], value)["bounds"]
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

    def returned_home(self, value, old_ids, requested_y, label):
        root = max((n for n in self.nodes(value) if n["kind"] == "Scroller"),
                   key=lambda n: n["bounds"][2] * n["bounds"][3])
        self.outer_id = root["id"]
        maximum = max(0, root["scrollUpper"] - root["scrollPage"])
        expected = min(max(0, requested_y), maximum)
        self.check(self.outer_id not in old_ids and abs(root.get("scrollY", 0) - expected) <= 1,
                   "home remount applies persisted bound scroll with allocation-time clamping",
                   route=label, requestedY=requested_y, firstY=root.get("scrollY", 0),
                   firstRangeMax=maximum, expectedY=expected)
        observations = []
        stable = {"range": None, "since": time.monotonic(), "frame": None}

        def settled(state):
            current = self.outer(state)
            extent = (current["scrollUpper"], current["scrollPage"])
            if extent != stable["range"]:
                stable["range"], stable["since"] = extent, time.monotonic()
            if state.get("frames") != stable["frame"]:
                stable["frame"] = state.get("frames")
                observations.append({"frames": state.get("frames"), "id": current["id"],
                                     "scrollY": current.get("scrollY", 0),
                                     "upper": extent[0], "page": extent[1],
                                     "nativeImagePending": state.get("nativeImagePending")})
                self.save("website-" + label + "-return-scroll.json", observations)
            images = [n for n in self.nodes(state) if n["kind"] == "NativeImage" and n.get("url")]
            return (time.monotonic() - stable["since"] >= 1 and images
                    and all(n.get("imageReady") for n in images))

        value = self.wait(settled, "remounted home layout and native images settle")
        root = self.outer(value)
        maximum = max(0, root["scrollUpper"] - root["scrollPage"])
        offset = root.get("scrollY", 0)
        self.check(-1 <= offset <= maximum + 1, "settled remount retains a legal bound scroll offset",
                   route=label, requestedY=requested_y, finalY=offset, finalRangeMax=maximum,
                   exactOriginalOffsetRestored=abs(offset - requested_y) <= 1)
        # The page owns its persisted offset. A native clamp is fed back to that
        # binding, so later extent growth does not resurrect the earlier request.
        self.outer_wheel(5 if offset < maximum - 5 else 4, 2)
        self.wait(lambda state: abs(self.outer(state).get("scrollY", 0) - offset) > 5,
                  "remounted home remains scrollable")
        self.check(True, "remounted home responds to native wheel input", route=label)

    def calculator(self):
        launch = self.seek(lambda s: self.text("TRY THE CALCULATOR  ↗", value=s),
                           "authored calculator route", 300)
        home_ids = {n["id"] for n in self.nodes()}
        home_scrollers = {n["id"] for n in self.nodes() if n["kind"] == "Scroller"}
        requested_home_y = self.outer().get("scrollY", 0)
        self.click(launch)
        opened = self.wait(lambda s: bool(self.text("CALCULATOR / LIVE", value=s))
                           and not (home_scrollers & {n["id"] for n in self.nodes(s)}),
                           "calculator route replaces the screenshot-only homepage")
        self.outer_id = max((n for n in self.nodes(opened) if n["kind"] == "Scroller"),
                            key=lambda n: n["bounds"][2] * n["bounds"][3])["id"]
        heading = self.text("PAX-83+", value=opened)[0]
        hx, hy, _, _ = heading["bounds"]
        region = self.outer(opened)["bounds"]

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
        self.check(True, "full-screen calculator arithmetic", expression="7+8", result="15")
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
                    and n["id"] != self.outer_id and self.visible(n, region)
                    and n.get("scrollUpper", 0) > 1000 and n["bounds"][2] > 200)
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
        self.check(True, "full-screen calculator graph and calculate mode lifecycle")
        calculator_scrollers = {n["id"] for n in self.nodes() if n["kind"] == "Scroller"}
        self.click(self.text("← BACK TO PAX")[0])
        restored = self.wait(lambda s: not self.text("CALCULATOR / LIVE", value=s)
                             and not (calculator_scrollers & {n["id"] for n in self.nodes(s)})
                             and any(n["kind"] == "Scroller" for n in self.nodes(s)),
                             "calculator returns to the homepage")
        self.returned_home(restored, home_ids, requested_home_y, "calculator-controls")

    def paxflix(self):
        launch = self.seek(lambda s: self.text("OPEN PAXFLIX FULL SCREEN  ↗", value=s)
                           or self.text("TRY PAXFLIX  ↗", value=s), "authored Paxflix route", 300)
        before_ids = {n["id"] for n in self.nodes()}
        old_outer = self.outer_id
        requested_home_y = self.outer().get("scrollY", 0)
        self.click(launch)
        opened = self.wait(lambda s: any(n.get("text") == "PAXFLIX / LIVE" for n in self.nodes(s)),
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
        self.wait(lambda s: all(n["id"] != old_outer for n in self.nodes(s)),
                  "Paxflix route retires the home scroller")
        self.check(True, "Paxflix route replaces the home scroller")
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
        self.check(abs(self.node(catalog["id"], moved).get("scrollY", 0) - catalog_y) <= 1,
                   "Paxflix shelf wheel preserves catalog vertical position")
        self.click(self.text("← BACK TO PAX")[0])
        restored = self.wait(lambda s: not ({n["id"] for n in self.nodes(s)} & new_ids)
                             and any(n["kind"] == "Scroller" for n in self.nodes(s)),
                             "Paxflix route retires and home remounts")
        self.returned_home(restored, before_ids, requested_home_y, "paxflix")

    def playgrounds(self):
        for label, heading, name in [
                ("TRY THE OSCILLOSCOPE  ↗", "OSCILLOSCOPE / LIVE", "oscilloscope"),
                ("PLAY SPACE GAME  ↗", "HOLD TO STEER / WASD + SPACE", "space-game")]:
            launch = self.seek_text(label, 300)
            old_ids = {n["id"] for n in self.nodes()}
            old_outer = self.outer_id
            requested_y = self.outer().get("scrollY", 0)
            self.click(launch)
            opened = self.wait(lambda s: self.text(heading, value=s)
                               and all(n["id"] != old_outer for n in self.nodes(s)),
                               name + " live route replaces the homepage", 60)
            self.check(True, "new playground route mounts and retires home scroller", route=name)
            if name == "oscilloscope":
                # The authored display occupies the left of the full-screen
                # instrument; clicking it opens the native message editor.
                x, y, width, height = opened["sceneBoundsInSurface"]
                self.point(x + width * .25, y + 52 + (height - 52) * .35)
                self.record_input("click", purpose="open oscilloscope message editor")
                self.xdo("click", 1)
                editor = self.wait(lambda s: any(n["kind"] == "Textbox" for n in self.nodes(s)),
                                   "oscilloscope message editor opens")
                textbox = next(n for n in self.nodes(editor) if n["kind"] == "Textbox")
                self.click(textbox)
                self.key("ctrl+a")
                message = "PAX LINUX ALPHA"
                self.xdo("type", "--clearmodifiers", "--delay", 25, message)
                self.wait(lambda s: self.node(textbox["id"], s).get("text") == message,
                          "native textbox accepts oscilloscope message")
                self.click(self.text("OK")[0])
                self.wait(lambda s: all(n["id"] != textbox["id"] for n in self.nodes(s)),
                          "message editor closes after confirmation")
                self.point(x + width * .25, y + 52 + (height - 52) * .35)
                self.xdo("click", 1)
                reopened = self.wait(lambda s: any(n["kind"] == "Textbox"
                                                   and n.get("text") == message for n in self.nodes(s)),
                                     "confirmed oscilloscope message persists on reopen")
                self.check(True, "oscilloscope native text editing persists through confirmation and reopen",
                           route=name, message=message)
                self.screenshot("website-oscilloscope-editor")
                self.click(self.text("OK", value=reopened)[0])
            else:
                game = self.wait(lambda s: any(n.get("text", "").startswith("Score:") for n in self.nodes(s)),
                                 "space game score mounts")
                self.save("website-space-game-arrival-state.json", game)
                restarted = bool(self.text("GAME OVER", value=game))
                if restarted:
                    self.click(self.text("Play Again", value=game)[0])
                    self.wait(lambda s: not self.text("GAME OVER", value=s)
                              and self.text("Score: 0", value=s),
                              "native Play Again starts a fresh round")
                    self.check(True, "space game restarts through its native Play Again button", route=name)
                x, y, width, height = opened["sceneBoundsInSurface"]
                self.point(x + width * .5, y + 52 + (height - 52) * .65)
                self.xdo("click", 1)
                active = self.wait(lambda s: s.get("canvasHasFocus")
                                   and not self.text("GAME OVER", value=s)
                                   and any(n.get("text", "").startswith("Score:") for n in self.nodes(s)),
                                   "active space-game canvas owns keyboard focus")
                self.save("website-space-game-before-input-state.json", active)
                start_frame = active["frames"]
                self.record_input("keyboard-hold", keys=["d", "space"], seconds=1)
                self.xdo("keydown", "d", "space")
                try:
                    time.sleep(1)
                finally:
                    self.xdo("keyup", "d", "space")
                advanced = self.wait(lambda s: s["frames"] > start_frame + 3,
                                     "space game continues publishing after real keyboard input")
                self.save("website-space-game-after-input-state.json", advanced)
                self.check(True, "space game stays live after keyboard input from an active round",
                           route=name, framesBefore=start_frame, framesAfter=advanced["frames"],
                           restarted=restarted, activeBeforeInput=True,
                           gameOverAfterInput=bool(self.text("GAME OVER", value=advanced)),
                           limitation="Smoke only: vector ship position and projectile behavior are not asserted")
            self.screenshot("website-" + name + "-live")
            self.click(self.text("← BACK TO PAX")[0])
            restored = self.wait(lambda s: not self.text(heading, value=s)
                                 and any(n["kind"] == "Scroller" for n in self.nodes(s)),
                                 name + " returns to homepage", 60)
            self.returned_home(restored, old_ids, requested_y, name)

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
        self.seek_text("A PORTABLE RUNTIME", 200)
        for width, height in ((640, 900), (1280, 960)):
            self.xdo("windowsize", "--sync", self.window, width, height)
            value = self.wait(lambda s: len(s.get("sceneBoundsInSurface", [])) == 4
                              and abs(s["sceneBoundsInSurface"][2] - width) <= 1
                              and abs(s["sceneBoundsInSurface"][3] - height) <= 1,
                              "native website resize")
            time.sleep(1)
            value = self.fresh()
            nav = "Get Started"
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

    def source_tabs(self):
        if not self.source_sections:
            raise RuntimeError("Source-tab qualification requires the authored section manifest")
        sections = self.source_sections["sections"]
        self.wait(lambda s: len([n for n in self.nodes(s) if n.get("text") == "VIEW SOURCE"])
                  == len(sections), "all authored source toggles mount", 60)
        drawers = sorted((n for n in self.nodes() if n.get("text") == "VIEW SOURCE"),
                         key=lambda n: n["bounds"][1])
        self.check(len(drawers) == len(sections), "website exposes every authored section source drawer", count=len(drawers))
        receipts = []
        for index, toggle in enumerate(drawers):
            # Keep the target clear of the website's fixed 80px navigation;
            # seek's tolerance and native wheel settling can shift it upward.
            self.seek(lambda s: [self.node(toggle["id"], s)], "section source drawer", 200)
            old_ids = {n["id"] for n in self.nodes()}
            self.click(toggle)
            value = self.wait(lambda s: any(n["id"] not in old_ids and n.get("richTextRunCount", 0) > 1
                                            for n in self.nodes(s)), "section source markup renders")
            time.sleep(.5)
            value = self.fresh()
            added = [n for n in self.nodes(value) if n["id"] not in old_ids]
            strips = [n for n in added if n["kind"] == "Scroller" and abs(n["bounds"][3] - 48) < 2]
            if len(strips) != 1:
                raise RuntimeError(f"Expected one newly mounted source tab strip, got {len(strips)}")
            strip_id = strips[0]["id"]
            bx, by, bw, bh = strips[0]["bounds"]
            tabs = sorted((n for n in added if n["kind"] == "Text" and n.get("text")
                           and by <= n["bounds"][1] < by + bh and n["bounds"][3] <= bh),
                          key=lambda n: n["bounds"][0])
            if not tabs:
                raise RuntimeError("Source drawer has no reported source tabs")
            close = self.node(toggle["id"], value)
            self.check(close.get("text") == "×", "source toggle exposes its native close action", drawer=index)
            expected = {n["label"]: n["displaySha256Utf8"] for n in sections[index]["sources"]}
            self.check([n["text"] for n in tabs] == list(expected),
                       "source tabs match the authored section membership and order", drawer=index,
                       section=sections[index]["id"], tabs=len(tabs))
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
                # ExampleSourceViewer's heading is the section title. The
                # authored selected tab uses black native text; idle tabs are
                # white. Verify that state, then independently check copied
                # content against the frozen source catalogue.
                selected = self.wait(lambda s: self.node(tab["id"], s).get("textStyle", {}).get("fill") == [0, 0, 0, 1]
                                     and sum(self.node(t["id"], s).get("textStyle", {}).get("fill") == [0, 0, 0, 1]
                                             for t in tabs) == 1,
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
                           "selectedNativeTextFill": self.node(tab["id"], selected)["textStyle"]["fill"],
                           "exactUTF8Match": copied == encoded,
                           "expectedSHA256": expected[tab["text"]],
                           "authoredSourceHashMatch": hashlib.sha256(encoded).hexdigest() == expected[tab["text"]],
                           "nonASCIICharacters": sum(ord(c) > 127 for c in text)}
                receipts.append(receipt)
                self.save("website-source-tabs.json", receipts)
                self.check(copied == encoded and receipt["authoredSourceHashMatch"],
                           "source tab exact native copy matches authored content", drawer=index, tab=tab["text"])
            self.click(close)
            self.wait(lambda s: all(n["id"] != strip_id for n in self.nodes(s)), "source drawer closes")
            self.check(True, "all tabs in section source drawer copied and closed", drawer=index, count=len(tabs))
        self.check(len(receipts) == sum(len(s["sources"]) for s in sections),
                   "all authored website source drawers and tabs", drawers=len(sections), tabs=len(receipts))

    def route(self):
        link = self.seek(lambda s: self.text("OPEN CALCULATOR FULL SCREEN  ↗", value=s)
                         or self.text("TRY THE CALCULATOR  ↗", value=s),
                         "authored calculator route", 400)
        old_scrollers = {n["id"] for n in self.nodes() if n["kind"] == "Scroller"}
        requested_home_y = self.outer().get("scrollY", 0)
        navigation_started = time.monotonic()
        self.click(link)
        calculator = self.wait(lambda s: bool(self.text("CALCULATOR / LIVE", value=s)),
                            "the authored /calculator route mounts the full calculator")
        self.save("website-route-first-calculator-state.json", calculator)
        first_pending = old_scrollers & {n["id"] for n in self.nodes(calculator)}
        first_frame = calculator["frames"]
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
            return not pending and any(n.get("text") == "CALCULATOR / LIVE" for n in value["nodes"])

        # Gallery studies author exit timelines. New route controls may mount
        # before outgoing descendants finish; require actual node retirement.
        calculator = self.wait(retired, "native route retires all previous home scrollers")
        self.check(True, "native route navigation retires the previous home scrollers",
                   pendingAtFirstRouteReport=len(first_pending),
                   navigationToRetirementSeconds=time.monotonic() - navigation_started,
                   retirementWaitSeconds=time.monotonic() - started,
                   retirementFrameDelta=calculator["frames"] - first_frame)
        self.save("website-route-calculator-state.json", calculator)
        calculator_scrollers = {n["id"] for n in self.nodes(calculator) if n["kind"] == "Scroller"}
        self.click(self.text("← BACK TO PAX")[0])
        restored = self.wait(lambda s: any(n["kind"] == "Scroller" for n in self.nodes(s))
                             and not (calculator_scrollers & {n["id"] for n in self.nodes(s)})
                             and not self.text("CALCULATOR / LIVE", value=s),
                             "native Return home remounts the website")
        self.returned_home(restored, old_scrollers, requested_home_y, "calculator")
        self.check(True, "owned native /calculator and Back to Pax route roundtrip")
