
// const rust = import('./dist/pax_chassis_web');
import {PaxChassisWeb} from './dist/pax_chassis_web';
// @ts-ignore
import snarkdown from 'snarkdown';

const MOUNT_ID = "mount";
const NATIVE_OVERLAY_ID = "native-overlay";
const CANVAS_ID = "canvas";

const NATIVE_LEAF_CLASS = "native-leaf";
const NATIVE_CLIPPING_CLASS = "native-clipping";

const CLIP_PREFIX = "clip"

const registeredFontFaces = new Set<string>();

//handle {click, mouseover, ...} on {canvas element, native elements}
//for both virtual and native events, pass:
//  - global (screen) coordinates
//  - local (canvas) coordinates
//  - element offset (where within element, top-left-0,0)
//for native events, pass also an ID for the native element (e.g. DOM node) that
//can be used by engine to resolve virtual element
//This ID mechanism will also likely knock out most of the work for DOM element pooling/recycling

function main(wasmMod: typeof import('./dist/pax_chassis_web')) {
    let mount = document.querySelector("#" + MOUNT_ID); // FUTURE: make more general; see approach used by Vue & React

    //Create layer for native (DOM) rendering
    let nativeLayer = document.createElement("div");
    nativeLayer.id = NATIVE_OVERLAY_ID;

    //Create canvas element for piet drawing.
    //Note that width and height are set by the chassis each frame.
    let canvas = document.createElement("canvas");
    canvas.id = CANVAS_ID;
    
    //Attach layers to mount
    //FIRST-APPLIED IS LOWEST
    mount?.appendChild(canvas);
    mount?.appendChild(nativeLayer);

    //Initialize chassis & engine
    let chassis = wasmMod.PaxChassisWeb.new();

    //Handle click events on native layer
    nativeLayer.addEventListener('click', (evt) => {
        let event = {
            "Click": {
                "x": evt.x,
                "y": evt.y,
            }
        }
        chassis.interrupt(JSON.stringify(event));
    }, true);

    //Handle scroll events on native layer
    nativeLayer.addEventListener('wheel', (evt) => {
        let event = {
            "Scroll": {
                "x": evt.x,
                "y": evt.y,
                "delta_x": evt.deltaX,
                "delta_y": evt.deltaY,
            }
        }
        chassis.interrupt(JSON.stringify(event));
    }, true);

    //Kick off render loop
    requestAnimationFrame(renderLoop.bind(renderLoop, chassis))

}

function getStringIdFromClippingId(prefix: string, id_chain: number[]) {
    return prefix + "_" + id_chain.join("_");
}

function escapeHtml(content: string){
    return new Option(content).innerHTML;
}

function renderLoop (chassis: PaxChassisWeb) {
     let messages : string = chassis.tick();
     messages = JSON.parse(messages);

     // @ts-ignore
     processMessages(messages);
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

        if (patch.font != null) {
            patch.font.applyFontToDiv(leaf);
        }

        let textChild = leaf.firstChild;
        if (patch.content != null) {
            textChild.innerHTML = snarkdown(patch.content);
            // Apply the link styles if they exist
            console.log(patch);
            if (patch.style_link != null) {
                let linkStyle = patch.style_link;
                const links = textChild.querySelectorAll('a');
                console.log(links);
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
                    if (linkStyle.size != null) {
                        link.style.fontSize = linkStyle.size + "px";
                    }
                    if (linkStyle.underline != null) {
                        link.style.textDecoration = linkStyle.underline ? 'underline' : 'none';
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

        if (patch.size != null) {
            textChild.style.fontSize = patch.size + "px";
        }

        if(patch.align_horizontal != null){
            leaf.style.display = "flex";
            leaf.style.justifyContent = getJustifyContent(patch.align_horizontal);
        }

        if(patch.align_vertical != null){
            leaf.style.alignItems = getAlignItems(patch.align_vertical);
        }

        if(patch.align_multiline != null){
            textChild.style.textAlign = getTextAlign(patch.align_multiline);
        } else if(patch.align_horizontal != null) {
            textChild.style.textAlign = getTextAlign(patch.align_horizontal);
        }

        if (patch.transform != null) {
            leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }

        if (patch.fill != null) {
            let newValue = "";
            if(patch.fill.Rgba != null) {
                let p = patch.fill.Rgba;
                newValue = `rgba(${p[0]! * 255.0},${p[1]! * 255.0},${p[2]! * 255.0},${p[3]! * 255.0})`;
            } else {
                let p = patch.fill.Hsla!;
                newValue = `hsla(${p[0]! * 255.0},${p[1]! * 255.0},${p[2]! * 255.0},${p[3]! * 255.0})`;
            }
            textChild.style.color = newValue;
        }

    }



    textDelete(id_chain: number[]) {

        // @ts-ignore
        let oldNode = this.textNodes[id_chain];
        console.assert(oldNode !== undefined);
        // @ts-ignore
        delete this.textNodes[id_chain];

        let nativeLayer = document.querySelector("#" + NATIVE_OVERLAY_ID);
        nativeLayer?.removeChild(oldNode);
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

        // let nativeLayer = document.querySelector("#" + NATIVE_OVERLAY_ID);
        // nativeLayer?.removeChild(oldNode);
    }
}



//Type-safe wrappers around JSON representation
class TextUpdatePatch {
    public id_chain: number[];
    public content?: string;
    public size_x?: number;
    public size_y?: number;
    public size? : number;
    public transform?: number[];
    public font?: Font;
    public fill: ColorGroup;
    public align_multiline: string;
    public align_horizontal: string;
    public align_vertical: string;
    public style_link? : LinkStyle;

    constructor(jsonMessage: any) {
        this.fill = jsonMessage["fill"];
        this.id_chain = jsonMessage["id_chain"];
        this.content = jsonMessage["content"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.align_multiline = jsonMessage["align_multiline"];
        this.align_horizontal = jsonMessage["align_horizontal"];
        this.align_vertical = jsonMessage["align_vertical"];
        this.size = jsonMessage["size"];

        const fontPatch = jsonMessage["font"];
        if(fontPatch){
            this.font = new Font();
            this.font.fromFontPatch(fontPatch);
        }

        let styleLinkPatch = jsonMessage["style_link"];
        if (styleLinkPatch) {
            this.style_link = new LinkStyle();
            this.style_link.fromFontPatch(jsonMessage["style_link"])
        }
    }
}

class ColorGroup {
    Hsla?: number[];
    Rgba?: number[];
}

class LinkStyle {
    public size? : number;
    public font?: Font;
    public fill?: ColorGroup;
    public underline?: boolean;

    fromFontPatch(linkStylePatch: any){
        const fontPatch = linkStylePatch["font"];
        if(fontPatch){
            this.font = new Font();
            this.font.fromFontPatch(fontPatch);
        }
        this.fill = linkStylePatch["fill"];
        this.underline = linkStylePatch["underline"];
        this.size = linkStylePatch["size"];
    }

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
        if (this.family) {
            div.style.fontFamily = this.family;
        }
        if (this.style) {
            div.style.fontStyle = this.mapFontStyle(this.style);
        }
        if (this.weight) {
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

function processMessages(messages: any[]) {

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
            return document.querySelector("#" + NATIVE_OVERLAY_ID)
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