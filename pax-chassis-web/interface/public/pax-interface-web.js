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
      this.id = jsonMessage["id"];
      this.parentFrame = jsonMessage["parent_frame"];
      this.occlusionLayerId = jsonMessage["occlusion_layer_id"];
    }
    cleanUp() {
      this.id = void 0;
      this.parentFrame = void 0;
      this.occlusionLayerId = -1;
    }
  };

  // src/classes/messages/frame-update-patch.ts
  var FrameUpdatePatch = class {
    fromPatch(jsonMessage) {
      if (jsonMessage != null) {
        this.id = jsonMessage["id"];
        this.sizeX = jsonMessage["size_x"];
        this.sizeY = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
      }
    }
    cleanUp() {
      this.id = void 0;
      this.sizeX = 0;
      this.sizeX = 0;
      this.transform = [];
    }
  };

  // src/classes/messages/text-update-patch.ts
  var TextUpdatePatch = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    fromPatch(jsonMessage, registeredFontFaces) {
      this.id = jsonMessage["id"];
      this.content = jsonMessage["content"];
      this.size_x = jsonMessage["size_x"];
      this.size_y = jsonMessage["size_y"];
      this.transform = jsonMessage["transform"];
      this.depth = jsonMessage["depth"];
      this.editable = jsonMessage["editable"];
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
      this.id = void 0;
      this.content = "";
      this.size_x = 0;
      this.size_y = 0;
      this.transform = [];
      this.objectManager.returnToPool(TEXT_STYLE, this.style);
      this.style = void 0;
      this.objectManager.returnToPool(TEXT_STYLE, this.style_link);
      this.style_link = void 0;
      this.editable = false;
    }
  };

  // src/classes/messages/scroller-update-patch.ts
  var ScrollerUpdatePatch = class {
    fromPatch(jsonMessage) {
      this.id = jsonMessage["id"];
      this.sizeX = jsonMessage["size_x"];
      this.sizeY = jsonMessage["size_y"];
      this.sizeInnerPaneX = jsonMessage["size_inner_pane_x"];
      this.sizeInnerPaneY = jsonMessage["size_inner_pane_y"];
      this.transform = jsonMessage["transform"];
      this.scrollX = jsonMessage["scroll_x"];
      this.scrollY = jsonMessage["scroll_y"];
    }
    cleanUp() {
      this.id = void 0;
      this.sizeX = 0;
      this.sizeY = 0;
      this.sizeInnerPaneX = 0;
      this.sizeInnerPaneY = 0;
      this.transform = [];
      this.scrollX = 0;
      this.scrollY = 0;
    }
  };

  // src/classes/messages/image-load-patch.ts
  var ImageLoadPatch = class {
    fromPatch(jsonMessage) {
      this.id = jsonMessage["id"];
      this.path = jsonMessage["path"];
    }
    cleanUp() {
      this.id = void 0;
      this.path = "";
    }
  };

  // src/utils/constants.ts
  var NATIVE_OVERLAY_CLASS = "native-overlay";
  var CANVAS_CLASS = "canvas";
  var NATIVE_LEAF_CLASS = "native-leaf";
  var BUTTON_CLASS = "button-styles";
  var CHECKBOX_CLASS = "checkbox-styles";
  var RADIO_SET_CLASS = "radio-set-style";
  var CLIPPING_CONTAINER = "clipping-container";
  var BUTTON_TEXT_CONTAINER_CLASS = "button-text-container";

  // src/classes/layer.ts
  var Layer = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    build(parent, occlusionLayerId, chassis, canvasMap) {
      this.occlusionLayerId = occlusionLayerId;
      this.chassis = chassis;
      this.canvasMap = canvasMap;
      this.canvas = this.objectManager.getFromPool(CANVAS);
      this.native = this.objectManager.getFromPool(DIV);
      this.canvas.style.zIndex = String(occlusionLayerId);
      this.canvas.id = String(occlusionLayerId);
      parent.appendChild(this.canvas);
      canvasMap.set(this.canvas.id, this.canvas);
      chassis.add_context(this.canvas.id);
      this.native.className = NATIVE_OVERLAY_CLASS;
      this.native.style.zIndex = String(occlusionLayerId);
      parent.appendChild(this.native);
    }
    cleanUp() {
      if (this.canvas != void 0 && this.chassis != void 0 && this.occlusionLayerId != void 0) {
        this.chassis.remove_context(this.occlusionLayerId.toString());
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
      this.occlusionLayerId = void 0;
    }
  };

  // src/utils/helpers.ts
  async function readImageToByteBuffer(imagePath) {
    let attempts = 8;
    let delay = 100;
    while (attempts > 0) {
      const response = await fetch(imagePath);
      if (response.ok) {
        const blob = await response.blob();
        const img = await createImageBitmap(blob);
        const canvas = new OffscreenCanvas(img.width + 1e3, img.height);
        const ctx = canvas.getContext("2d");
        ctx.drawImage(img, 0, 0, img.width, img.height);
        const imageData = ctx.getImageData(0, 0, img.width, img.height);
        let pixels = imageData.data;
        return { pixels, width: img.width, height: img.height };
      }
      await new Promise((resolve) => setTimeout(resolve, delay));
      delay *= 2;
      attempts--;
    }
    return Promise.reject("Failed to fetch image after maximum retries.");
  }
  function affineMultiply(point, matrix) {
    let x = point[0];
    let y = point[1];
    let a = matrix[0];
    let b = matrix[1];
    let c = matrix[2];
    let d = matrix[3];
    let e2 = matrix[4];
    let f = matrix[5];
    let xOut = a * x + c * y + e2;
    let yOut = b * x + d * y + f;
    return [xOut, yOut];
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
  function getQuadClipPolygonCommand(width, height, transform) {
    let point0 = affineMultiply([0, 0], transform);
    let point1 = affineMultiply([width, 0], transform);
    let point2 = affineMultiply([width, height], transform);
    let point3 = affineMultiply([0, height], transform);
    let polygon = `polygon(${point0[0]}px ${point0[1]}px, ${point1[0]}px ${point1[1]}px, ${point2[0]}px ${point2[1]}px, ${point3[0]}px ${point3[1]}px)`;
    return polygon;
  }

  // src/classes/occlusion-context.ts
  var OcclusionLayerManager = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
      this.containers = /* @__PURE__ */ new Map();
    }
    attach(parent, chassis, canvasMap) {
      this.layers = this.objectManager.getFromPool(ARRAY);
      this.parent = parent;
      this.chassis = chassis;
      this.canvasMap = canvasMap;
      this.growTo(0);
    }
    growTo(newOcclusionLayerId) {
      let occlusionLayerId = newOcclusionLayerId + 1;
      if (this.layers.length <= occlusionLayerId) {
        for (let i = this.layers.length; i <= occlusionLayerId; i++) {
          let newLayer = this.objectManager.getFromPool(LAYER, this.objectManager);
          newLayer.build(this.parent, i, this.chassis, this.canvasMap);
          this.layers.push(newLayer);
        }
      }
    }
    shrinkTo(occlusionLayerId) {
      if (this.layers == void 0) {
        return;
      }
      if (this.layers.length >= occlusionLayerId) {
        for (let i = this.layers.length - 1; i > occlusionLayerId; i--) {
          this.objectManager.returnToPool(LAYER, this.layers[i]);
          this.layers.pop();
        }
      }
    }
    addElement(element, parent_container, occlusionLayerId) {
      this.growTo(occlusionLayerId);
      let attach_point = this.getOrCreateContainer(parent_container, occlusionLayerId);
      attach_point.appendChild(element);
    }
    // If a div for the container referenced already exists, returns it. if not,
    // create it (and all non-existent parents)
    getOrCreateContainer(id, occlusionLayerId) {
      let layer = this.layers[occlusionLayerId].native;
      if (id == void 0) {
        return layer;
      }
      let elem = layer.querySelector(`[data-container-id="${id}"]`);
      if (elem != void 0) {
        return elem;
      }
      let container = this.containers.get(id);
      if (container == null) {
        throw new Error("something referenced a container that doesn't exist");
      }
      let new_container = this.objectManager.getFromPool(DIV);
      new_container.dataset.containerId = id.toString();
      new_container.setAttribute("class", CLIPPING_CONTAINER);
      let var_val = `var(${containerCssClipPathVar(id)})`;
      new_container.style.clipPath = var_val;
      new_container.style.webkitClipPath = var_val;
      let parent_container = this.getOrCreateContainer(container.parentFrame, occlusionLayerId);
      parent_container.appendChild(new_container);
      return new_container;
    }
    addContainer(id, parentId) {
      this.containers.set(id, new Container(id, parentId));
    }
    updateContainer(id, styles) {
      let container = this.containers.get(id);
      if (container == null) {
        throw new Error("tried to update non existent container");
      }
      container.updateClippingPath(styles);
    }
    removeContainer(id) {
      let container = this.containers.get(id);
      if (container == null) {
        throw new Error(`tried to delete non-existent container with id ${id}`);
      }
      this.containers.delete(id);
      let existing_layer_instantiations = document.querySelectorAll(`[data-container-id="${id}"]`);
      existing_layer_instantiations.forEach((elem, _key, _parent) => {
        let parent = elem.parentElement;
        if (elem.children.length > 0) {
          throw new Error(`tried to remove container width id ${id} while children still present`);
        }
        parent.removeChild(elem);
      });
      let var_name = containerCssClipPathVar(id);
      document.documentElement.style.removeProperty(var_name);
    }
    cleanUp() {
      if (this.layers != void 0) {
        this.layers.forEach((layer) => {
          this.objectManager.returnToPool(LAYER, layer);
        });
      }
      this.canvasMap = void 0;
      this.parent = void 0;
    }
  };
  var Container = class {
    constructor(id, parentId) {
      this.parentFrame = parentId;
      this.styles = new ContainerStyle();
      this.id = id;
    }
    updateClippingPath(patch) {
      this.styles = { ...this.styles, ...patch };
      let polygonDef = getQuadClipPolygonCommand(this.styles.width, this.styles.height, this.styles.transform);
      let var_name = containerCssClipPathVar(this.id);
      document.documentElement.style.setProperty(var_name, polygonDef);
    }
  };
  function containerCssClipPathVar(id) {
    return `--container-${id}-clip-path`;
  }
  var ContainerStyle = class {
    constructor() {
      this.transform = [0, 0, 0, 0, 0, 0];
      this.width = 0;
      this.height = 0;
    }
  };

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

  // src/classes/messages/checkbox-update-patch.ts
  var CheckboxUpdatePatch = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    fromPatch(jsonMessage) {
      this.id = jsonMessage["id"];
      this.size_x = jsonMessage["size_x"];
      this.size_y = jsonMessage["size_y"];
      this.transform = jsonMessage["transform"];
      this.checked = jsonMessage["checked"];
      this.borderRadius = jsonMessage["border_radius"];
      this.outlineColor = jsonMessage["outline_color"];
      this.outlineWidth = jsonMessage["outline_width"];
      this.background = jsonMessage["background"];
      this.backgroundChecked = jsonMessage["background_checked"];
    }
    cleanUp() {
      this.id = void 0;
      this.size_x = 0;
      this.size_y = 0;
      this.transform = [];
      this.checked = void 0;
    }
  };

  // src/classes/messages/occlusion-update-patch.ts
  var OcclusionUpdatePatch = class {
    fromPatch(jsonMessage) {
      this.id = jsonMessage["id"];
      this.occlusionLayerId = jsonMessage["occlusion_layer_id"];
      this.zIndex = jsonMessage["z_index"];
    }
    cleanUp() {
      this.id = void 0;
      this.occlusionLayerId = -1;
      this.zIndex = -1;
    }
  };

  // src/classes/messages/button-update-patch.ts
  var ButtonUpdatePatch = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    fromPatch(jsonMessage, registeredFontFaces) {
      this.id = jsonMessage["id"];
      this.content = jsonMessage["content"];
      this.size_x = jsonMessage["size_x"];
      this.size_y = jsonMessage["size_y"];
      this.transform = jsonMessage["transform"];
      this.color = jsonMessage["color"];
      this.hoverColor = jsonMessage["hover_color"];
      this.outlineStrokeColor = jsonMessage["outline_stroke_color"];
      this.outlineStrokeWidth = jsonMessage["outline_stroke_width"];
      this.borderRadius = jsonMessage["border_radius"];
      const styleMessage = jsonMessage["style"];
      if (styleMessage) {
        this.style = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
        this.style.build(styleMessage, registeredFontFaces);
      }
    }
    cleanUp() {
      this.id = void 0;
      this.size_x = 0;
      this.size_y = 0;
      this.transform = [];
      this.objectManager.returnToPool(TEXT_STYLE, this.style);
      this.style = void 0;
    }
  };

  // src/classes/messages/textbox-update-patch.ts
  var TextboxUpdatePatch = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    fromPatch(jsonMessage, registeredFontFaces) {
      this.id = jsonMessage["id"];
      this.size_x = jsonMessage["size_x"];
      this.size_y = jsonMessage["size_y"];
      this.transform = jsonMessage["transform"];
      this.text = jsonMessage["text"];
      this.stroke_color = jsonMessage["stroke_color"];
      this.stroke_width = jsonMessage["stroke_width"];
      this.background = jsonMessage["background"];
      this.border_radius = jsonMessage["border_radius"];
      this.focus_on_mount = jsonMessage["focus_on_mount"];
      const styleMessage = jsonMessage["style"];
      if (styleMessage) {
        this.style = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
        this.style.build(styleMessage, registeredFontFaces);
      }
    }
    cleanUp() {
      this.id = void 0;
      this.size_x = 0;
      this.size_y = 0;
      this.transform = [];
      this.text = "";
    }
  };

  // src/classes/messages/dropdown-update-patch.ts
  var DropdownUpdatePatch = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    fromPatch(jsonMessage, registeredFontFaces) {
      this.id = jsonMessage["id"];
      this.size_x = jsonMessage["size_x"];
      this.size_y = jsonMessage["size_y"];
      this.transform = jsonMessage["transform"];
      this.options = jsonMessage["options"];
      this.stroke_color = jsonMessage["stroke_color"];
      this.stroke_width = jsonMessage["stroke_width"];
      this.background = jsonMessage["background"];
      this.selected_id = jsonMessage["selected_id"];
      this.borderRadius = jsonMessage["border_radius"];
      const styleMessage = jsonMessage["style"];
      if (styleMessage) {
        this.style = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
        this.style.build(styleMessage, registeredFontFaces);
      }
    }
    cleanUp() {
      this.id = void 0;
      this.size_x = 0;
      this.size_y = 0;
      this.transform = [];
      this.options = [];
      this.selected_id = 0;
    }
  };

  // src/classes/messages/slider-update-patch.ts
  var SliderUpdatePatch = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    fromPatch(jsonMessage) {
      this.id = jsonMessage["id"];
      this.size_x = jsonMessage["size_x"];
      this.size_y = jsonMessage["size_y"];
      this.transform = jsonMessage["transform"];
      this.accent = jsonMessage["accent"];
      this.value = jsonMessage["value"];
      this.step = jsonMessage["step"];
      this.min = jsonMessage["min"];
      this.max = jsonMessage["max"];
      this.borderRadius = jsonMessage["border_radius"];
      this.background = jsonMessage["background"];
    }
    cleanUp() {
      this.id = void 0;
      this.size_x = 0;
      this.size_y = 0;
      this.value = 0;
      this.step = 0;
      this.min = 0;
      this.max = 0;
      this.borderRadius = 0;
      this.background = void 0;
      this.accent = void 0;
      this.transform = [];
    }
  };

  // src/classes/messages/radio-set-update-patch.ts
  var RadioSetUpdatePatch = class {
    constructor(objectManager2) {
      this.objectManager = objectManager2;
    }
    fromPatch(jsonMessage, registeredFontFaces) {
      this.id = jsonMessage["id"];
      this.size_x = jsonMessage["size_x"];
      this.size_y = jsonMessage["size_y"];
      this.transform = jsonMessage["transform"];
      this.options = jsonMessage["options"];
      this.background = jsonMessage["background"];
      this.selected_id = jsonMessage["selected_id"];
      this.backgroundChecked = jsonMessage["background_checked"];
      this.outlineColor = jsonMessage["outline_color"];
      this.outlineWidth = jsonMessage["outline_width"];
      const styleMessage = jsonMessage["style"];
      if (styleMessage) {
        this.style = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
        this.style.build(styleMessage, registeredFontFaces);
      }
    }
    cleanUp() {
      this.id = void 0;
      this.size_x = 0;
      this.size_y = 0;
      this.transform = [];
      this.options = [];
      this.selected_id = 0;
    }
  };

  // src/classes/messages/event-blocker-update-patch.ts
  var EventBlockerUpdatePatch = class {
    fromPatch(jsonMessage) {
      if (jsonMessage != null) {
        this.id = jsonMessage["id"];
        this.sizeX = jsonMessage["size_x"];
        this.sizeY = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
      }
    }
    cleanUp() {
      this.id = void 0;
      this.sizeX = 0;
      this.sizeX = 0;
      this.transform = [];
    }
  };

  // src/pools/supported-objects.ts
  var OBJECT = "Object";
  var ARRAY = "Array";
  var DIV = "DIV";
  var INPUT = "Input";
  var SELECT = "Select";
  var BUTTON = "Button";
  var CANVAS = "Canvas";
  var ANY_CREATE_PATCH = "Any Create Patch";
  var OCCLUSION_UPDATE_PATCH = "Occlusion Update Patch";
  var FRAME_UPDATE_PATCH = "Frame Update Patch";
  var EVENT_BLOCKER_UPDATE_PATCH = "Event Blocker Update Patch";
  var IMAGE_LOAD_PATCH = "IMAGE LOAD PATCH";
  var SCROLLER_UPDATE_PATCH = "Scroller Update Patch";
  var TEXT_UPDATE_PATCH = "Text Update Patch";
  var CHECKBOX_UPDATE_PATCH = "Checkbox Update Patch";
  var TEXTBOX_UPDATE_PATCH = "Textbox Update Patch";
  var DROPDOWN_UPDATE_PATCH = "Dropdown Update Patch";
  var BUTTON_UPDATE_PATCH = "Button Update Patch";
  var SLIDER_UPDATE_PATCH = "Slider Update Patch";
  var RADIOSET_UPDATE_PATCH = "Radio Set Update Patch";
  var LAYER = "LAYER";
  var OCCLUSION_CONTEXT = "Occlusion Context";
  var FONT = "Font";
  var TEXT_STYLE = "Text Style";
  var UINT32ARRAY = "UInt32Array";
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
      name: SELECT,
      factory: () => document.createElement("select"),
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
      name: ARRAY,
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
      name: EVENT_BLOCKER_UPDATE_PATCH,
      factory: () => new EventBlockerUpdatePatch(),
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
      name: TEXTBOX_UPDATE_PATCH,
      factory: (objectManager2) => new TextboxUpdatePatch(objectManager2),
      cleanUp: (patch) => {
        patch.cleanUp();
      }
    },
    {
      name: DROPDOWN_UPDATE_PATCH,
      factory: (objectManager2) => new DropdownUpdatePatch(objectManager2),
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
      name: SLIDER_UPDATE_PATCH,
      factory: (objectManager2) => new SliderUpdatePatch(objectManager2),
      cleanUp: (patch) => {
        patch.cleanUp();
      }
    },
    {
      name: RADIOSET_UPDATE_PATCH,
      factory: (objectManager2) => new RadioSetUpdatePatch(objectManager2),
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
      factory: (objectManager2) => new OcclusionLayerManager(objectManager2),
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
      name: UINT32ARRAY,
      factory: () => new Uint32Array(128),
      cleanUp: (array) => {
        array.fill(0);
      }
    }
  ];

  // node_modules/snarkdown/dist/snarkdown.es.js
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

  // src/classes/native-element-pool.ts
  var NativeElementPool = class {
    constructor(objectManager2) {
      this.nodesLookup = /* @__PURE__ */ new Map();
      this.objectManager = objectManager2;
      this.canvases = /* @__PURE__ */ new Map();
      this.layers = objectManager2.getFromPool(OCCLUSION_CONTEXT, objectManager2);
      this.registeredFontFaces = /* @__PURE__ */ new Set();
      this.resizeObserver = new ResizeObserver((entries) => {
        let resize_requests = [];
        for (const entry of entries) {
          let node = entry.target;
          let id = parseInt(node.getAttribute("pax_id"));
          let width = entry.contentRect.width;
          let height = entry.contentRect.height;
          let message = {
            "id": id,
            "width": width,
            "height": height
          };
          resize_requests.push(message);
        }
        this.chassis.interrupt(JSON.stringify({
          "ChassisResizeRequestCollection": resize_requests
        }), void 0);
      });
    }
    attach(chassis, mount2) {
      this.chassis = chassis;
      this.layers.attach(mount2, chassis, this.canvases);
    }
    clearCanvases() {
      this.canvases.forEach((canvas, _key) => {
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
    occlusionUpdate(patch) {
      let node = this.nodesLookup.get(patch.id);
      if (node) {
        let parent = node.parentElement;
        let id_str = parent?.dataset.containerId;
        let id;
        if (id_str !== void 0) {
          id = parseInt(id_str);
        } else {
          id = void 0;
        }
        this.layers.addElement(node, id, patch.occlusionLayerId);
        node.style.zIndex = patch.zIndex;
        const focusableElements = node.querySelectorAll("input, button, select, textarea, a[href]");
        focusableElements.forEach((element, _index) => {
          element.setAttribute("tabindex", 1e6 - patch.zIndex);
        });
      }
    }
    checkboxCreate(patch) {
      console.assert(patch.id != null);
      console.assert(patch.occlusionLayerId != null);
      const checkbox = this.objectManager.getFromPool(INPUT);
      checkbox.type = "checkbox";
      checkbox.style.margin = "0";
      checkbox.setAttribute("class", CHECKBOX_CLASS);
      checkbox.addEventListener("change", (event2) => {
        const is_checked = event2.target.checked;
        checkbox.checked = !is_checked;
        let message = {
          "FormCheckboxToggle": {
            "id": patch.id,
            "state": is_checked
          }
        };
        this.chassis.interrupt(JSON.stringify(message), void 0);
      });
      let checkbox_div = this.objectManager.getFromPool(DIV);
      checkbox_div.appendChild(checkbox);
      checkbox_div.setAttribute("class", NATIVE_LEAF_CLASS);
      checkbox_div.setAttribute("pax_id", String(patch.id));
      if (patch.id != void 0 && patch.occlusionLayerId != void 0) {
        this.layers.addElement(checkbox_div, patch.parentFrame, patch.occlusionLayerId);
      }
      this.nodesLookup.set(patch.id, checkbox_div);
    }
    checkboxUpdate(patch) {
      let leaf = this.nodesLookup.get(patch.id);
      let checkbox = leaf.firstChild;
      updateCommonProps(leaf, patch);
      if (patch.checked !== null) {
        checkbox.checked = patch.checked;
      }
      if (patch.background) {
        checkbox.style.background = toCssColor(patch.background);
      }
      if (patch.borderRadius) {
        checkbox.style.borderRadius = patch.borderRadius + "px";
      }
      if (patch.outlineWidth !== void 0) {
        checkbox.style.borderWidth = patch.outlineWidth + "px";
      }
      if (patch.outlineColor) {
        checkbox.style.borderColor = toCssColor(patch.outlineColor);
      }
      if (patch.backgroundChecked) {
        checkbox.style.setProperty("--checked-color", toCssColor(patch.backgroundChecked));
      }
    }
    checkboxDelete(id) {
      let oldNode = this.nodesLookup.get(id);
      if (oldNode) {
        let parent = oldNode.parentElement;
        parent.removeChild(oldNode);
        this.nodesLookup.delete(id);
      }
    }
    textboxCreate(patch) {
      const textbox = this.objectManager.getFromPool(INPUT);
      textbox.type = "text";
      textbox.style.margin = "0";
      textbox.style.padding = "0";
      textbox.style.paddingInline = "5px 5px";
      textbox.style.paddingBlock = "0";
      textbox.style.borderWidth = "0";
      textbox.addEventListener("input", (_event) => {
        let message = {
          "FormTextboxInput": {
            "id": patch.id,
            "text": textbox.value
          }
        };
        this.chassis.interrupt(JSON.stringify(message), void 0);
      });
      textbox.addEventListener("change", (_event) => {
        let message = {
          "FormTextboxChange": {
            "id": patch.id,
            "text": textbox.value
          }
        };
        this.chassis.interrupt(JSON.stringify(message), void 0);
      });
      let textboxDiv = this.objectManager.getFromPool(DIV);
      textboxDiv.appendChild(textbox);
      textboxDiv.setAttribute("class", NATIVE_LEAF_CLASS);
      textboxDiv.setAttribute("pax_id", String(patch.id));
      if (patch.id != void 0 && patch.occlusionLayerId != void 0) {
        this.layers.addElement(textboxDiv, patch.parentFrame, patch.occlusionLayerId);
        this.nodesLookup.set(patch.id, textboxDiv);
      } else {
        throw new Error("undefined id or occlusionLayer");
      }
    }
    textboxUpdate(patch) {
      let leaf = this.nodesLookup.get(patch.id);
      updateCommonProps(leaf, patch);
      if (patch.size_x != null) {
        leaf.firstChild.style.width = patch.size_x - 10 + "px";
      }
      let textbox = leaf.firstChild;
      applyTextTyle(textbox, textbox, patch.style);
      textbox.style.borderStyle = "solid";
      if (patch.background) {
        textbox.style.background = toCssColor(patch.background);
      }
      if (patch.border_radius) {
        textbox.style.borderRadius = patch.border_radius + "px";
      }
      if (patch.stroke_color) {
        textbox.style.borderColor = toCssColor(patch.stroke_color);
      }
      if (patch.stroke_width) {
        textbox.style.borderWidth = patch.stroke_width + "px";
      }
      if (patch.text != null) {
        if (document.activeElement === textbox) {
          const selectionStart = textbox.selectionStart || 0;
          textbox.value = patch.text;
          const newCursorPosition = Math.min(selectionStart, patch.text.length);
          textbox.setSelectionRange(newCursorPosition, newCursorPosition);
        } else {
          textbox.value = patch.text;
        }
      }
      if (patch.focus_on_mount) {
        setTimeout(() => {
          textbox.focus();
        }, 10);
      }
    }
    textboxDelete(id) {
      let oldNode = this.nodesLookup.get(id);
      if (oldNode) {
        let parent = oldNode.parentElement;
        parent.removeChild(oldNode);
        this.nodesLookup.delete(id);
      }
    }
    radioSetCreate(patch) {
      let fields = document.createElement("fieldset");
      fields.style.border = "0";
      fields.style.margin = "0";
      fields.style.padding = "0";
      fields.addEventListener("change", (event2) => {
        let target = event2.target;
        if (target && target.matches("input[type='radio']")) {
          let container = target.parentNode;
          let index = Array.from(container.parentNode.children).indexOf(container);
          let message = {
            "FormRadioSetChange": {
              "id": patch.id,
              "selected_id": index
            }
          };
          this.chassis.interrupt(JSON.stringify(message), void 0);
        }
      });
      let radioSetDiv = this.objectManager.getFromPool(DIV);
      radioSetDiv.setAttribute("class", NATIVE_LEAF_CLASS);
      radioSetDiv.setAttribute("pax_id", String(patch.id));
      radioSetDiv.appendChild(fields);
      if (patch.id != void 0 && patch.occlusionLayerId != void 0) {
        this.layers.addElement(radioSetDiv, patch.parentFrame, patch.occlusionLayerId);
        this.nodesLookup.set(patch.id, radioSetDiv);
      } else {
        throw new Error("undefined id or occlusionLayer");
      }
    }
    radioSetUpdate(patch) {
      let leaf = this.nodesLookup.get(patch.id);
      updateCommonProps(leaf, patch);
      if (patch.style) {
        applyTextTyle(leaf, leaf, patch.style);
      }
      let fields = leaf.firstChild;
      if (patch.options) {
        fields.innerHTML = "";
        patch.options.forEach((optionText, _index) => {
          let div = document.createElement("div");
          div.style.alignItems = "center";
          div.style.display = "flex";
          div.style.marginBottom = "3px";
          const option = document.createElement("input");
          option.type = "radio";
          option.name = `radio-${patch.id}`;
          option.value = optionText.toString();
          option.setAttribute("class", RADIO_SET_CLASS);
          div.appendChild(option);
          const label = document.createElement("label");
          label.innerHTML = optionText.toString();
          div.appendChild(label);
          fields.appendChild(div);
        });
      }
      if (patch.selected_id) {
        let radio = fields.children[patch.selected_id].firstChild;
        if (radio.checked == false) {
          radio.checked = true;
        }
      }
      if (patch.background) {
        fields.style.setProperty("--background-color", toCssColor(patch.background));
      }
      if (patch.backgroundChecked) {
        fields.style.setProperty("--selected-color", toCssColor(patch.backgroundChecked));
      }
      if (patch.outlineWidth != null) {
        fields.style.setProperty("--border-width", patch.outlineWidth + "px");
      }
      if (patch.outlineColor) {
        fields.style.setProperty("--border-color", toCssColor(patch.outlineColor));
      }
    }
    radioSetDelete(id) {
      let oldNode = this.nodesLookup.get(id);
      if (oldNode) {
        let parent = oldNode.parentElement;
        parent.removeChild(oldNode);
        this.nodesLookup.delete(id);
      }
    }
    sliderCreate(patch) {
      const slider = this.objectManager.getFromPool(INPUT);
      slider.type = "range";
      slider.style.padding = "0px";
      slider.style.margin = "0px";
      slider.style.appearance = "none";
      slider.style.display = "block";
      slider.addEventListener("input", (_event) => {
        let message = {
          "FormSliderChange": {
            "id": patch.id,
            "value": parseFloat(slider.value)
          }
        };
        this.chassis.interrupt(JSON.stringify(message), void 0);
      });
      let sliderDiv = this.objectManager.getFromPool(DIV);
      sliderDiv.appendChild(slider);
      sliderDiv.setAttribute("class", NATIVE_LEAF_CLASS);
      sliderDiv.style.overflow = "visible";
      sliderDiv.setAttribute("pax_id", String(patch.id));
      if (patch.id != void 0 && patch.occlusionLayerId != void 0) {
        this.layers.addElement(sliderDiv, patch.parentFrame, patch.occlusionLayerId);
        this.nodesLookup.set(patch.id, sliderDiv);
      } else {
        throw new Error("undefined id or occlusionLayer");
      }
    }
    sliderUpdate(patch) {
      let leaf = this.nodesLookup.get(patch.id);
      updateCommonProps(leaf, patch);
      let slider = leaf.firstChild;
      if (patch.value && patch.value.toString() != slider.value) {
        slider.value = patch.value.toString();
      }
      if (patch.step && patch.step.toString() != slider.step) {
        slider.step = patch.step.toString();
      }
      if (patch.min && patch.min.toString() != slider.min) {
        slider.min = patch.min.toString();
      }
      if (patch.max && patch.max.toString() != slider.max) {
        slider.max = patch.max.toString();
      }
      if (patch.accent) {
        let color = toCssColor(patch.accent);
        slider.style.accentColor = color;
      }
      if (patch.background) {
        let color = toCssColor(patch.background);
        slider.style.backgroundColor = color;
      }
      if (patch.borderRadius) {
        slider.style.borderRadius = patch.borderRadius + "px";
      }
    }
    sliderDelete(id) {
      let oldNode = this.nodesLookup.get(id);
      if (oldNode) {
        let parent = oldNode.parentElement;
        parent.removeChild(oldNode);
        this.nodesLookup.delete(id);
      }
    }
    dropdownCreate(patch) {
      const dropdown = this.objectManager.getFromPool(SELECT);
      dropdown.addEventListener("change", (event2) => {
        let message = {
          "FormDropdownChange": {
            "id": patch.id,
            "selected_id": event2.target.selectedIndex
          }
        };
        this.chassis.interrupt(JSON.stringify(message), void 0);
      });
      let textboxDiv = this.objectManager.getFromPool(DIV);
      textboxDiv.appendChild(dropdown);
      textboxDiv.setAttribute("class", NATIVE_LEAF_CLASS);
      textboxDiv.setAttribute("pax_id", String(patch.id));
      if (patch.id != void 0 && patch.occlusionLayerId != void 0) {
        this.layers.addElement(textboxDiv, patch.parentFrame, patch.occlusionLayerId);
        this.nodesLookup.set(patch.id, textboxDiv);
      } else {
        throw new Error("undefined id or occlusionLayer");
      }
    }
    dropdownUpdate(patch) {
      let leaf = this.nodesLookup.get(patch.id);
      updateCommonProps(leaf, patch);
      let dropdown = leaf.firstChild;
      applyTextTyle(dropdown, dropdown, patch.style);
      dropdown.style.borderStyle = "solid";
      if (patch.selected_id && dropdown.options.selectedIndex != patch.selected_id) {
        dropdown.options.selectedIndex = patch.selected_id;
      }
      if (patch.background) {
        dropdown.style.backgroundColor = toCssColor(patch.background);
      }
      if (patch.stroke_color) {
        dropdown.style.borderColor = toCssColor(patch.stroke_color);
      }
      if (patch.stroke_width != null) {
        dropdown.style.borderWidth = patch.stroke_width + "px";
      }
      if (patch.borderRadius != null) {
        dropdown.style.borderRadius = patch.borderRadius + "px";
      }
      if (patch.options != null) {
        dropdown.innerHTML = "";
        patch.options.forEach((optionText, index) => {
          const option = document.createElement("option");
          option.value = index.toString();
          option.textContent = optionText;
          dropdown.appendChild(option);
        });
      }
    }
    dropdownDelete(id) {
      let oldNode = this.nodesLookup.get(id);
      if (oldNode) {
        let parent = oldNode.parentElement;
        parent.removeChild(oldNode);
        this.nodesLookup.delete(id);
      }
    }
    buttonCreate(patch) {
      console.assert(patch.id != null);
      console.assert(patch.occlusionLayerId != null);
      const button = this.objectManager.getFromPool(BUTTON);
      const textContainer = this.objectManager.getFromPool(DIV);
      const textChild = this.objectManager.getFromPool(DIV);
      button.setAttribute("class", BUTTON_CLASS);
      textContainer.setAttribute("class", BUTTON_TEXT_CONTAINER_CLASS);
      textChild.style.margin = "0";
      button.addEventListener("click", (_event) => {
        let message = {
          "FormButtonClick": {
            "id": patch.id
          }
        };
        this.chassis.interrupt(JSON.stringify(message), void 0);
      });
      let buttonDiv = this.objectManager.getFromPool(DIV);
      textContainer.appendChild(textChild);
      button.appendChild(textContainer);
      buttonDiv.appendChild(button);
      buttonDiv.setAttribute("class", NATIVE_LEAF_CLASS);
      buttonDiv.setAttribute("pax_id", String(patch.id));
      if (patch.id != void 0 && patch.occlusionLayerId != void 0) {
        this.layers.addElement(buttonDiv, patch.parentFrame, patch.occlusionLayerId);
        this.nodesLookup.set(patch.id, buttonDiv);
      } else {
        throw new Error("undefined id or occlusionLayer");
      }
    }
    buttonUpdate(patch) {
      let leaf = this.nodesLookup.get(patch.id);
      updateCommonProps(leaf, patch);
      console.assert(leaf !== void 0);
      let button = leaf.firstChild;
      let textContainer = button.firstChild;
      let textChild = textContainer.firstChild;
      if (patch.content != null) {
        textChild.innerHTML = t(patch.content);
      }
      if (textChild.innerHTML.length == 0) {
        textChild.innerHTML = " ";
      }
      if (patch.color) {
        button.style.background = toCssColor(patch.color);
      }
      if (patch.hoverColor) {
        let color = toCssColor(patch.hoverColor);
        button.style.setProperty("--hover-color", color);
      }
      if (patch.borderRadius) {
        button.style.borderRadius = patch.borderRadius + "px";
      }
      if (patch.outlineStrokeColor) {
        button.style.borderColor = toCssColor(patch.outlineStrokeColor);
      }
      if (patch.outlineStrokeWidth != null) {
        button.style.borderWidth = patch.outlineStrokeWidth + "px";
      }
      applyTextTyle(textContainer, textChild, patch.style);
    }
    buttonDelete(id) {
      let oldNode = this.nodesLookup.get(id);
      if (oldNode) {
        let parent = oldNode.parentElement;
        parent.removeChild(oldNode);
        this.nodesLookup.delete(id);
      }
    }
    textCreate(patch) {
      console.assert(patch.id != null);
      console.assert(patch.occlusionLayerId != null);
      let textDiv = this.objectManager.getFromPool(DIV);
      let textChild = this.objectManager.getFromPool(DIV);
      textChild.addEventListener("input", (_event) => {
        let message = {
          "TextInput": {
            "id": patch.id,
            "text": sanitizeContentEditableString(textChild.innerHTML)
          }
        };
        this.chassis.interrupt(JSON.stringify(message), void 0);
      });
      textDiv.appendChild(textChild);
      textDiv.setAttribute("class", NATIVE_LEAF_CLASS);
      textDiv.setAttribute("pax_id", String(patch.id));
      if (patch.id != void 0 && patch.occlusionLayerId != void 0) {
        this.layers.addElement(textDiv, patch.parentFrame, patch.occlusionLayerId);
        this.nodesLookup.set(patch.id, textDiv);
      } else {
        throw new Error("undefined id or occlusionLayer");
      }
    }
    textUpdate(patch) {
      let leaf = this.nodesLookup.get(patch.id);
      let textChild = leaf.firstChild;
      let start_listening = false;
      if (patch.size_x != null) {
        if (patch.size_x == -1) {
          start_listening = true;
        } else {
          leaf.style.width = patch.size_x + "px";
        }
      }
      if (patch.size_y != null) {
        if (patch.size_y == -1) {
          start_listening = true;
        } else {
          leaf.style.height = patch.size_y + "px";
        }
      }
      if (start_listening) {
        this.resizeObserver.observe(leaf);
      }
      if (patch.transform != null) {
        leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
      }
      if (patch.editable != null) {
        textChild.setAttribute("contenteditable", patch.editable.toString());
        if (patch.editable == true) {
          textChild.style.outline = "none";
          textChild.style.width = "inherit";
          textChild.style.height = "inherit";
        }
      }
      applyTextTyle(leaf, textChild, patch.style);
      if (patch.content != null) {
        if (sanitizeContentEditableString(textChild.innerHTML) != patch.content) {
          textChild.innerHTML = patch.content;
        }
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
                newValue = `rgba(${p[0] * 255},${p[1] * 255},${p[2] * 255},${p[3]})`;
              } else {
                console.warn("Unsupported Color Format");
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
    }
    textDelete(id) {
      let oldNode = this.nodesLookup.get(id);
      this.resizeObserver.unobserve(oldNode);
      if (oldNode) {
        let parent = oldNode.parentElement;
        parent.removeChild(oldNode);
        this.nodesLookup.delete(id);
      }
    }
    frameCreate(patch) {
      console.assert(patch.id != null);
      this.layers.addContainer(patch.id, patch.parentFrame);
    }
    frameUpdate(patch) {
      console.assert(patch.id != null);
      let styles = {};
      if (patch.sizeX != null) {
        styles.width = patch.sizeX;
      }
      if (patch.sizeY != null) {
        styles.height = patch.sizeY;
      }
      if (patch.transform != null) {
        styles.transform = patch.transform;
      }
      this.layers.updateContainer(patch.id, styles);
    }
    frameDelete(id) {
      this.layers.removeContainer(id);
    }
    scrollerCreate(patch) {
      console.assert(patch.id != null);
      console.assert(patch.occlusionLayerId != null);
      let scrollerDiv = this.objectManager.getFromPool(DIV);
      let scroller = this.objectManager.getFromPool(DIV);
      scrollerDiv.addEventListener("scroll", (_event) => {
      });
      scrollerDiv.appendChild(scroller);
      scrollerDiv.setAttribute("class", NATIVE_LEAF_CLASS);
      scrollerDiv.setAttribute("pax_id", String(patch.id));
      if (patch.id != void 0 && patch.occlusionLayerId != void 0) {
        this.layers.addElement(scrollerDiv, patch.parentFrame, patch.occlusionLayerId);
        this.nodesLookup.set(patch.id, scrollerDiv);
      } else {
        throw new Error("undefined id or occlusionLayer");
      }
    }
    scrollerUpdate(patch) {
      let leaf = this.nodesLookup.get(patch.id);
      if (leaf == void 0) {
        throw new Error("tried to update non-existent scroller");
      }
      let scroller_inner = leaf.firstChild;
      if (patch.sizeX != null) {
        leaf.style.width = patch.sizeX + "px";
      }
      if (patch.sizeY != null) {
        leaf.style.height = patch.sizeY + "px";
      }
      if (patch.scrollX != null) {
        leaf.scrollLeft = patch.scrollX;
      }
      if (patch.scrollY != null) {
        leaf.scrollTop = patch.scrollY;
      }
      if (patch.transform != null) {
        leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
      }
      if (patch.sizeInnerPaneX != null) {
        if (patch.sizeInnerPaneX <= parseFloat(leaf.style.width)) {
          leaf.style.overflowX = "hidden";
        } else {
          leaf.style.overflowX = "auto";
        }
        scroller_inner.style.width = patch.sizeInnerPaneX + "px";
      }
      if (patch.sizeInnerPaneY != null) {
        if (patch.sizeInnerPaneY <= parseFloat(leaf.style.height)) {
          leaf.style.overflowY = "hidden";
        } else {
          leaf.style.overflowY = "auto";
        }
        scroller_inner.style.height = patch.sizeInnerPaneY + "px";
      }
    }
    scrollerDelete(id) {
      let oldNode = this.nodesLookup.get(id);
      if (oldNode == void 0) {
        throw new Error("tried to delete non-existent scroller");
      }
      let parent = oldNode.parentElement;
      parent.removeChild(oldNode);
      this.nodesLookup.delete(id);
    }
    eventBlockerCreate(patch) {
      console.assert(patch.id != null);
      console.assert(patch.occlusionLayerId != null);
      let eventBlockerDiv = this.objectManager.getFromPool(DIV);
      eventBlockerDiv.setAttribute("class", NATIVE_LEAF_CLASS);
      eventBlockerDiv.setAttribute("pax_id", String(patch.id));
      if (patch.id != void 0 && patch.occlusionLayerId != void 0) {
        this.layers.addElement(eventBlockerDiv, patch.parentFrame, patch.occlusionLayerId);
        this.nodesLookup.set(patch.id, eventBlockerDiv);
      } else {
        throw new Error("undefined id or occlusionLayer");
      }
    }
    eventBlockerUpdate(patch) {
      let leaf = this.nodesLookup.get(patch.id);
      if (leaf == void 0) {
        throw new Error("tried to update non-existent event blocker");
      }
      if (patch.sizeX != null) {
        leaf.style.width = patch.sizeX + "px";
      }
      if (patch.sizeY != null) {
        leaf.style.height = patch.sizeY + "px";
      }
      if (patch.transform != null) {
        leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
      }
    }
    eventBlockerDelete(id) {
      let oldNode = this.nodesLookup.get(id);
      if (oldNode == void 0) {
        throw new Error("tried to delete non-existent event blocker");
      }
      let parent = oldNode.parentElement;
      parent.removeChild(oldNode);
      this.nodesLookup.delete(id);
    }
    async imageLoad(patch, chassis) {
      if (chassis.image_loaded(patch.path ?? "")) {
        return;
      }
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
      const BASE_PATH = getScriptBasePath("pax-cartridge.js");
      let path = (BASE_PATH + patch.path).replace("//", "/");
      let image_data = await readImageToByteBuffer(path);
      let message = {
        "Image": {
          "Data": {
            "id": patch.id,
            "path": patch.path,
            "width": image_data.width,
            "height": image_data.height
          }
        }
      };
      chassis.interrupt(JSON.stringify(message), image_data.pixels);
    }
  };
  function toCssColor(color) {
    if (color.Rgba != null) {
      let p = color.Rgba;
      return `rgba(${p[0] * 255},${p[1] * 255},${p[2] * 255},${p[3]})`;
    } else {
      throw new TypeError("Unsupported Color Format");
    }
  }
  function applyTextTyle(textContainer, textElem, style) {
    if (style) {
      if (style.font) {
        style.font.applyFontToDiv(textContainer);
      }
      if (style.fill) {
        textElem.style.color = toCssColor(style.fill);
      }
      if (style.font_size) {
        textElem.style.fontSize = style.font_size + "px";
      }
      if (style.underline != null) {
        textElem.style.textDecoration = style.underline ? "underline" : "none";
      }
      if (style.align_horizontal) {
        textContainer.style.display = "flex";
        textContainer.style.justifyContent = getJustifyContent(style.align_horizontal);
      }
      if (style.align_vertical) {
        textContainer.style.alignItems = getAlignItems(style.align_vertical);
      }
      if (style.align_multiline) {
        textElem.style.textAlign = getTextAlign(style.align_multiline);
      }
    }
  }
  function sanitizeContentEditableString(string) {
    return string.replace(/<br\s*\/*>/ig, "\n").replace(/(<(p|div))/ig, "\n$1").replace(/(<([^>]+)>)/ig, "") ?? "";
  }
  function updateCommonProps(leaf, patch) {
    let elem = leaf.firstChild;
    if (patch.size_x != null) {
      elem.style.width = patch.size_x + "px";
    }
    if (patch.size_y != null) {
      elem.style.height = patch.size_y + "px";
    }
    if (patch.transform != null) {
      leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
    }
  }

  // src/events/listeners.ts
  function convertModifiers(event2) {
    let modifiers = [];
    if (event2.shiftKey)
      modifiers.push("Shift");
    if (event2.ctrlKey)
      modifiers.push("Control");
    if (event2.altKey)
      modifiers.push("Alt");
    if (event2.metaKey)
      modifiers.push("Command");
    return modifiers;
  }
  function getMouseButton(event2) {
    switch (event2.button) {
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
  function setupEventListeners(chassis) {
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
    window.addEventListener("click", (evt) => {
      let clickEvent = {
        "Click": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      let r1 = chassis.interrupt(JSON.stringify(clickEvent), []);
      let clapEvent = {
        "Clap": {
          "x": evt.clientX,
          "y": evt.clientY
        }
      };
      let r2 = chassis.interrupt(JSON.stringify(clapEvent), []);
      if (r1.prevent_default || r2.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("dblclick", (evt) => {
      let event2 = {
        "DoubleClick": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("mousemove", (evt) => {
      let button = window.current_button || "Left";
      let event2 = {
        "MouseMove": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": button,
          "modifiers": convertModifiers(evt)
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("wheel", (evt) => {
      let event2 = {
        "Wheel": {
          "x": evt.clientX,
          "y": evt.clientY,
          "delta_x": evt.deltaX,
          "delta_y": evt.deltaY,
          "modifiers": convertModifiers(evt)
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, { "passive": false, "capture": true });
    window.addEventListener("mousedown", (evt) => {
      let button = getMouseButton(evt);
      window.current_button = button;
      let event2 = {
        "MouseDown": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("mouseup", (evt) => {
      let event2 = {
        "MouseUp": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("mouseover", (evt) => {
      let event2 = {
        "MouseOver": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("mouseout", (evt) => {
      let event2 = {
        "MouseOut": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("contextmenu", (evt) => {
      let event2 = {
        "ContextMenu": {
          "x": evt.clientX,
          "y": evt.clientY,
          "button": getMouseButton(evt),
          "modifiers": convertModifiers(evt)
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("touchstart", (evt) => {
      let event2 = {
        "TouchStart": {
          "touches": getTouchMessages(evt.touches)
        }
      };
      Array.from(evt.changedTouches).forEach((touch) => {
        lastPositions.set(touch.identifier, { x: touch.clientX, y: touch.clientY });
      });
      let r1 = chassis.interrupt(JSON.stringify(event2), []);
      let clapEvent = {
        "Clap": {
          "x": evt.touches[0].clientX,
          "y": evt.touches[0].clientY
        }
      };
      let r2 = chassis.interrupt(JSON.stringify(clapEvent), []);
      if (r1.prevent_default || r2.prevent_default) {
        evt.preventDefault();
      }
    }, { "passive": true, "capture": true });
    window.addEventListener("touchmove", (evt) => {
      let touches = getTouchMessages(evt.touches);
      let event2 = {
        "TouchMove": {
          "touches": touches
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, { "passive": true, "capture": true });
    window.addEventListener("touchend", (evt) => {
      let event2 = {
        "TouchEnd": {
          "touches": getTouchMessages(evt.changedTouches)
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
      Array.from(evt.changedTouches).forEach((touch) => {
        lastPositions.delete(touch.identifier);
      });
    }, { "passive": true, "capture": true });
    window.addEventListener("keydown", (evt) => {
      if (document.activeElement != document.body) {
        return;
      }
      let event2 = {
        "KeyDown": {
          "key": evt.key,
          "modifiers": convertModifiers(evt),
          "is_repeat": evt.repeat
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("keyup", (evt) => {
      if (document.activeElement != document.body) {
        return;
      }
      let event2 = {
        "KeyUp": {
          "key": evt.key,
          "modifiers": convertModifiers(evt),
          "is_repeat": evt.repeat
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("keypress", (evt) => {
      if (document.activeElement != document.body) {
        return;
      }
      let event2 = {
        "KeyPress": {
          "key": evt.key,
          "modifiers": convertModifiers(evt),
          "is_repeat": evt.repeat
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), []);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("drop", async (evt) => {
      evt.stopPropagation();
      evt.preventDefault();
      if (document.activeElement != document.body) {
        return;
      }
      let file = evt.dataTransfer?.files[0];
      let bytes = await readFileAsByteArray(file);
      let event2 = {
        "DropFile": {
          "x": evt.clientX,
          "y": evt.clientY,
          "name": file.name,
          "mime_type": file.type,
          "size": file.size
        }
      };
      let res = chassis.interrupt(JSON.stringify(event2), bytes);
      if (res.prevent_default) {
        evt.preventDefault();
      }
    }, true);
    window.addEventListener("dragover", (evt) => {
      evt.stopPropagation();
      evt.preventDefault();
      event.dataTransfer.dropEffect = "copy";
    }, { "passive": false, "capture": true });
  }
  function readFileAsByteArray(file) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = (event2) => {
        if (event2.target && event2.target.result instanceof ArrayBuffer) {
          const arrayBuffer = event2.target.result;
          const byteArray = new Uint8Array(arrayBuffer);
          resolve(byteArray);
        } else {
          reject(new Error("File reading did not return an ArrayBuffer"));
        }
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsArrayBuffer(file);
    });
  }

  // src/index.ts
  var objectManager = new ObjectManager(SUPPORTED_OBJECTS);
  var messages;
  var nativePool = new NativeElementPool(objectManager);
  var textDecoder = new TextDecoder();
  var initializedChassis = false;
  function mount(selector_or_element, extensionlessUrl) {
    let link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = "pax-interface-web.css";
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
      await glueCodeModule.default(wasmArrayBuffer);
      let chassis = await glueCodeModule.pax_init();
      window.chassis = chassis;
      let get_latest_memory = glueCodeModule.wasm_memory;
      return { chassis, get_latest_memory };
    } catch (err) {
      throw new Error(`Failed to load WASM module: ${err}`);
    }
  }
  async function startRenderLoop(extensionlessUrl, mount2) {
    try {
      let { chassis, get_latest_memory } = await loadWasmModule(extensionlessUrl);
      nativePool.attach(chassis, mount2);
      requestAnimationFrame(renderLoop.bind(renderLoop, chassis, mount2, get_latest_memory));
    } catch (error) {
      console.error("Failed to load or instantiate Wasm module:", error);
    }
  }
  function renderLoop(chassis, mount2, get_latest_memory) {
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
      };
      window.addEventListener("resize", resizeHandler);
      resizeHandler();
      setupEventListeners(chassis);
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
      } else if (unwrapped_msg["SliderCreate"]) {
        let msg = unwrapped_msg["SliderCreate"];
        let patch = objectManager2.getFromPool(ANY_CREATE_PATCH);
        patch.fromPatch(msg);
        nativePool.sliderCreate(patch);
      } else if (unwrapped_msg["SliderUpdate"]) {
        let msg = unwrapped_msg["SliderUpdate"];
        let patch = objectManager2.getFromPool(SLIDER_UPDATE_PATCH, objectManager2);
        patch.fromPatch(msg);
        nativePool.sliderUpdate(patch);
      } else if (unwrapped_msg["SliderDelete"]) {
        let msg = unwrapped_msg["SliderDelete"];
        nativePool.sliderDelete(msg);
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
      } else if (unwrapped_msg["TextboxCreate"]) {
        let msg = unwrapped_msg["TextboxCreate"];
        let patch = objectManager2.getFromPool(ANY_CREATE_PATCH);
        patch.fromPatch(msg);
        nativePool.textboxCreate(patch);
      } else if (unwrapped_msg["TextboxUpdate"]) {
        let msg = unwrapped_msg["TextboxUpdate"];
        let patch = objectManager2.getFromPool(TEXTBOX_UPDATE_PATCH, objectManager2);
        patch.fromPatch(msg, nativePool.registeredFontFaces);
        nativePool.textboxUpdate(patch);
      } else if (unwrapped_msg["TextboxDelete"]) {
        let msg = unwrapped_msg["TextboxDelete"];
        nativePool.textboxDelete(msg);
      } else if (unwrapped_msg["RadioSetCreate"]) {
        let msg = unwrapped_msg["RadioSetCreate"];
        let patch = objectManager2.getFromPool(ANY_CREATE_PATCH);
        patch.fromPatch(msg);
        nativePool.radioSetCreate(patch);
      } else if (unwrapped_msg["RadioSetUpdate"]) {
        let msg = unwrapped_msg["RadioSetUpdate"];
        let patch = objectManager2.getFromPool(RADIOSET_UPDATE_PATCH, objectManager2);
        patch.fromPatch(msg, nativePool.registeredFontFaces);
        nativePool.radioSetUpdate(patch);
      } else if (unwrapped_msg["RadioSetDelete"]) {
        let msg = unwrapped_msg["RadioSetDelete"];
        nativePool.radioSetDelete(msg);
      } else if (unwrapped_msg["DropdownCreate"]) {
        let msg = unwrapped_msg["DropdownCreate"];
        let patch = objectManager2.getFromPool(ANY_CREATE_PATCH);
        patch.fromPatch(msg);
        nativePool.dropdownCreate(patch);
      } else if (unwrapped_msg["DropdownUpdate"]) {
        let msg = unwrapped_msg["DropdownUpdate"];
        let patch = objectManager2.getFromPool(DROPDOWN_UPDATE_PATCH, objectManager2);
        patch.fromPatch(msg, nativePool.registeredFontFaces);
        nativePool.dropdownUpdate(patch);
      } else if (unwrapped_msg["DropdownDelete"]) {
        let msg = unwrapped_msg["DropdownDelete"];
        nativePool.dropdownDelete(msg);
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
        nativePool.frameDelete(msg);
      } else if (unwrapped_msg["EventBlockerCreate"]) {
        let msg = unwrapped_msg["EventBlockerCreate"];
        let patch = objectManager2.getFromPool(ANY_CREATE_PATCH);
        patch.fromPatch(msg);
        nativePool.eventBlockerCreate(patch);
      } else if (unwrapped_msg["EventBlockerUpdate"]) {
        let msg = unwrapped_msg["EventBlockerUpdate"];
        let patch = objectManager2.getFromPool(EVENT_BLOCKER_UPDATE_PATCH);
        patch.fromPatch(msg);
        nativePool.eventBlockerUpdate(patch);
      } else if (unwrapped_msg["EventBlockerDelete"]) {
        let msg = unwrapped_msg["EventBlockerDelete"];
        nativePool.eventBlockerDelete(msg);
      } else if (unwrapped_msg["ImageLoad"]) {
        let msg = unwrapped_msg["ImageLoad"];
        let patch = objectManager2.getFromPool(IMAGE_LOAD_PATCH);
        patch.fromPatch(msg);
        nativePool.imageLoad(patch, chassis);
      } else if (unwrapped_msg["ScrollerCreate"]) {
        let msg = unwrapped_msg["ScrollerCreate"];
        let patch = objectManager2.getFromPool(ANY_CREATE_PATCH);
        patch.fromPatch(msg);
        nativePool.scrollerCreate(patch);
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
