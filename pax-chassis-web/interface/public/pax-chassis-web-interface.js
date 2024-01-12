"use strict";
var Pax = (() => {
  var __defProp = Object.defineProperty;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };
  var __copyProps = (to, from, except, desc) => {
    if (from && typeof from === "object" || typeof from === "function") {
      for (let key of __getOwnPropNames(from))
        if (!__hasOwnProp.call(to, key) && key !== except)
          __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
    }
    return to;
  };
  var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

  // src/index.ts
  var src_exports = {};
  __export(src_exports, {
    mount: () => mount,
    processMessages: () => processMessages
  });

  // src/pools/object-pool.ts
  var ObjectPool = class {
    constructor(factory, cleanUp) {
      this.pool = [];
      this.factory = factory;
      this.cleanUp = cleanUp;
    }
    get(args) {
      if (this.pool.length > 0) {
        return this.pool.pop();
      }
      return this.factory(args);
    }
    put(item) {
      this.cleanUp(item);
      this.pool.push(item);
    }
  };

  // src/pools/object-manager.ts
  var ObjectManager = class {
    constructor(pools) {
      this.pools = /* @__PURE__ */ new Map();
      for (const pool of pools) {
        this.registerPool(pool.name, pool.factory, pool.cleanUp);
      }
    }
    registerPool(name, factory, reset) {
      this.pools.set(name, new ObjectPool(factory, reset));
    }
    getFromPool(name, args) {
      const pool = this.pools.get(name);
      if (!pool) {
        throw new Error(`No pool registered with name: ${name}`);
      }
      return pool.get(args);
    }
    returnToPool(name, item) {
      const pool = this.pools.get(name);
      if (!pool) {
        throw new Error(`No pool registered with name: ${name}`);
      }
      pool.put(item);
    }
  };

  // src/classes/messages/any-create-patch.ts
  var AnyCreatePatch = class {
    fromPatch(jsonMessage) {
      this.idChain = jsonMessage["id_chain"];
      this.clippingIds = jsonMessage["clipping_ids"];
      this.scrollerIds = jsonMessage["scroller_ids"];
      this.zIndex = jsonMessage["z_index"];
    }
    cleanUp() {
      this.idChain = [];
      this.clippingIds = [];
      this.scrollerIds = [];
      this.zIndex = -1;
    }
  };

  // src/classes/messages/frame-update-patch.ts
  var FrameUpdatePatch = class {
    fromPatch(jsonMessage) {
      if (jsonMessage != null) {
        this.id_chain = jsonMessage["id_chain"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
      }
    }
    cleanUp() {
      this.id_chain = [];
      this.size_x = 0;
      this.size_y = 0;
      this.transform = [];
    }
  };

  // src/classes/messages/text-update-patch.ts
  var TextUpdatePatch = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    fromPatch(jsonMessage, registeredFontFaces) {
      this.id_chain = jsonMessage["id_chain"];
      this.content = jsonMessage["content"];
      this.size_x = jsonMessage["size_x"];
      this.size_y = jsonMessage["size_y"];
      this.transform = jsonMessage["transform"];
      this.depth = jsonMessage["depth"];
      const styleMessage = jsonMessage["style"];
      if (styleMessage) {
        this.style = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
        this.style.build(styleMessage, registeredFontFaces);
      }
      const styleLinkMessage = jsonMessage["style_link"];
      if (styleLinkMessage) {
        this.style_link = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
        this.style_link.build(styleLinkMessage, registeredFontFaces);
      }
    }
    cleanUp() {
      this.id_chain = [];
      this.content = "";
      this.size_x = 0;
      this.size_y = 0;
      this.transform = [];
      this.objectManager.returnToPool(TEXT_STYLE, this.style);
      this.style = void 0;
      this.objectManager.returnToPool(TEXT_STYLE, this.style_link);
      this.style_link = void 0;
    }
  };

  // src/classes/messages/scroller-update-patch.ts
  var ScrollerUpdatePatch = class {
    fromPatch(jsonMessage) {
      this.idChain = jsonMessage["id_chain"];
      this.sizeX = jsonMessage["size_x"];
      this.sizeY = jsonMessage["size_y"];
      this.sizeInnerPaneX = jsonMessage["size_inner_pane_x"];
      this.sizeInnerPaneY = jsonMessage["size_inner_pane_y"];
      this.transform = jsonMessage["transform"];
      this.scrollX = jsonMessage["scroll_x"];
      this.scrollY = jsonMessage["scroll_y"];
      this.subtreeDepth = jsonMessage["subtree_depth"];
    }
    cleanUp() {
      this.idChain = [];
      this.sizeX = 0;
      this.sizeY = 0;
      this.sizeInnerPaneX = 0;
      this.sizeInnerPaneY = 0;
      this.transform = [];
      this.scrollX = false;
      this.scrollY = false;
      this.subtreeDepth = 0;
    }
  };

  // src/classes/messages/image-load-patch.ts
  var ImageLoadPatch = class {
    fromPatch(jsonMessage) {
      this.id_chain = jsonMessage["id_chain"];
      this.path = jsonMessage["path"];
    }
    cleanUp() {
      this.id_chain = [];
      this.path = "";
    }
  };

  // src/utils/constants.ts
  var NATIVE_OVERLAY_CLASS = "native-overlay";
  var CANVAS_CLASS = "canvas";
  var SCROLLER_CONTAINER = "scroller-container";
  var INNER_PANE = "inner-pane";
  var NATIVE_LEAF_CLASS = "native-leaf";

  // src/utils/helpers.ts
  async function readImageToByteBuffer(imagePath) {
    const response = await fetch(imagePath);
    const blob = await response.blob();
    const img = await createImageBitmap(blob);
    const canvas = new OffscreenCanvas(img.width + 1e3, img.height);
    const ctx = canvas.getContext("2d");
    ctx.drawImage(img, 0, 0, img.width, img.height);
    const imageData = ctx.getImageData(0, 0, img.width, img.height);
    let pixels = imageData.data;
    return { pixels, width: img.width, height: img.height };
  }
  function packAffineCoeffsIntoMatrix3DString(coeffs) {
    return "matrix3d(" + [
      //begin column 0
      coeffs[0].toFixed(6),
      coeffs[1].toFixed(6),
      0,
      0,
      //begin column 1
      coeffs[2].toFixed(6),
      coeffs[3].toFixed(6),
      0,
      0,
      //begin column 2
      0,
      0,
      1,
      0,
      //begin column 3
      coeffs[4].toFixed(6),
      coeffs[5].toFixed(6),
      0,
      1
    ].join(",") + ")";
  }
  function generateLocationId(scrollerId, zIndex) {
    if (scrollerId) {
      return `[${scrollerId.join(",")}]_${zIndex}`;
    } else {
      return `${zIndex}`;
    }
  }
  function arrayToKey(arr) {
    return arr.join(",");
  }

  // src/classes/layer.ts
  var Layer = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    build(parent, zIndex, scroller_id, chassis, canvasMap) {
      this.zIndex = zIndex;
      this.scrollerId = scroller_id;
      this.chassis = chassis;
      this.canvasMap = canvasMap;
      this.canvas = this.objectManager.getFromPool(CANVAS);
      this.native = this.objectManager.getFromPool(DIV);
      this.canvas.id = generateLocationId(scroller_id, zIndex);
      this.canvas.style.zIndex = String(zIndex);
      parent.appendChild(this.canvas);
      canvasMap.set(this.canvas.id, this.canvas);
      chassis.add_context(this.canvas.id);
      this.native.className = NATIVE_OVERLAY_CLASS;
      this.native.style.zIndex = String(zIndex);
      parent.appendChild(this.native);
      if (scroller_id != void 0) {
        this.canvas.style.position = "sticky";
        if (zIndex > 0) {
          this.canvas.style.marginTop = String(-this.canvas.style.height) + "px";
        }
        this.native.style.position = "sticky";
        this.native.style.marginTop = String(-this.canvas.style.height) + "px";
      }
    }
    cleanUp() {
      if (this.canvas != void 0 && this.chassis != void 0 && this.zIndex != void 0) {
        this.chassis.remove_context(generateLocationId(this.scrollerId, this.zIndex));
        this.canvasMap?.delete(this.canvas.id);
        let parent = this.canvas.parentElement;
        parent.removeChild(this.canvas);
        this.objectManager.returnToPool(CANVAS, this.canvas);
      }
      if (this.native != void 0) {
        let parent = this.native.parentElement;
        parent.removeChild(this.native);
        this.objectManager.returnToPool(DIV, this.native);
      }
      this.scrollerId = [];
      this.zIndex = void 0;
    }
    updateCanvas(width, height) {
      requestAnimationFrame(() => {
        if (this.scrollerId != void 0 && (this.zIndex != void 0 && this.zIndex > 0)) {
          if (this.canvas != void 0) {
            this.canvas.style.marginTop = String(-height) + "px";
          }
        }
      });
    }
    updateNativeOverlay(width, height) {
      requestAnimationFrame(() => {
        if (this.native != void 0) {
          if (this.scrollerId != void 0) {
            this.native.style.marginTop = String(-height) + "px";
          }
          this.native.style.width = String(width) + "px";
          this.native.style.height = String(height) + "px";
        }
      });
    }
  };

  // src/classes/occlusion-context.ts
  var OcclusionContext = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    build(parent, scrollerId, chassis, canvasMap) {
      this.layers = this.objectManager.getFromPool(ARRAY2);
      this.parent = parent;
      this.zIndex = -1;
      this.scrollerId = scrollerId;
      this.chassis = chassis;
      this.canvasMap = canvasMap;
      this.growTo(0);
    }
    growTo(z_index) {
      let zIndex = z_index + 1;
      if (this.parent == void 0 || this.canvasMap == void 0 || this.layers == void 0 || this.chassis == void 0) {
        return;
      }
      if (this.zIndex != void 0 && this.zIndex < zIndex) {
        for (let i = this.zIndex + 1; i <= zIndex; i++) {
          let newLayer = this.objectManager.getFromPool(LAYER, this.objectManager);
          newLayer.build(this.parent, i, this.scrollerId, this.chassis, this.canvasMap);
          this.layers.push(newLayer);
        }
        this.zIndex = zIndex;
      }
    }
    shrinkTo(zIndex) {
      if (this.layers == void 0) {
        return;
      }
      if (this.zIndex != void 0 && this.zIndex > zIndex) {
        for (let i = this.zIndex; i > zIndex; i--) {
          this.objectManager.returnToPool(LAYER, this.layers[i]);
          this.layers.pop();
        }
        this.zIndex = zIndex;
      }
    }
    addElement(element, zIndex) {
      if (this.zIndex != void 0) {
        this.growTo(zIndex);
        element.style.zIndex = String(zIndex);
        this.layers[zIndex].native.prepend(element);
      }
    }
    updateCanvases(width, height) {
      if (this.layers != void 0) {
        this.layers.forEach((layer) => {
          layer.updateCanvas(width, height);
        });
      }
    }
    cleanUp() {
      if (this.layers != void 0) {
        this.layers.forEach((layer) => {
          this.objectManager.returnToPool(LAYER, layer);
        });
      }
      this.canvasMap = void 0;
      this.parent = void 0;
      this.zIndex = void 0;
      this.scrollerId = void 0;
    }
    updateNativeOverlays(width, height) {
      if (this.layers != void 0) {
        this.layers.forEach((layer) => {
          layer.updateNativeOverlay(width, height);
        });
      }
    }
  };

  // ../node_modules/snarkdown/dist/snarkdown.es.js
  var e = { "": ["<em>", "</em>"], _: ["<strong>", "</strong>"], "*": ["<strong>", "</strong>"], "~": ["<s>", "</s>"], "\n": ["<br />"], " ": ["<br />"], "-": ["<hr />"] };
  function n(e2) {
    return e2.replace(RegExp("^" + (e2.match(/^(\t| )+/) || "")[0], "gm"), "");
  }
  function r(e2) {
    return (e2 + "").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }
  function t(a, c) {
    var o, l, g, s, p, u = /((?:^|\n+)(?:\n---+|\* \*(?: \*)+)\n)|(?:^``` *(\w*)\n([\s\S]*?)\n```$)|((?:(?:^|\n+)(?:\t|  {2,}).+)+\n*)|((?:(?:^|\n)([>*+-]|\d+\.)\s+.*)+)|(?:!\[([^\]]*?)\]\(([^)]+?)\))|(\[)|(\](?:\(([^)]+?)\))?)|(?:(?:^|\n+)([^\s].*)\n(-{3,}|={3,})(?:\n+|$))|(?:(?:^|\n+)(#{1,6})\s*(.+)(?:\n+|$))|(?:`([^`].*?)`)|(  \n\n*|\n{2,}|__|\*\*|[_*]|~~)/gm, m = [], h = "", i = c || {}, d = 0;
    function f(n2) {
      var r2 = e[n2[1] || ""], t2 = m[m.length - 1] == n2;
      return r2 ? r2[1] ? (t2 ? m.pop() : m.push(n2), r2[0 | t2]) : r2[0] : n2;
    }
    function $() {
      for (var e2 = ""; m.length; )
        e2 += f(m[m.length - 1]);
      return e2;
    }
    for (a = a.replace(/^\[(.+?)\]:\s*(.+)$/gm, function(e2, n2, r2) {
      return i[n2.toLowerCase()] = r2, "";
    }).replace(/^\n+|\n+$/g, ""); g = u.exec(a); )
      l = a.substring(d, g.index), d = u.lastIndex, o = g[0], l.match(/[^\\](\\\\)*\\$/) || ((p = g[3] || g[4]) ? o = '<pre class="code ' + (g[4] ? "poetry" : g[2].toLowerCase()) + '"><code' + (g[2] ? ' class="language-' + g[2].toLowerCase() + '"' : "") + ">" + n(r(p).replace(/^\n+|\n+$/g, "")) + "</code></pre>" : (p = g[6]) ? (p.match(/\./) && (g[5] = g[5].replace(/^\d+/gm, "")), s = t(n(g[5].replace(/^\s*[>*+.-]/gm, ""))), ">" == p ? p = "blockquote" : (p = p.match(/\./) ? "ol" : "ul", s = s.replace(/^(.*)(\n|$)/gm, "<li>$1</li>")), o = "<" + p + ">" + s + "</" + p + ">") : g[8] ? o = '<img src="' + r(g[8]) + '" alt="' + r(g[7]) + '">' : g[10] ? (h = h.replace("<a>", '<a href="' + r(g[11] || i[l.toLowerCase()]) + '">'), o = $() + "</a>") : g[9] ? o = "<a>" : g[12] || g[14] ? o = "<" + (p = "h" + (g[14] ? g[14].length : g[13] > "=" ? 1 : 2)) + ">" + t(g[12] || g[15], i) + "</" + p + ">" : g[16] ? o = "<code>" + r(g[16]) + "</code>" : (g[17] || g[1]) && (o = f(g[17] || "--"))), h += l, h += o;
    return (h + a.substring(d) + $()).replace(/^\n+|\n+$/g, "");
  }

  // src/classes/text.ts
  var FontStyle = /* @__PURE__ */ ((FontStyle2) => {
    FontStyle2[FontStyle2["Normal"] = 0] = "Normal";
    FontStyle2[FontStyle2["Italic"] = 1] = "Italic";
    FontStyle2[FontStyle2["Oblique"] = 2] = "Oblique";
    return FontStyle2;
  })(FontStyle || {});
  var FontWeight = /* @__PURE__ */ ((FontWeight2) => {
    FontWeight2[FontWeight2["Thin"] = 0] = "Thin";
    FontWeight2[FontWeight2["ExtraLight"] = 1] = "ExtraLight";
    FontWeight2[FontWeight2["Light"] = 2] = "Light";
    FontWeight2[FontWeight2["Normal"] = 3] = "Normal";
    FontWeight2[FontWeight2["Medium"] = 4] = "Medium";
    FontWeight2[FontWeight2["SemiBold"] = 5] = "SemiBold";
    FontWeight2[FontWeight2["Bold"] = 6] = "Bold";
    FontWeight2[FontWeight2["ExtraBold"] = 7] = "ExtraBold";
    FontWeight2[FontWeight2["Black"] = 8] = "Black";
    return FontWeight2;
  })(FontWeight || {});
  var Font = class {
    // for LocalFontMessage
    mapFontWeight(fontWeight) {
      switch (fontWeight) {
        case 0 /* Thin */:
          return 100;
        case 1 /* ExtraLight */:
          return 200;
        case 2 /* Light */:
          return 300;
        case 3 /* Normal */:
          return 400;
        case 4 /* Medium */:
          return 500;
        case 5 /* SemiBold */:
          return 600;
        case 6 /* Bold */:
          return 700;
        case 7 /* ExtraBold */:
          return 800;
        case 8 /* Black */:
          return 900;
        default:
          return 400;
      }
    }
    mapFontStyle(fontStyle) {
      switch (fontStyle) {
        case 0 /* Normal */:
          return "normal";
        case 1 /* Italic */:
          return "italic";
        case 2 /* Oblique */:
          return "oblique";
        default:
          return "normal";
      }
    }
    fromFontPatch(fontPatch, registeredFontFaces) {
      const type = Object.keys(fontPatch)[0];
      const data = fontPatch[type];
      this.type = type;
      if (type === "System") {
        this.family = data.family;
        this.style = FontStyle[data.style];
        this.weight = FontWeight[data.weight];
      } else if (type === "Web") {
        this.family = data.family;
        this.url = data.url;
        this.style = FontStyle[data.style];
        this.weight = FontWeight[data.weight];
      } else if (type === "Local") {
        this.family = data.family;
        this.path = data.path;
        this.style = FontStyle[data.style];
        this.weight = FontWeight[data.weight];
      }
      this.registerFontFace(registeredFontFaces);
    }
    cleanUp() {
      this.type = void 0;
      this.family = void 0;
      this.style = void 0;
      this.url = void 0;
      this.style = void 0;
      this.weight = void 0;
      this.path = void 0;
    }
    fontKey() {
      return `${this.type}-${this.family}-${this.style}-${this.weight}`;
    }
    registerFontFace(registeredFontFaces) {
      const fontKey = this.fontKey();
      if (!registeredFontFaces.has(fontKey)) {
        registeredFontFaces.add(fontKey);
        if (this.type === "Web" && this.url && this.family) {
          if (this.url.includes("fonts.googleapis.com/css")) {
            fetch(this.url).then((response) => response.text()).then((css) => {
              const style = document.createElement("style");
              style.textContent = css;
              document.head.appendChild(style);
            });
          } else {
            const fontFace = new FontFace(this.family, `url(${this.url})`, {
              style: this.style ? FontStyle[this.style] : void 0,
              weight: this.weight ? FontWeight[this.weight] : void 0
            });
            fontFace.load().then((loadedFontFace) => {
              document.fonts.add(loadedFontFace);
            });
          }
        } else if (this.type === "Local" && this.path && this.family) {
          const fontFace = new FontFace(this.family, `url(${this.path})`, {
            style: this.style ? FontStyle[this.style] : void 0,
            weight: this.weight ? FontWeight[this.weight] : void 0
          });
          fontFace.load().then((loadedFontFace) => {
            document.fonts.add(loadedFontFace);
          });
        }
      }
    }
    applyFontToDiv(div) {
      if (this.family != void 0) {
        div.style.fontFamily = this.family;
      }
      if (this.style != void 0) {
        div.style.fontStyle = this.mapFontStyle(this.style);
      }
      if (this.weight != void 0) {
        div.style.fontWeight = String(this.mapFontWeight(this.weight));
      }
    }
  };
  var TextStyle = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    build(styleMessage, registeredFontFaces) {
      if (styleMessage["font"]) {
        const font = this.objectManager.getFromPool(FONT);
        font.fromFontPatch(styleMessage["font"], registeredFontFaces);
        this.font = font;
      }
      this.fill = styleMessage["fill"];
      this.font_size = styleMessage["font_size"];
      this.underline = styleMessage["underline"];
      this.align_multiline = styleMessage["align_multiline"];
      this.align_horizontal = styleMessage["align_horizontal"];
      this.align_vertical = styleMessage["align_vertical"];
    }
    cleanUp() {
      if (this.font) {
        this.objectManager.returnToPool(FONT, this.font);
        this.font = void 0;
      }
      this.fill = void 0;
      this.font_size = void 0;
      this.underline = void 0;
      this.align_multiline = void 0;
      this.align_horizontal = void 0;
      this.align_vertical = void 0;
    }
  };
  function getJustifyContent(horizontalAlignment) {
    switch (horizontalAlignment) {
      case "Left" /* Left */:
        return "flex-start";
      case "Center" /* Center */:
        return "center";
      case "Right" /* Right */:
        return "flex-end";
      default:
        return "flex-start";
    }
  }
  function getTextAlign(paragraphAlignment) {
    switch (paragraphAlignment) {
      case "Left" /* Left */:
        return "left";
      case "Center" /* Center */:
        return "center";
      case "Right" /* Right */:
        return "right";
      default:
        return "left";
    }
  }
  function getAlignItems(verticalAlignment) {
    switch (verticalAlignment) {
      case "Top" /* Top */:
        return "flex-start";
      case "Center" /* Center */:
        return "center";
      case "Bottom" /* Bottom */:
        return "flex-end";
      default:
        return "flex-start";
    }
  }

  // src/classes/native-element-pool.ts
  var NativeElementPool = class _NativeElementPool {
    constructor(objectManager2) {
      this.textNodes = {};
      this.messageList = [];
      this.isMobile = false;
      this.objectManager = objectManager2;
      this.canvases = /* @__PURE__ */ new Map();
      this.scrollers = /* @__PURE__ */ new Map();
      this.baseOcclusionContext = objectManager2.getFromPool(OCCLUSION_CONTEXT, objectManager2);
      this.registeredFontFaces = /* @__PURE__ */ new Set();
    }
    build(chassis, isMobile2, mount2) {
      this.isMobile = isMobile2;
      this.chassis = chassis;
      this.baseOcclusionContext.build(mount2, void 0, chassis, this.canvases);
    }
    static addNativeElement(elem, baseOcclusionContext, scrollers, idChain, scrollerIdChain, zIndex) {
      if (scrollerIdChain != void 0) {
        let scroller = scrollers.get(arrayToKey(scrollerIdChain));
        scroller.addElement(elem, zIndex);
      } else {
        baseOcclusionContext.addElement(elem, zIndex);
      }
    }
    clearCanvases() {
      this.canvases.forEach((canvas, key) => {
        let dpr = window.devicePixelRatio;
        const context = canvas.getContext("2d");
        if (context) {
          context.clearRect(0, 0, canvas.width, canvas.height);
        }
        if (canvas.width != canvas.clientWidth * dpr || canvas.height != canvas.clientHeight * dpr) {
          canvas.width = canvas.clientWidth * dpr;
          canvas.height = canvas.clientHeight * dpr;
          if (context) {
            context.scale(dpr, dpr);
          }
        }
      });
    }
    sendScrollerValues() {
      this.scrollers.forEach((scroller, id) => {
        let deltaX = 0;
        let deltaY = scroller.getTickScrollDelta();
        if (deltaY && Math.abs(deltaY) > 0) {
          const scrollEvent = this.objectManager.getFromPool(OBJECT);
          const deltas = this.objectManager.getFromPool(OBJECT);
          deltas["delta_x"] = deltaX;
          deltas["delta_y"] = deltaY;
          scrollEvent.Scroll = deltas;
          const scrollEventStringified = JSON.stringify(scrollEvent);
          this.messageList.push(scrollEventStringified);
          this.chassis.interrupt(scrollEventStringified, []);
          this.objectManager.returnToPool(OBJECT, deltas);
          this.objectManager.returnToPool(OBJECT, scrollEvent);
        }
      });
    }
    occlusionUpdate(patch) {
      let node = this.textNodes[patch.idChain];
      if (node) {
        let parent = node.parentElement;
        parent.removeChild(node);
        _NativeElementPool.addNativeElement(
          node,
          this.baseOcclusionContext,
          // @ts-ignore
          this.scrollers,
          patch.idChain,
          void 0,
          patch.zIndex
        );
      }
    }
    checkboxCreate(patch) {
      console.assert(patch.idChain != null);
      console.assert(patch.clippingIds != null);
      console.assert(patch.scrollerIds != null);
      console.assert(patch.zIndex != null);
      const checkbox = this.objectManager.getFromPool(INPUT);
      checkbox.type = "checkbox";
      checkbox.style.margin = "0";
      checkbox.addEventListener("change", (event) => {
        const is_checked = event.target.checked;
        checkbox.checked = !is_checked;
        let message = {
          "FormCheckboxToggle": {
            "id_chain": patch.idChain,
            "state": checkbox.checked
          }
        };
        this.chassis.interrupt(JSON.stringify(message), void 0);
      });
      let runningChain = this.objectManager.getFromPool(DIV);
      runningChain.appendChild(checkbox);
      runningChain.setAttribute("class", NATIVE_LEAF_CLASS);
      runningChain.setAttribute("id_chain", String(patch.idChain));
      let scroller_id;
      if (patch.scrollerIds != null) {
        let length = patch.scrollerIds.length;
        if (length != 0) {
          scroller_id = patch.scrollerIds[length - 1];
        }
      }
      if (patch.idChain != void 0 && patch.zIndex != void 0) {
        _NativeElementPool.addNativeElement(
          runningChain,
          this.baseOcclusionContext,
          this.scrollers,
          patch.idChain,
          scroller_id,
          patch.zIndex
        );
      }
      this.textNodes[patch.idChain] = runningChain;
    }
    checkboxUpdate(patch) {
      window.textNodes = this.textNodes;
      let leaf = this.textNodes[patch.id_chain];
      console.assert(leaf !== void 0);
      let checkbox = leaf.firstChild;
      if (patch.checked !== null) {
        checkbox.checked = patch.checked;
      }
      if (patch.size_x != null) {
        checkbox.style.width = patch.size_x - 1 + "px";
      }
      if (patch.size_y != null) {
        checkbox.style.height = patch.size_y + "px";
      }
      if (patch.transform != null) {
        leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
      }
    }
    checkboxDelete(id_chain) {
      let oldNode = this.textNodes[id_chain];
      if (oldNode) {
        let parent = oldNode.parentElement;
        parent.removeChild(oldNode);
      }
    }
    buttonCreate(patch) {
      console.assert(patch.idChain != null);
      console.assert(patch.clippingIds != null);
      console.assert(patch.scrollerIds != null);
      console.assert(patch.zIndex != null);
      const button = this.objectManager.getFromPool(BUTTON);
      const textContainer = this.objectManager.getFromPool(DIV);
      const textChild = this.objectManager.getFromPool(DIV);
      button.style.margin = "0";
      button.style.padding = "0";
      textContainer.style.margin = "0";
      textContainer.style.display = "flex";
      textContainer.style.width = "100%";
      textContainer.style.height = "100%";
      textChild.style.margin = "0";
      button.addEventListener("click", (event) => {
        console.log("button click!");
        let message = {
          "FormButtonClick": {
            "id_chain": patch.idChain
          }
        };
        this.chassis.interrupt(JSON.stringify(message), void 0);
      });
      let runningChain = this.objectManager.getFromPool(DIV);
      textContainer.appendChild(textChild);
      button.appendChild(textContainer);
      runningChain.appendChild(button);
      runningChain.setAttribute("class", NATIVE_LEAF_CLASS);
      runningChain.setAttribute("id_chain", String(patch.idChain));
      let scroller_id;
      if (patch.scrollerIds != null) {
        let length = patch.scrollerIds.length;
        if (length != 0) {
          scroller_id = patch.scrollerIds[length - 1];
        }
      }
      if (patch.idChain != void 0 && patch.zIndex != void 0) {
        _NativeElementPool.addNativeElement(
          runningChain,
          this.baseOcclusionContext,
          this.scrollers,
          patch.idChain,
          scroller_id,
          patch.zIndex
        );
      }
      this.textNodes[patch.idChain] = runningChain;
    }
    buttonUpdate(patch) {
      window.textNodes = this.textNodes;
      let leaf = this.textNodes[patch.id_chain];
      console.assert(leaf !== void 0);
      let button = leaf.firstChild;
      let textContainer = button.firstChild;
      let textChild = textContainer.firstChild;
      if (patch.content != null) {
        textChild.innerHTML = t(patch.content);
      }
      if (patch.style) {
        const style = patch.style;
        if (style.font) {
          style.font.applyFontToDiv(textContainer);
        }
        if (style.fill) {
          let newValue = "";
          if (style.fill.Rgba != null) {
            let p = style.fill.Rgba;
            newValue = `rgba(${p[0] * 255},${p[1] * 255},${p[2] * 255},${p[3] * 255})`;
          } else if (style.fill.Hsla != null) {
            let p = style.fill.Hsla;
            newValue = `hsla(${p[0] * 255},${p[1] * 255},${p[2] * 255},${p[3] * 255})`;
          } else if (style.fill.Rgb != null) {
            let p = style.fill.Rgb;
            newValue = `rgb(${p[0] * 255},${p[1] * 255},${p[2] * 255})`;
          } else if (style.fill.Hsl != null) {
            let p = style.fill.Hsl;
            newValue = `hsl(${p[0] * 255},${p[1] * 255},${p[2] * 255})`;
          } else {
            throw new TypeError("Unsupported Color Format");
          }
          textChild.style.color = newValue;
        }
        if (style.font_size) {
          textChild.style.fontSize = style.font_size + "px";
        }
        if (style.underline != null) {
          textChild.style.textDecoration = style.underline ? "underline" : "none";
        }
        if (style.align_horizontal) {
          leaf.style.display = "flex";
          textContainer.style.justifyContent = getJustifyContent(style.align_horizontal);
        }
        if (style.align_vertical) {
          textContainer.style.alignItems = getAlignItems(style.align_vertical);
        }
        if (style.align_multiline) {
          textChild.style.textAlign = getTextAlign(style.align_multiline);
        }
      }
      if (patch.size_x != null) {
        button.style.width = patch.size_x - 1 + "px";
      }
      if (patch.size_y != null) {
        button.style.height = patch.size_y + "px";
      }
      if (patch.transform != null) {
        leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
      }
    }
    buttonDelete(id_chain) {
      let oldNode = this.textNodes[id_chain];
      if (oldNode) {
        let parent = oldNode.parentElement;
        parent.removeChild(oldNode);
      }
    }
    textCreate(patch) {
      console.assert(patch.idChain != null);
      console.assert(patch.clippingIds != null);
      console.assert(patch.scrollerIds != null);
      console.assert(patch.zIndex != null);
      let runningChain = this.objectManager.getFromPool(DIV);
      let textChild = this.objectManager.getFromPool(DIV);
      runningChain.appendChild(textChild);
      runningChain.setAttribute("class", NATIVE_LEAF_CLASS);
      runningChain.setAttribute("id_chain", String(patch.idChain));
      let scroller_id;
      if (patch.scrollerIds != null) {
        let length = patch.scrollerIds.length;
        if (length != 0) {
          scroller_id = patch.scrollerIds[length - 1];
        }
      }
      if (patch.idChain != void 0 && patch.zIndex != void 0) {
        _NativeElementPool.addNativeElement(
          runningChain,
          this.baseOcclusionContext,
          this.scrollers,
          patch.idChain,
          scroller_id,
          patch.zIndex
        );
      }
      this.textNodes[patch.idChain] = runningChain;
    }
    textUpdate(patch) {
      window.textNodes = this.textNodes;
      let leaf = this.textNodes[patch.id_chain];
      console.assert(leaf !== void 0);
      let textChild = leaf.firstChild;
      if (patch.style) {
        const style = patch.style;
        if (style.font) {
          style.font.applyFontToDiv(leaf);
        }
        if (style.fill) {
          let newValue = "";
          if (style.fill.Rgba != null) {
            let p = style.fill.Rgba;
            newValue = `rgba(${p[0] * 255},${p[1] * 255},${p[2] * 255},${p[3] * 255})`;
          } else if (style.fill.Hsla != null) {
            let p = style.fill.Hsla;
            newValue = `hsla(${p[0] * 255},${p[1] * 255},${p[2] * 255},${p[3] * 255})`;
          } else if (style.fill.Rgb != null) {
            let p = style.fill.Rgb;
            newValue = `rgb(${p[0] * 255},${p[1] * 255},${p[2] * 255})`;
          } else if (style.fill.Hsl != null) {
            let p = style.fill.Hsl;
            newValue = `hsl(${p[0] * 255},${p[1] * 255},${p[2] * 255})`;
          } else {
            throw new TypeError("Unsupported Color Format");
          }
          textChild.style.color = newValue;
        }
        if (style.font_size) {
          textChild.style.fontSize = style.font_size + "px";
        }
        if (style.underline != null) {
          textChild.style.textDecoration = style.underline ? "underline" : "none";
        }
        if (style.align_horizontal) {
          leaf.style.display = "flex";
          leaf.style.justifyContent = getJustifyContent(style.align_horizontal);
        }
        if (style.align_vertical) {
          leaf.style.alignItems = getAlignItems(style.align_vertical);
        }
        if (style.align_multiline) {
          textChild.style.textAlign = getTextAlign(style.align_multiline);
        }
      }
      if (patch.content != null) {
        textChild.innerHTML = t(patch.content);
        if (patch.style_link) {
          let linkStyle = patch.style_link;
          const links = textChild.querySelectorAll("a");
          links.forEach((link) => {
            if (linkStyle.font) {
              linkStyle.font.applyFontToDiv(link);
            }
            if (linkStyle.fill) {
              let newValue = "";
              if (linkStyle.fill.Rgba != null) {
                let p = linkStyle.fill.Rgba;
                newValue = `rgba(${p[0] * 255},${p[1] * 255},${p[2] * 255},${p[3] * 255})`;
              } else {
                let p = linkStyle.fill.Hsla;
                newValue = `hsla(${p[0] * 255},${p[1] * 255},${p[2] * 255},${p[3] * 255})`;
              }
              link.style.color = newValue;
            }
            if (linkStyle.align_horizontal) {
              leaf.style.display = "flex";
              leaf.style.justifyContent = getJustifyContent(linkStyle.align_horizontal);
            }
            if (linkStyle.font_size) {
              textChild.style.fontSize = linkStyle.font_size + "px";
            }
            if (linkStyle.align_vertical) {
              leaf.style.alignItems = getAlignItems(linkStyle.align_vertical);
            }
            if (linkStyle.align_multiline) {
              textChild.style.textAlign = getTextAlign(linkStyle.align_multiline);
            }
            if (linkStyle.underline != null) {
              link.style.textDecoration = linkStyle.underline ? "underline" : "none";
            }
          });
        }
      }
      if (patch.size_x != null) {
        leaf.style.width = patch.size_x + "px";
      }
      if (patch.size_y != null) {
        leaf.style.height = patch.size_y + "px";
      }
      if (patch.transform != null) {
        leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
      }
    }
    textDelete(id_chain) {
      let oldNode = this.textNodes[id_chain];
      if (oldNode) {
        let parent = oldNode.parentElement;
        parent.removeChild(oldNode);
      }
    }
    frameCreate(_patch) {
    }
    frameUpdate(_patch) {
    }
    frameDelete(_id_chain) {
    }
    scrollerCreate(patch, _chassis) {
      let scroller_id;
      if (patch.scrollerIds != null) {
        let length = patch.scrollerIds.length;
        if (length != 0) {
          scroller_id = patch.scrollerIds[length - 1];
        }
      }
      let scroller = this.objectManager.getFromPool(SCROLLER, this.objectManager);
      scroller.build(
        patch.idChain,
        patch.zIndex,
        scroller_id,
        this.chassis,
        this.scrollers,
        this.baseOcclusionContext,
        this.canvases,
        this.isMobile
      );
      this.scrollers.set(arrayToKey(patch.idChain), scroller);
    }
    scrollerUpdate(patch) {
      this.scrollers.get(arrayToKey(patch.idChain)).handleScrollerUpdate(patch);
    }
    scrollerDelete(idChain) {
      if (this.scrollers.has(arrayToKey(idChain))) {
        this.objectManager.returnToPool(SCROLLER, this.scrollers.get(arrayToKey(idChain)));
        this.scrollers.delete(arrayToKey(idChain));
      }
    }
    async imageLoad(patch, chassis) {
      function getScriptBasePath(scriptName) {
        const scripts = document.getElementsByTagName("script");
        for (let i = 0; i < scripts.length; i++) {
          if (scripts[i].src.indexOf(scriptName) > -1) {
            const path2 = new URL(scripts[i].src).pathname;
            return path2.replace(scriptName, "");
          }
        }
        return "/";
      }
      const BASE_PATH = getScriptBasePath("pax-chassis-web-interface.js");
      let path = (BASE_PATH + patch.path).replace("//", "/");
      let image_data = await readImageToByteBuffer(path);
      let message = {
        "Image": {
          "Data": {
            "id_chain": patch.id_chain,
            "width": image_data.width,
            "height": image_data.height
          }
        }
      };
      chassis.interrupt(JSON.stringify(message), image_data.pixels);
    }
  };

  // src/classes/scroll-manager.ts
  var ScrollManager = class {
    constructor(parent, isMobile2) {
      this.lastInterruptScrollTop = 0;
      this.lastScrollTop = 0;
      this.predicting = false;
      this.count = 0;
      this.stopped = false;
      this.scrollContainer = parent;
      this.interpolator = new HermiteInterpolator();
      if (isMobile2) {
        setInterval(() => {
          let currentScrollTop = this.scrollContainer.scrollTop;
          this.interpolator.update(Date.now(), currentScrollTop);
          if (currentScrollTop == this.lastScrollTop) {
            this.count++;
          } else {
            this.count = 0;
            this.stopped = false;
          }
          if (this.count > 3) {
            this.stopped = true;
          }
          this.lastScrollTop = currentScrollTop;
        }, 1);
        this.scrollContainer.addEventListener("touchstart", () => {
          this.predicting = false;
        });
        this.scrollContainer.addEventListener("touchend", () => {
          this.predicting = true;
        });
      } else {
        setInterval(() => {
          this.lastScrollTop = this.scrollContainer.scrollTop;
        }, 0);
      }
    }
    getScrollDelta() {
      let ret;
      if (!this.predicting || this.stopped) {
        ret = this.lastScrollTop - this.lastInterruptScrollTop;
        this.lastInterruptScrollTop = this.lastScrollTop;
      } else {
        const predictedScrollTop = this.interpolator.predict(Date.now());
        ret = predictedScrollTop - this.lastInterruptScrollTop;
        this.lastInterruptScrollTop = predictedScrollTop;
      }
      return ret;
    }
  };
  var HermiteInterpolator = class {
    constructor() {
      this.buffer = [];
      this.initialTimestamp = null;
    }
    normalizeTimestamp(timestamp) {
      if (this.initialTimestamp === null) {
        this.initialTimestamp = timestamp;
      }
      return timestamp - this.initialTimestamp;
    }
    update(actualTimestamp, position) {
      const timestamp = this.normalizeTimestamp(actualTimestamp);
      if (this.buffer.length === 100) {
        this.buffer.shift();
      }
      let velocity = 0;
      if (this.buffer.length === 2) {
        const prevDelta = position - this.buffer[1].position;
        const prevTime = timestamp - this.buffer[1].timestamp;
        const earlierDelta = this.buffer[1].position - this.buffer[0].position;
        const earlierTime = this.buffer[1].timestamp - this.buffer[0].timestamp;
        velocity = 0.5 * (prevDelta / prevTime + earlierDelta / earlierTime);
      } else if (this.buffer.length === 1) {
        velocity = (position - this.buffer[0].position) / (timestamp - this.buffer[0].timestamp);
      }
      this.buffer.push({ timestamp, position, velocity });
    }
    predict(actualTimestamp) {
      if (this.buffer.length < 2) {
        return this.buffer.length === 1 ? this.buffer[0].position : 0;
      }
      const timestamp = this.normalizeTimestamp(actualTimestamp);
      const t0 = this.buffer[this.buffer.length - 2].timestamp;
      const t1 = this.buffer[this.buffer.length - 1].timestamp;
      const y0 = this.buffer[this.buffer.length - 2].position;
      const y1 = this.buffer[this.buffer.length - 1].position;
      const m = (timestamp - t0) / (t1 - t0);
      const h00 = 2 * m * m * m - 3 * m * m + 1;
      const h10 = m * m * m - 2 * m * m + m;
      const h01 = -2 * m * m * m + 3 * m * m;
      const h11 = m * m * m - m * m;
      const predictedPosition = h00 * y0 + h10 * (t1 - t0) * this.buffer[this.buffer.length - 2].velocity + h01 * y1 + h11 * (t1 - t0) * this.buffer[this.buffer.length - 1].velocity;
      return predictedPosition;
    }
  };

  // src/classes/scroller.ts
  var Scroller = class {
    constructor(objectManager2) {
      this.unsentX = 0;
      this.unsentY = 0;
      this.isMobile = false;
      this.objectManager = objectManager2;
    }
    build(idChain, zIndex, scrollerId, chassis, scrollers, baseOcclusionContext, canvasMap, isMobile2) {
      this.isMobile = isMobile2;
      this.idChain = idChain;
      this.parentScrollerId = scrollerId;
      this.zIndex = zIndex;
      this.scrollOffsetX = 0;
      this.scrollOffsetY = 0;
      this.sizeX = 0;
      this.sizeY = 0;
      this.sizeInnerPaneX = 0;
      this.sizeInnerPaneY = 0;
      this.container = this.objectManager.getFromPool(DIV);
      this.container.className = SCROLLER_CONTAINER;
      NativeElementPool.addNativeElement(this.container, baseOcclusionContext, scrollers, idChain, scrollerId, zIndex);
      this.scrollManager = new ScrollManager(this.container, isMobile2);
      this.innerPane = this.objectManager.getFromPool(DIV);
      this.innerPane.className = INNER_PANE;
      this.container.appendChild(this.innerPane);
      this.occlusionContext = this.objectManager.getFromPool(OCCLUSION_CONTEXT, this.objectManager);
      this.occlusionContext.build(this.container, idChain, chassis, canvasMap);
    }
    getTickScrollDelta() {
      return this.scrollManager?.getScrollDelta();
    }
    cleanUp() {
      if (this.occlusionContext != void 0) {
        this.occlusionContext.cleanUp();
        this.occlusionContext = void 0;
      }
      if (this.innerPane != void 0) {
        this.objectManager.returnToPool(DIV, this.innerPane);
        this.innerPane = void 0;
      }
      if (this.container != void 0) {
        let parent = this.container.parentElement;
        parent?.removeChild(this.container);
        this.objectManager.returnToPool(DIV, this.container);
        this.container = void 0;
      }
      this.idChain = void 0;
      this.parentScrollerId = void 0;
      this.zIndex = void 0;
      this.sizeX = void 0;
      this.sizeY = void 0;
      this.sizeInnerPaneX = void 0;
      this.sizeInnerPaneY = void 0;
      this.transform = void 0;
      this.scrollX = void 0;
      this.scrollY = void 0;
      this.scrollOffsetX = void 0;
      this.scrollOffsetY = void 0;
      this.subtreeDepth = void 0;
    }
    handleScrollerUpdate(msg) {
      if (this.container != void 0 && this.occlusionContext != void 0 && this.innerPane != void 0) {
        if (msg.sizeX != null) {
          this.sizeX = msg.sizeX;
          this.container.style.width = msg.sizeX + "px";
        }
        if (msg.sizeY != null) {
          this.sizeY = msg.sizeY;
          this.container.style.height = msg.sizeY + "px";
        }
        if (msg.sizeInnerPaneX != null) {
          this.sizeInnerPaneX = msg.sizeInnerPaneX;
        }
        if (msg.sizeInnerPaneY != null) {
          this.sizeInnerPaneY = msg.sizeInnerPaneY;
        }
        if (msg.scrollX != null) {
          this.scrollX = msg.scrollX;
          if (!msg.scrollX) {
            this.container.style.overflowX = "hidden";
          }
        }
        if (msg.scrollY != null) {
          this.scrollY = msg.scrollY;
          if (!msg.scrollY) {
            this.container.style.overflowY = "hidden";
          }
        }
        if (msg.subtreeDepth != null) {
          this.subtreeDepth = msg.subtreeDepth;
          this.occlusionContext.shrinkTo(msg.subtreeDepth);
        }
        if (msg.transform != null) {
          this.container.style.transform = packAffineCoeffsIntoMatrix3DString(msg.transform);
          this.transform = msg.transform;
        }
        if (msg.sizeX != null || msg.sizeY != null) {
          this.occlusionContext.updateCanvases(this.sizeX, this.sizeY);
          this.occlusionContext.updateNativeOverlays(this.sizeX, this.sizeY);
        }
        if (msg.sizeInnerPaneX != null || msg.sizeInnerPaneY != null) {
          this.innerPane.style.width = String(this.sizeInnerPaneX) + "px";
          this.innerPane.style.height = String(this.sizeInnerPaneY) + "px";
        }
      }
    }
    addElement(elem, zIndex) {
      if (this.occlusionContext != void 0) {
        this.occlusionContext.addElement(elem, zIndex);
      }
    }
  };

  // src/classes/messages/checkbox-update-patch.ts
  var CheckboxUpdatePatch = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    fromPatch(jsonMessage) {
      this.id_chain = jsonMessage["id_chain"];
      this.size_x = jsonMessage["size_x"];
      this.size_y = jsonMessage["size_y"];
      this.transform = jsonMessage["transform"];
      this.checked = jsonMessage["checked"];
    }
    cleanUp() {
      this.id_chain = [];
      this.size_x = 0;
      this.size_y = 0;
      this.transform = [];
      this.checked = void 0;
    }
  };

  // src/classes/messages/occlusion-update-patch.ts
  var OcclusionUpdatePatch = class {
    fromPatch(jsonMessage) {
      this.idChain = jsonMessage["id_chain"];
      this.zIndex = jsonMessage["z_index"];
    }
    cleanUp() {
      this.idChain = [];
      this.zIndex = -1;
    }
  };

  // src/classes/messages/button-update-patch.ts
  var ButtonUpdatePatch = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    fromPatch(jsonMessage, registeredFontFaces) {
      this.id_chain = jsonMessage["id_chain"];
      this.content = jsonMessage["content"];
      this.size_x = jsonMessage["size_x"];
      this.size_y = jsonMessage["size_y"];
      this.transform = jsonMessage["transform"];
      const styleMessage = jsonMessage["style"];
      if (styleMessage) {
        this.style = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
        this.style.build(styleMessage, registeredFontFaces);
      }
    }
    cleanUp() {
      this.id_chain = [];
      this.size_x = 0;
      this.size_y = 0;
      this.transform = [];
      this.objectManager.returnToPool(TEXT_STYLE, this.style);
      this.style = void 0;
    }
  };

  // src/pools/supported-objects.ts
  var OBJECT = "Object";
  var ARRAY2 = "Array";
  var DIV = "DIV";
  var INPUT = "Input";
  var BUTTON = "Button";
  var CANVAS = "Canvas";
  var ANY_CREATE_PATCH = "Any Create Patch";
  var OCCLUSION_UPDATE_PATCH = "Occlusion Update Patch";
  var FRAME_UPDATE_PATCH = "Frame Update Patch";
  var IMAGE_LOAD_PATCH = "IMAGE LOAD PATCH";
  var SCROLLER_UPDATE_PATCH = "Scroller Update Patch";
  var TEXT_UPDATE_PATCH = "Text Update Patch";
  var CHECKBOX_UPDATE_PATCH = "Checkbox Update Patch";
  var BUTTON_UPDATE_PATCH = "Button Update Patch";
  var LAYER = "LAYER";
  var OCCLUSION_CONTEXT = "Occlusion Context";
  var SCROLLER = "Scroller";
  var FONT = "Font";
  var TEXT_STYLE = "Text Style";
  var UINT32ARRAY2 = "UInt32Array";
  var SUPPORTED_OBJECTS = [
    {
      name: OBJECT,
      factory: () => ({}),
      cleanUp: (obj) => {
        for (let prop in obj) {
          if (obj.hasOwnProperty(prop)) {
            delete obj[prop];
          }
        }
      }
    },
    {
      name: INPUT,
      factory: () => document.createElement("input"),
      cleanUp: (input) => {
        input.removeAttribute("style");
        input.innerHTML = "";
      }
    },
    {
      name: BUTTON,
      factory: () => document.createElement("button"),
      cleanUp: (input) => {
        input.removeAttribute("style");
        input.innerHTML = "";
      }
    },
    {
      name: ARRAY2,
      factory: () => [],
      cleanUp: (arr) => {
        arr.length = 0;
      }
    },
    {
      name: DIV,
      factory: () => document.createElement("div"),
      cleanUp: (div) => {
        div.removeAttribute("style");
        div.innerHTML = "";
      }
    },
    {
      name: CANVAS,
      factory: () => {
        let canvas = document.createElement("canvas");
        canvas.className = CANVAS_CLASS;
        return canvas;
      },
      cleanUp: (canvas) => {
        let ctx = canvas.getContext("2d");
        ctx && ctx.clearRect(0, 0, canvas.width, canvas.height);
        canvas.width = 0;
        canvas.height = 0;
        canvas.id = "";
        canvas.removeAttribute("style");
      }
    },
    {
      name: ANY_CREATE_PATCH,
      factory: () => new AnyCreatePatch(),
      cleanUp: (patch) => {
        patch.cleanUp();
      }
    },
    {
      name: OCCLUSION_UPDATE_PATCH,
      factory: () => new OcclusionUpdatePatch(),
      cleanUp: (patch) => {
        patch.cleanUp();
      }
    },
    {
      name: FRAME_UPDATE_PATCH,
      factory: () => new FrameUpdatePatch(),
      cleanUp: (patch) => {
        patch.cleanUp();
      }
    },
    {
      name: TEXT_UPDATE_PATCH,
      factory: (objectManager2) => new TextUpdatePatch(objectManager2),
      cleanUp: (patch) => {
        patch.cleanUp();
      }
    },
    {
      name: CHECKBOX_UPDATE_PATCH,
      factory: (objectManager2) => new CheckboxUpdatePatch(objectManager2),
      cleanUp: (patch) => {
        patch.cleanUp();
      }
    },
    {
      name: BUTTON_UPDATE_PATCH,
      factory: (objectManager2) => new ButtonUpdatePatch(objectManager2),
      cleanUp: (patch) => {
        patch.cleanUp();
      }
    },
    {
      name: IMAGE_LOAD_PATCH,
      factory: () => new ImageLoadPatch(),
      cleanUp: (patch) => {
        patch.cleanUp();
      }
    },
    {
      name: SCROLLER_UPDATE_PATCH,
      factory: () => new ScrollerUpdatePatch(),
      cleanUp: (patch) => {
        patch.cleanUp();
      }
    },
    {
      name: LAYER,
      factory: (objectManager2) => new Layer(objectManager2),
      cleanUp: (layer) => {
        layer.cleanUp();
      }
    },
    {
      name: OCCLUSION_CONTEXT,
      factory: (objectManager2) => new OcclusionContext(objectManager2),
      cleanUp: (oc) => {
        oc.cleanUp();
      }
    },
    {
      name: SCROLLER,
      factory: (objectManager2) => new Scroller(objectManager2),
      cleanUp: (oc) => {
        oc.cleanUp();
      }
    },
    {
      name: FONT,
      factory: () => new Font(),
      cleanUp: (font) => {
        font.cleanUp();
      }
    },
    {
      name: TEXT_STYLE,
      factory: (objectManager2) => new TextStyle(objectManager2),
      cleanUp: (ts) => {
        ts.cleanUp();
      }
    },
    {
      name: UINT32ARRAY2,
      factory: () => new Uint32Array(128),
      cleanUp: (array) => {
        array.fill(0);
      }
    }
  ];

  // src/events/listeners.ts
  function convertModifiers(event) {
    let modifiers = [];
    if (event.shiftKey)
      modifiers.push("Shift");
    if (event.ctrlKey)
      modifiers.push("Control");
    if (event.altKey)
      modifiers.push("Alt");
    if (event.metaKey)
      modifiers.push("Command");
    return modifiers;
  }
  function getMouseButton(event) {
    switch (event.button) {
      case 0:
        return "Left";
      case 1:
        return "Middle";
      case 2:
        return "Right";
      default:
        return "Unknown";
    }
  }
  function setupEventListeners(chassis, layer) {
    let lastPositions = /* @__PURE__ */ new Map();
    function getTouchMessages(touchList) {
      return Array.from(touchList).map((touch) => {
        let lastPosition = lastPositions.get(touch.identifier) || { x: touch.clientX, y: touch.clientY };
        let delta_x = touch.clientX - lastPosition.x;
        let delta_y = touch.clientY - lastPosition.y;
        lastPositions.set(touch.identifier, { x: touch.clientX, y: touch.clientY });
        return {
          x: touch.clientX,
          y: touch.clientY,
          identifier: touch.identifier,
          delta_x,
          delta_y
        };
      });
    }
    layer.addEventListener("click", (evt) => {
      let clickEvent = {
        "Click": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      chassis.interrupt(JSON.stringify(clickEvent), []);
      let clapEvent = {
        "Clap": {
          "x": evt.clientX,
          "y": evt.clientY
        }
      };
      chassis.interrupt(JSON.stringify(clapEvent), []);
    }, true);
    layer.addEventListener("dblclick", (evt) => {
      let event = {
        "DoubleClick": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, true);
    layer.addEventListener("mousemove", (evt) => {
      let event = {
        "MouseMove": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, true);
    layer.addEventListener("wheel", (evt) => {
      let event = {
        "Wheel": {
          "x": evt.clientX,
          "y": evt.clientY,
          "delta_x": evt.deltaX,
          "delta_y": evt.deltaY,
          "modifiers": convertModifiers(evt)
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, { "passive": true, "capture": true });
    layer.addEventListener("mousedown", (evt) => {
      let event = {
        "MouseDown": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, true);
    layer.addEventListener("mouseup", (evt) => {
      let event = {
        "MouseUp": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, true);
    layer.addEventListener("mouseover", (evt) => {
      let event = {
        "MouseOver": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, true);
    layer.addEventListener("mouseout", (evt) => {
      let event = {
        "MouseOut": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, true);
    layer.addEventListener("contextmenu", (evt) => {
      let event = {
        "ContextMenu": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, true);
    layer.addEventListener("touchstart", (evt) => {
      let event = {
        "TouchStart": {
          "touches": getTouchMessages(evt.touches)
        }
      };
      Array.from(evt.changedTouches).forEach((touch) => {
        lastPositions.set(touch.identifier, { x: touch.clientX, y: touch.clientY });
      });
      chassis.interrupt(JSON.stringify(event), []);
      let clapEvent = {
        "Clap": {
          "x": evt.touches[0].clientX,
          "y": evt.touches[0].clientY
        }
      };
      chassis.interrupt(JSON.stringify(clapEvent), []);
    }, { "passive": true, "capture": true });
    layer.addEventListener("touchmove", (evt) => {
      let touches = getTouchMessages(evt.touches);
      let event = {
        "TouchMove": {
          "touches": touches
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, { "passive": true, "capture": true });
    layer.addEventListener("touchend", (evt) => {
      let event = {
        "TouchEnd": {
          "touches": getTouchMessages(evt.changedTouches)
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
      Array.from(evt.changedTouches).forEach((touch) => {
        lastPositions.delete(touch.identifier);
      });
    }, { "passive": true, "capture": true });
    layer.addEventListener("keydown", (evt) => {
      let event = {
        "KeyDown": {
          "key": evt.key,
          "modifiers": convertModifiers(evt),
          "is_repeat": evt.repeat
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, true);
    layer.addEventListener("keyup", (evt) => {
      let event = {
        "KeyUp": {
          "key": evt.key,
          "modifiers": convertModifiers(evt),
          "is_repeat": evt.repeat
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, true);
    layer.addEventListener("keypress", (evt) => {
      let event = {
        "KeyPress": {
          "key": evt.key,
          "modifiers": convertModifiers(evt),
          "is_repeat": evt.repeat
        }
      };
      chassis.interrupt(JSON.stringify(event), []);
    }, true);
  }

  // src/index.ts
  var objectManager = new ObjectManager(SUPPORTED_OBJECTS);
  var messages;
  var nativePool = new NativeElementPool(objectManager);
  var textDecoder = new TextDecoder();
  var isMobile = false;
  var initializedChassis = false;
  function mount(selector_or_element, extensionlessUrl) {
    let link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = "pax-chassis-web-interface.css";
    document.head.appendChild(link);
    let mount2;
    if (typeof selector_or_element === "string") {
      mount2 = document.querySelector(selector_or_element);
    } else {
      mount2 = selector_or_element;
    }
    if (mount2) {
      startRenderLoop(extensionlessUrl, mount2).then();
    } else {
      console.error("Unable to find mount element");
    }
  }
  async function loadWasmModule(extensionlessUrl) {
    try {
      const glueCodeModule = await import(`${extensionlessUrl}.js`);
      const wasmBinary = await fetch(`${extensionlessUrl}_bg.wasm`);
      const wasmArrayBuffer = await wasmBinary.arrayBuffer();
      let _io = glueCodeModule.initSync(wasmArrayBuffer);
      let chassis = glueCodeModule.PaxChassisWeb.new();
      let get_latest_memory = glueCodeModule.wasm_memory;
      return { chassis, get_latest_memory };
    } catch (err) {
      throw new Error(`Failed to load WASM module: ${err}`);
    }
  }
  async function startRenderLoop(extensionlessUrl, mount2) {
    try {
      let { chassis, get_latest_memory } = await loadWasmModule(extensionlessUrl);
      isMobile = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
      nativePool.build(chassis, isMobile, mount2);
      requestAnimationFrame(renderLoop.bind(renderLoop, chassis, mount2, get_latest_memory));
    } catch (error) {
      console.error("Failed to load or instantiate Wasm module:", error);
    }
  }
  function renderLoop(chassis, mount2, get_latest_memory) {
    nativePool.sendScrollerValues();
    nativePool.clearCanvases();
    const memorySliceSpec = chassis.tick();
    const latestMemory = get_latest_memory();
    const memoryBuffer = new Uint8Array(latestMemory.buffer);
    const jsonString = textDecoder.decode(memoryBuffer.subarray(memorySliceSpec.ptr(), memorySliceSpec.ptr() + memorySliceSpec.len()));
    messages = JSON.parse(jsonString);
    if (!initializedChassis) {
      let resizeHandler = () => {
        let width = mount2.clientWidth;
        let height = mount2.clientHeight;
        chassis.send_viewport_update(width, height);
        nativePool.baseOcclusionContext.updateCanvases(width, height);
      };
      window.addEventListener("resize", resizeHandler);
      resizeHandler();
      setupEventListeners(chassis, mount2);
      initializedChassis = true;
    }
    processMessages(messages, chassis, objectManager);
    chassis.render();
    chassis.deallocate(memorySliceSpec);
    requestAnimationFrame(renderLoop.bind(renderLoop, chassis, mount2, get_latest_memory));
  }
  function processMessages(messages2, chassis, objectManager2) {
    messages2?.forEach((unwrapped_msg) => {
      if (unwrapped_msg["OcclusionUpdate"]) {
        let msg = unwrapped_msg["OcclusionUpdate"];
        let patch = objectManager2.getFromPool(OCCLUSION_UPDATE_PATCH);
        patch.fromPatch(msg);
        nativePool.occlusionUpdate(patch);
      } else if (unwrapped_msg["ButtonCreate"]) {
        let msg = unwrapped_msg["ButtonCreate"];
        let patch = objectManager2.getFromPool(ANY_CREATE_PATCH);
        patch.fromPatch(msg);
        nativePool.buttonCreate(patch);
      } else if (unwrapped_msg["ButtonUpdate"]) {
        let msg = unwrapped_msg["ButtonUpdate"];
        let patch = objectManager2.getFromPool(BUTTON_UPDATE_PATCH, objectManager2);
        patch.fromPatch(msg, nativePool.registeredFontFaces);
        nativePool.buttonUpdate(patch);
      } else if (unwrapped_msg["ButtonDelete"]) {
        let msg = unwrapped_msg["ButtonDelete"];
        nativePool.buttonDelete(msg);
      } else if (unwrapped_msg["CheckboxCreate"]) {
        let msg = unwrapped_msg["CheckboxCreate"];
        let patch = objectManager2.getFromPool(ANY_CREATE_PATCH);
        patch.fromPatch(msg);
        nativePool.checkboxCreate(patch);
      } else if (unwrapped_msg["CheckboxUpdate"]) {
        let msg = unwrapped_msg["CheckboxUpdate"];
        let patch = objectManager2.getFromPool(CHECKBOX_UPDATE_PATCH, objectManager2);
        patch.fromPatch(msg);
        nativePool.checkboxUpdate(patch);
      } else if (unwrapped_msg["CheckboxDelete"]) {
        let msg = unwrapped_msg["CheckboxDelete"];
        nativePool.checkboxDelete(msg);
      } else if (unwrapped_msg["TextCreate"]) {
        let msg = unwrapped_msg["TextCreate"];
        let patch = objectManager2.getFromPool(ANY_CREATE_PATCH);
        patch.fromPatch(msg);
        nativePool.textCreate(patch);
      } else if (unwrapped_msg["TextUpdate"]) {
        let msg = unwrapped_msg["TextUpdate"];
        let patch = objectManager2.getFromPool(TEXT_UPDATE_PATCH, objectManager2);
        patch.fromPatch(msg, nativePool.registeredFontFaces);
        nativePool.textUpdate(patch);
      } else if (unwrapped_msg["TextDelete"]) {
        let msg = unwrapped_msg["TextDelete"];
        nativePool.textDelete(msg);
      } else if (unwrapped_msg["FrameCreate"]) {
        let msg = unwrapped_msg["FrameCreate"];
        let patch = objectManager2.getFromPool(ANY_CREATE_PATCH);
        patch.fromPatch(msg);
        nativePool.frameCreate(patch);
      } else if (unwrapped_msg["FrameUpdate"]) {
        let msg = unwrapped_msg["FrameUpdate"];
        let patch = objectManager2.getFromPool(FRAME_UPDATE_PATCH);
        patch.fromPatch(msg);
        nativePool.frameUpdate(patch);
      } else if (unwrapped_msg["FrameDelete"]) {
        let msg = unwrapped_msg["FrameDelete"];
        nativePool.frameDelete(msg["id_chain"]);
      } else if (unwrapped_msg["ImageLoad"]) {
        let msg = unwrapped_msg["ImageLoad"];
        let patch = objectManager2.getFromPool(IMAGE_LOAD_PATCH);
        patch.fromPatch(msg);
        nativePool.imageLoad(patch, chassis);
      } else if (unwrapped_msg["ScrollerCreate"]) {
        let msg = unwrapped_msg["ScrollerCreate"];
        let patch = objectManager2.getFromPool(ANY_CREATE_PATCH);
        patch.fromPatch(msg);
        nativePool.scrollerCreate(patch, chassis);
      } else if (unwrapped_msg["ScrollerUpdate"]) {
        let msg = unwrapped_msg["ScrollerUpdate"];
        let patch = objectManager2.getFromPool(SCROLLER_UPDATE_PATCH);
        patch.fromPatch(msg);
        nativePool.scrollerUpdate(patch);
      } else if (unwrapped_msg["ScrollerDelete"]) {
        let msg = unwrapped_msg["ScrollerDelete"];
        nativePool.scrollerDelete(msg);
      }
    });
  }
  return __toCommonJS(src_exports);
})();
