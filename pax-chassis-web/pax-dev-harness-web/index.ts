
// const rust = import('./dist/pax_chassis_web');
import {PaxChassisWeb} from './dist/pax_chassis_web';
// @ts-ignore
import snarkdown from 'snarkdown';

const MOUNT_ID = "mount";
const NATIVE_OVERLAY_CLASS = "native-overlay";
const CANVAS_CLASS = "canvas";

const NATIVE_LEAF_CLASS = "native-leaf";
const NATIVE_CLIPPING_CLASS = "native-clipping";

const CLIP_PREFIX = "clip"

const registeredFontFaces = new Set<string>();

let layers: { "native": HTMLDivElement[], "canvas": HTMLCanvasElement[] } = { "native": [], "canvas": [] };

//handle {click, mouseover, ...} on {canvas element, native elements}
//for both virtual and native events, pass:
//  - global (screen) coordinates
//  - local (canvas) coordinates
//  - element offset (where within element, top-left-0,0)
//for native events, pass also an ID for the native element (e.g. DOM node) that
//can be used by engine to resolve virtual element
//This ID mechanism will also likely knock out most of the work for DOM element pooling/recycling

function main(wasmMod: typeof import('./dist/pax_chassis_web')) {

    initializeLayers(1);
    //Initialize chassis & engine
    let chassis = wasmMod.PaxChassisWeb.new();

    //Handle click events on canvas layer
    setupEventListeners(chassis,layers.canvas[0]);

    addEventListenersToNativeLayers(0, layers.native.length, chassis);

    //Kick off render loop
    requestAnimationFrame(renderLoop.bind(renderLoop, chassis))

}

function convertModifiers(event: MouseEvent | KeyboardEvent) {
    let modifiers = [];
    if (event.shiftKey) modifiers.push('Shift');
    if (event.ctrlKey) modifiers.push('Control');
    if (event.altKey) modifiers.push('Alt');
    if (event.metaKey) modifiers.push('Command');
    return modifiers;
}

function getMouseButton(event: MouseEvent) {
    switch (event.button) {
        case 0: return 'Left';
        case 1: return 'Middle';
        case 2: return 'Right';
        default: return 'Unknown';
    }
}



function setupEventListeners(chassis: any, layer: any) {
    // Need to make the layer focusable it can receive keyboard events
    layer.setAttribute('tabindex', '1000');
    layer.focus();

    let lastPositions = new Map<number, {x: number, y: number}>();
    // @ts-ignore
    function getTouchMessages(touchList: TouchList) {
        return Array.from(touchList).map(touch => {
            let lastPosition = lastPositions.get(touch.identifier) || { x: touch.clientX, y: touch.clientY };
            let delta_x = touch.clientX - lastPosition.x;
            let delta_y = touch.clientY - lastPosition.y;
            lastPositions.set(touch.identifier, { x: touch.clientX, y: touch.clientY });
            return {
                x: touch.clientX,
                y: touch.clientY,
                identifier: touch.identifier,
                delta_x: delta_x,
                delta_y: delta_y
            };
        });
    }

    // @ts-ignore
    layer.addEventListener('click', (evt) => {
        let clickEvent = {
            "Click": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        chassis.interrupt(JSON.stringify(clickEvent), []);
        let jabEvent = {
            "Jab": {
                "x": evt.clientX,
                "y": evt.clientY,
            }
        };
        chassis.interrupt(JSON.stringify(jabEvent), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('dblclick', (evt) => {
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
    // @ts-ignore
    layer.addEventListener('mousemove', (evt) => {
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
    // @ts-ignore
    layer.addEventListener('wheel', (evt) => {
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
        let scrollEvent = {
            "Scroll": {
                "delta_x": evt.deltaX,
                "delta_y": evt.deltaY,
            }
        };
        chassis.interrupt(JSON.stringify(scrollEvent), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('mousedown', (evt) => {
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
    // @ts-ignore
    layer.addEventListener('mouseup', (evt) => {
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
    // @ts-ignore
    layer.addEventListener('mouseover', (evt) => {
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
    // @ts-ignore
    layer.addEventListener('mouseout', (evt) => {
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
    // @ts-ignore
    layer.addEventListener('contextmenu', (evt) => {
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
    // @ts-ignore
    layer.addEventListener('touchstart', (evt) => {
        let event = {
            "TouchStart": {
                "touches": getTouchMessages(evt.touches)
            }
        };
        Array.from(evt.changedTouches).forEach(touch => { // @ts-ignore
            lastPositions.set(touch.identifier, { x: touch.clientX, y: touch.clientY });
        });
        chassis.interrupt(JSON.stringify(event), []);

        let jabEvent = {
            "Jab": {
                "x": evt.touches[0].clientX,
                "y": evt.touches[0].clientY,
            }
        };
        chassis.interrupt(JSON.stringify(jabEvent), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('touchmove', (evt) => {
        let touches = getTouchMessages(evt.touches);
        let event = {
            "TouchMove": {
                "touches": touches
            }
        };
        chassis.interrupt(JSON.stringify(event), []);

        let scrollEvent = {
            "Scroll": {
                "delta_x": touches[0].delta_x,
                "delta_y": touches[0].delta_y,
            }
        };
        chassis.interrupt(JSON.stringify(scrollEvent), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('touchend', (evt) => {
        let event = {
            "TouchEnd": {
                "touches": getTouchMessages(evt.changedTouches)
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
        Array.from(evt.changedTouches).forEach(touch => { // @ts-ignore
            lastPositions.delete(touch.identifier);
        });
    }, true);
    // @ts-ignore
    layer.addEventListener('keydown', (evt) => {
        let event = {
            "KeyDown": {
                "key": evt.key,
                "modifiers": convertModifiers(evt),
                "is_repeat": evt.repeat
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('keyup', (evt) => {
        let event = {
            "KeyUp": {
                "key": evt.key,
                "modifiers": convertModifiers(evt),
                "is_repeat": evt.repeat
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('keypress', (evt) => {
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


function addEventListenersToNativeLayers(starting_index: number, count: number, chassis: PaxChassisWeb){
    for (let i = starting_index; i < starting_index+count; i++) {
        setupEventListeners(chassis,layers.native[i]);
    }
}

function initializeLayers(num: number){
    let mount = document.querySelector("#" + MOUNT_ID); // FUTURE: make more general; see approach used by Vue & React
    // Set the position of the container to relative
    // @ts-ignore
    mount.style.position = 'relative';

    let starting_index = layers.canvas.length;

    for (let i = 0; i < num; i++) {
        let index = starting_index + i;
        //Create canvas element for piet drawing.
        //Note that width and height are set by the chassis each frame.
        let canvas = document.createElement("canvas");
        canvas.className = CANVAS_CLASS
        canvas.id = CANVAS_CLASS + "_" + index.toString();
        layers.canvas.push(canvas)

        if(index != 0) {
            // Ignore pointer events on the canvas
            canvas.style.pointerEvents = 'none';
        }

        //Create layer for native (DOM) rendering
        let nativeLayer = document.createElement("div");
        nativeLayer.className = NATIVE_OVERLAY_CLASS;
        nativeLayer.id = NATIVE_OVERLAY_CLASS +"_"+ index.toString();
        nativeLayer.style.pointerEvents = 'none';
        layers.native.push(nativeLayer)

        //Attach layers to mount
        //FIRST-APPLIED IS LOWEST
        mount?.appendChild(canvas)
        mount?.appendChild(nativeLayer);
    }
}

function getStringIdFromClippingId(prefix: string, id_chain: number[]) {
    return prefix + "_" + id_chain.join("_");
}

function escapeHtml(content: string){
    return new Option(content).innerHTML;
}

function clearCanvases(){
    for (let i = 0; i < layers.canvas.length; i++) {
        let canvas = layers.canvas[i];
        let context = canvas.getContext('2d');
        if (context) {
            context.clearRect(0, 0, canvas.width, canvas.height);
        }
    }
}

function renderLoop (chassis: PaxChassisWeb) {
     clearCanvases()
     let messages : string = chassis.tick();
     messages = JSON.parse(messages);

     // @ts-ignore
     processMessages(messages, chassis);
     messages;
     requestAnimationFrame(renderLoop.bind(renderLoop, chassis))
}

enum TextAlignHorizontal {
    Left = "Left",
    Center = "Center",
    Right = "Right",
}

function getJustifyContent(horizontalAlignment: string): string {
    switch (horizontalAlignment) {
        case TextAlignHorizontal.Left:
            return 'flex-start';
        case TextAlignHorizontal.Center:
            return 'center';
        case TextAlignHorizontal.Right:
            return 'flex-end';
        default:
            return 'flex-start';
    }
}

function getTextAlign(paragraphAlignment: string): string {
    switch (paragraphAlignment) {
        case TextAlignHorizontal.Left:
            return 'left';
        case TextAlignHorizontal.Center:
            return 'center';
        case TextAlignHorizontal.Right:
            return 'right';
        default:
            return 'left';
    }
}

enum TextAlignVertical {
    Top = "Top",
    Center = "Center",
    Bottom = "Bottom",
}

function getAlignItems(verticalAlignment: string): string {
    switch (verticalAlignment) {
        case TextAlignVertical.Top:
            return 'flex-start';
        case TextAlignVertical.Center:
            return 'center';
        case TextAlignVertical.Bottom:
            return 'flex-end';
        default:
            return 'flex-start';
    }
}


class NativeElementPool {
    private textNodes : any = {};
    private clippingNodes : any = {};
    private clippingValueCache : any = {};

    textCreate(patch: AnyCreatePatch) {
        console.assert(patch.id_chain != null);
        console.assert(patch.clipping_ids != null);

        let runningChain = document.createElement("div")
        let textChild = document.createElement('div');
        runningChain.appendChild(textChild);
        runningChain.setAttribute("class", NATIVE_LEAF_CLASS)

        let attachPoint = getAttachPointFromClippingIds(patch.clipping_ids);

        attachPoint?.appendChild(runningChain);

        // @ts-ignore
        this.textNodes[patch.id_chain] = runningChain;
    }

    textUpdate(patch: TextUpdatePatch) {

        //@ts-ignore
        window.textNodes = this.textNodes;
        // @ts-ignore
        let leaf = this.textNodes[patch.id_chain];
        console.assert(leaf !== undefined);

        // Handle depth
        let depth = patch.depth;
        if (depth != null && depth < layers.native.length) {
            let parentElement = leaf.parentElement;
            let newParent = layers.native[depth];
            if (parentElement != newParent) {
                parentElement.removeChild(leaf);
                newParent?.appendChild(leaf);
            }
        }

        let textChild = leaf.firstChild;

        // Apply TextStyle from patch.style
        if (patch.style) {
            const style = patch.style;
            if (style.font) {
                style.font.applyFontToDiv(leaf);
            }
            if (style.fill) {
                let newValue = "";
                if(style.fill.Rgba != null) {
                    let p = style.fill.Rgba;
                    newValue = `rgba(${p[0]! * 255.0},${p[1]! * 255.0},${p[2]! * 255.0},${p[3]! * 255.0})`;
                } else {
                    let p = style.fill.Hsla!;
                    newValue = `hsla(${p[0]! * 255.0},${p[1]! * 255.0},${p[2]! * 255.0},${p[3]! * 255.0})`;
                }
                textChild.style.color = newValue;
            }
            if (style.font_size) {
                textChild.style.fontSize = style.font_size + "px";
            }
            if (style.underline != null) {
                textChild.style.textDecoration = style.underline ? 'underline' : 'none';
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

        // Apply the content
        if (patch.content != null) {
            textChild.innerHTML = snarkdown(patch.content);

            // Apply the link styles if they exist
            if (patch.style_link) {
                let linkStyle = patch.style_link;
                const links = textChild.querySelectorAll('a');
                links.forEach((link: HTMLDivElement) => {
                    if (linkStyle.font) {
                        linkStyle.font.applyFontToDiv(link);
                    }
                    if (linkStyle.fill) {
                        let newValue = "";
                        if(linkStyle.fill.Rgba != null) {
                            let p = linkStyle.fill.Rgba;
                            newValue = `rgba(${p[0]! * 255.0},${p[1]! * 255.0},${p[2]! * 255.0},${p[3]! * 255.0})`;
                        } else {
                            let p = linkStyle.fill.Hsla!;
                            newValue = `hsla(${p[0]! * 255.0},${p[1]! * 255.0},${p[2]! * 255.0},${p[3]! * 255.0})`;
                        }
                        link.style.color = newValue;
                    }

                    if (linkStyle.align_horizontal) {
                        leaf.style.display = "flex";
                        leaf.style.justifyContent = getJustifyContent(linkStyle.align_horizontal);
                    }
                    if (linkStyle.align_vertical) {
                        leaf.style.alignItems = getAlignItems(linkStyle.align_vertical);
                    }
                    if (linkStyle.align_multiline) {
                        textChild.style.textAlign = getTextAlign(linkStyle.align_multiline);
                    }
                    if (linkStyle.underline != null) {
                        link.style.textDecoration = linkStyle.underline ? 'underline' : 'none';
                    }
                });
            }
        }

        // Handle size_x and size_y
        if (patch.size_x != null) {
            leaf.style.width = patch.size_x + "px";
        }
        if (patch.size_y != null) {
            leaf.style.height = patch.size_y + "px";
        }

        // Handle transform
        if (patch.transform != null) {
            leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }
    }




    textDelete(id_chain: number[]) {
        // @ts-ignore
        let oldNode = this.textNodes[id_chain];
        //console.assert(oldNode !== undefined);
        if (oldNode){
            let parent = oldNode.parentElement;


            oldNode.style.pointerEvents = "none";
            requestAnimationFrame(() => {
                parent.removeChild(oldNode);
            })
            // @ts-ignore
            delete this.textNodes[id_chain];
        }
    }

    frameCreate(patch: AnyCreatePatch) {
        console.assert(patch.id_chain != null);
        console.assert(this.clippingNodes["id_chain"] === undefined);

        let attachPoint = getAttachPointFromClippingIds(patch.clipping_ids);

        let newClip = document.createElement("div");
        newClip.id = getStringIdFromClippingId("clip", patch.id_chain);
        newClip.classList.add(NATIVE_CLIPPING_CLASS);

        attachPoint!.appendChild(newClip);
    }

    frameUpdate(patch: FrameUpdatePatch) {
        //@ts-ignore
        let cacheContainer : FrameUpdatePatch = this.clippingValueCache[patch.id_chain] || new FrameUpdatePatch();

        let shouldRedraw = false;
        if (patch.size_x != null) {
            shouldRedraw = true;
            cacheContainer.size_x = patch.size_x
        }
        if (patch.size_y != null) {
            shouldRedraw = true;
            cacheContainer.size_y = patch.size_y
        }
        if (patch.transform != null) {
            shouldRedraw = true;
            cacheContainer.transform = patch.transform;
        }

        if (shouldRedraw) {
            let node : HTMLElement = document.querySelector("#" + getStringIdFromClippingId(CLIP_PREFIX, patch.id_chain!))!
            
            // Fallback and/or perf optimizer: `polygon` instead of `path`.
            let polygonDef = getQuadClipPolygonCommand(cacheContainer.size_x!, cacheContainer.size_y!, cacheContainer.transform!)
            node.style.clipPath = polygonDef;
            //@ts-ignore
            node.style.webkitClipPath = polygonDef;

            // PoC arbitrary path clipping (noticeably poorer perf in Firefox at time of authoring)
            // let pathDef = getQuadClipPathCommand(cacheContainer.size_x!, cacheContainer.size_y!, cacheContainer.transform!)
            // node.style.clipPath = pathDef;
            // //@ts-ignore
            // node.style.webkitClipPath = pathDef;
        }
        //@ts-ignore
        this.clippingValueCache[patch.id_chain] = cacheContainer;
    }

    frameDelete(id_chain: number[]) {
        // NOTE: this should be supported, and may cause a memory leak if left unaddressed;
        //       was likely unplugged during v0 implementation due to some deeper bug that was interfering with 'hello world'

        // let oldNode = this.textNodes.get(id_chain);
        // console.assert(oldNode !== undefined);
        // this.textNodes.delete(id_chain);

        // let nativeLayer = document.querySelector("#" + NATIVE_OVERLAY_CLASS);
        // nativeLayer?.removeChild(oldNode);
    }



    async imageLoad(patch: ImageLoadPatch, chassis: PaxChassisWeb) {

        let path = patch.path;
        let id_chain = patch.id_chain;
        let image_data = await readImageToByteBuffer(path);
        let message = {
            "Image": {
                "Data": {
                    "id_chain": patch.id_chain,
                    "width": image_data.width,
                    "height": image_data.height,
                }
            }
        }
        chassis.interrupt(JSON.stringify(message), image_data.pixels);
    }
}


async function readImageToByteBuffer(imagePath: string): Promise<{ pixels: Uint8ClampedArray, width: number, height: number }> {
    const response = await fetch(imagePath);
    const blob = await response.blob();
    const img = await createImageBitmap(blob);
    const canvas = new OffscreenCanvas(img.width, img.height);
    const ctx = canvas.getContext('2d');

    // @ts-ignore
    ctx.drawImage(img, 0, 0, img.width, img.height);
    // @ts-ignore
    const imageData = ctx.getImageData(0, 0, img.width, img.height);
    let pixels = imageData.data;

    const width = img.width;
    const height = img.height;
    return { pixels, width, height };
}

class ImageLoadPatch {
    public id_chain: number[];
    public path: string;

    constructor(jsonMessage: any) {
        this.id_chain = jsonMessage["id_chain"];
        this.path = jsonMessage["path"];
    }
}

class TextStyle {
    public font?: Font;
    public fill?: ColorGroup;
    public font_size?: number;
    public underline?: boolean;
    public align_multiline?: TextAlignHorizontal;
    public align_horizontal?: TextAlignHorizontal;
    public align_vertical?: TextAlignVertical;

    constructor(styleMessage: any) {
        if (styleMessage["font"]) {
            const font = new Font();
            font.fromFontPatch(styleMessage["font"]);
            this.font = font;
        }
        this.fill = styleMessage["fill"];
        this.font_size = styleMessage["font_size"];
        this.underline = styleMessage["underline"];
        this.align_multiline = styleMessage["align_multiline"];
        this.align_horizontal = styleMessage["align_horizontal"];
        this.align_vertical = styleMessage["align_vertical"];
    }
}

//Type-safe wrappers around JSON representation
class TextUpdatePatch {
    public id_chain: number[];
    public content?: string;
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    public style?: TextStyle;
    public style_link?: TextStyle;
    public depth?: number;

    constructor(jsonMessage: any) {
        this.id_chain = jsonMessage["id_chain"];
        this.content = jsonMessage["content"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.depth = jsonMessage["depth"];

        const styleMessage = jsonMessage["style"];
        if (styleMessage) {
            this.style = new TextStyle(styleMessage);
        }

        const styleLinkMessage = jsonMessage["style_link"];
        if (styleLinkMessage) {
            this.style_link = new TextStyle(styleLinkMessage);
        }
    }
}



class ColorGroup {
    Hsla?: number[];
    Rgba?: number[];
}


class Font {
    public type?: string;
    public family?: string;
    public style?: FontStyle;
    public weight?: FontWeight;
    public url?: string; // for WebFontMessage
    public path?: string; // for LocalFontMessage

    mapFontWeight(fontWeight : FontWeight) {
        switch (fontWeight) {
            case FontWeight.Thin:
                return 100;
            case FontWeight.ExtraLight:
                return 200;
            case FontWeight.Light:
                return 300;
            case FontWeight.Normal:
                return 400;
            case FontWeight.Medium:
                return 500;
            case FontWeight.SemiBold:
                return 600;
            case FontWeight.Bold:
                return 700;
            case FontWeight.ExtraBold:
                return 800;
            case FontWeight.Black:
                return 900;
            default:
                return 400; // Return a default value if fontWeight is not found
        }
    }

    mapFontStyle(fontStyle: FontStyle) {
        switch (fontStyle) {
            case FontStyle.Normal:
                return 'normal';
            case FontStyle.Italic:
                return 'italic';
            case FontStyle.Oblique:
                return 'oblique';
            default:
                return 'normal'; // Return a default value if fontStyle is not found
        }
    }
    fromFontPatch(fontPatch: any) {
        const type = Object.keys(fontPatch)[0];
        const data = fontPatch[type];
        this.type = type;
        if (type === "System") {
            this.family = data.family;
            this.style = FontStyle[data.style as keyof typeof FontStyle];
            this.weight = FontWeight[data.weight as keyof typeof FontWeight];
        } else if (type === "Web") {
            this.family = data.family;
            this.url = data.url;
            this.style = FontStyle[data.style as keyof typeof FontStyle];
            this.weight = FontWeight[data.weight as keyof typeof FontWeight];
        } else if (type === "Local") {
            this.family = data.family;
            this.path = data.path;
            this.style = FontStyle[data.style as keyof typeof FontStyle];
            this.weight = FontWeight[data.weight as keyof typeof FontWeight];
        }
        this.registerFontFace();
    }


    private fontKey(): string {
        return `${this.type}-${this.family}-${this.style}-${this.weight}`;
    }

    registerFontFace() {
        const fontKey = this.fontKey();
        if (!registeredFontFaces.has(fontKey)) {
            registeredFontFaces.add(fontKey);

            if (this.type === "Web" && this.url && this.family) {
                if (this.url.includes("fonts.googleapis.com/css")) {
                    // Fetch the Google Fonts CSS file and create a <style> element to insert its content
                    fetch(this.url)
                        .then(response => response.text())
                        .then(css => {
                            const style = document.createElement("style");
                            style.textContent = css;
                            document.head.appendChild(style);
                        });
                } else {
                    const fontFace = new FontFace(this.family, `url(${this.url})`, {
                        style: this.style ? FontStyle[this.style] : undefined,
                        weight: this.weight ? FontWeight[this.weight] : undefined,
                    });

                    fontFace.load().then(loadedFontFace => {
                        document.fonts.add(loadedFontFace);
                    });
                }
            } else if (this.type === "Local" && this.path && this.family) {
                const fontFace = new FontFace(this.family, `url(${this.path})`, {
                    style: this.style ? FontStyle[this.style] : undefined,
                    weight: this.weight ? FontWeight[this.weight] : undefined,
                });

                fontFace.load().then(loadedFontFace => {
                    document.fonts.add(loadedFontFace);
                });
            }
        }
    }


    applyFontToDiv(div: HTMLDivElement) {
        if (this.family != undefined) {
            div.style.fontFamily = this.family;
        }
        if (this.style != undefined) {
            div.style.fontStyle = this.mapFontStyle(this.style);
        }
        if (this.weight != undefined) {
            div.style.fontWeight = String(this.mapFontWeight(this.weight));
        }
    }
}

enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}



class FrameUpdatePatch {
    public id_chain?: number[];
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    constructor(jsonMessage: any) {
        if(jsonMessage != null) { 
            this.id_chain = jsonMessage["id_chain"];
            this.size_x = jsonMessage["size_x"];
            this.size_y = jsonMessage["size_y"];
            this.transform = jsonMessage["transform"];
        }
    }
}

class AnyCreatePatch {
    public id_chain: number[];
    public clipping_ids: number[][];
    constructor(jsonMessage: any) {
        this.id_chain = jsonMessage["id_chain"];
        this.clipping_ids = jsonMessage["clipping_ids"];
    }
}

// function getQuadClipPathCommand(width: number, height: number, transform: number[]) {
//     let point0 = affineMultiply([0, 0], transform);
//     let point1 = affineMultiply([width, 0], transform);
//     let point2 = affineMultiply([width, height], transform);
//     let point3 = affineMultiply([0, height], transform);

//     let command = `path('M ${point0[0]} ${point0[1]} L ${point1[0]} ${point1[1]} L ${point2[0]} ${point2[1]} L ${point3[0]} ${point3[1]} Z')`
//     return command;
// }

//Rectilinear-affine alternative to `clip-path: path(...)` clipping.  Might be faster than `path`
function getQuadClipPolygonCommand(width: number, height: number, transform: number[]) {
    let point0 = affineMultiply([0, 0], transform);
    let point1 = affineMultiply([width, 0], transform);
    let point2 = affineMultiply([width, height], transform);
    let point3 = affineMultiply([0, height], transform);

    let polygon = `polygon(${point0[0]}px ${point0[1]}px, ${point1[0]}px ${point1[1]}px, ${point2[0]}px ${point2[1]}px, ${point3[0]}px ${point3[1]}px)`
    return polygon;
}

let nativePool = new NativeElementPool();

function processMessages(messages: any[], chassis: PaxChassisWeb) {

    messages?.forEach((unwrapped_msg) => {

        // if (Math.random() < .1) console.log(unwrapped_msg)
        if(unwrapped_msg["TextCreate"]) {
            let msg = unwrapped_msg["TextCreate"]
            nativePool.textCreate(new AnyCreatePatch(msg));
        }else if (unwrapped_msg["TextUpdate"]){
            let msg = unwrapped_msg["TextUpdate"]
            nativePool.textUpdate(new TextUpdatePatch(msg));
        }else if (unwrapped_msg["TextDelete"]) {
            let msg = unwrapped_msg["TextDelete"];
            nativePool.textDelete(msg)
        } else if(unwrapped_msg["FrameCreate"]) {
            let msg = unwrapped_msg["FrameCreate"]
            nativePool.frameCreate(new AnyCreatePatch(msg));
        }else if (unwrapped_msg["FrameUpdate"]){
            let msg = unwrapped_msg["FrameUpdate"]
            nativePool.frameUpdate(new FrameUpdatePatch(msg));
        }else if (unwrapped_msg["FrameDelete"]) {
            let msg = unwrapped_msg["FrameDelete"];
            nativePool.frameDelete(msg["id_chain"])
        }else if (unwrapped_msg["ImageLoad"]){
            let msg = unwrapped_msg["ImageLoad"];
            nativePool.imageLoad(new ImageLoadPatch(msg), chassis)
        }else if (unwrapped_msg["LayerAdd"]){
            let msg = unwrapped_msg["LayerAdd"];
            let layersToAdd = msg["num_layers_to_add"];
            let old_length = layers.native.length;
            initializeLayers(layersToAdd);
            addEventListenersToNativeLayers(old_length, layersToAdd, chassis);
            let event = {
                "AddedLayer": {
                    "num_layers_added": layersToAdd,
                }
            }
            chassis.interrupt(JSON.stringify(event), []);
        }
    })
}

/// Our 2D affine transform comes across the wire as an array of
/// floats in column-major order, (a,b,c,d,e,f) representing the
/// augmented matrix:
///  | a c e |
///  | b d f |
///  | 0 0 1 |
///
///  In order to pack this into a CSS-ready matrix3d format, we must
///  imagine packing into the following matrix for a "dont-care Z"
///
///  | a c 0 e |
///  | b d 0 f |
///  | 0 0 1 0 | //note that 1 will preserve a dont-care z, vs 0 will 'flatten' it
///  | 0 0 0 1 |
///
///  and then unroll into a comma-separated list, following column-major order
///
function packAffineCoeffsIntoMatrix3DString(coeffs: number[]) : string {
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

function packAffineCoeffsIntoMatrix2DString(coeffs: number[]) : string {
    return "matrix(" + [
        //begin column 0
        coeffs[0].toFixed(6),
        coeffs[1].toFixed(6),
        //begin column 1
        coeffs[2].toFixed(6),
        coeffs[3].toFixed(6),
        //begin column 2
        coeffs[4].toFixed(6),
        coeffs[5].toFixed(6),
    ].join(",") + ")";
}

function getAttachPointFromClippingIds(clipping_ids: number[][]) {
    // If there's a clipping context, attach to it.  Otherwise, attach directly to the native element layer.
    let attachPoint = (() => {
        if (clipping_ids.length > 0) {
            let clippingLeaf = document.querySelector("#" + getStringIdFromClippingId(CLIP_PREFIX, clipping_ids[clipping_ids.length - 1]));
            console.assert(clippingLeaf != null);
            return clippingLeaf
        }else {
            return document.querySelector("#" + NATIVE_OVERLAY_CLASS+"_0")
        }
    })();
    return attachPoint;
}

//Required due to Safari bug, unable to clip DOM elements to SVG=>`transform: matrix(...)` elements; see https://bugs.webkit.org/show_bug.cgi?id=126207
//  and repro in this repo: `878576bf0e9`
//Work-around is to manually affine-multiply coordinates of relevant elements and plot as `Path`s (without `transform`) in SVG.
//
//For the point V [x,y]
//And the affine coefficients in column-major order, (a,b,c,d,e,f) representing the matrix M:
//  | a c e |
//  | b d f |
//  | 0 0 1 |
//Return the product `V * M`
// Given a matrix A∈ℝm×n and vector x∈ℝn the matrix-vector multiplication of A and x is defined as
// Ax:=x1a∗,1+x2a∗,2+⋯+xna∗,n
// where a∗,i is the ith column vector of A.
function affineMultiply(point: number[], matrix: number[]) : number[] {
    let x = point[0];
    let y = point[1];
    let a = matrix[0];
    let b = matrix[1];
    let c = matrix[2];
    let d = matrix[3];
    let e = matrix[4];
    let f = matrix[5];
    let xOut = a*x + c*y + e;
    let yOut = b*x + d*y + f;
    return [xOut, yOut];
}


// Wasm + TS Bootstrapping boilerplate
async function load() {
    main(await import('./dist/pax_chassis_web'));
}
load().then();