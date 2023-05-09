
// const rust = import('./dist/pax_chassis_web');
import {PaxChassisWeb} from './dist/pax_chassis_web';
import './fonts.css';
// @ts-ignore
import snarkdown from 'snarkdown';

const MOUNT_ID = "mount";
const NATIVE_OVERLAY_ID = "native-overlay";
const CANVAS_ID = "canvas";

const NATIVE_LEAF_CLASS = "native-leaf";
const NATIVE_CLIPPING_CLASS = "native-clipping";

const CLIP_PREFIX = "clip"

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

enum Alignment {
    Left = "Left",
    Center = "Center",
    Right = "Right",
}

function getJustifyContent(horizontalAlignment: string): string {
    switch (horizontalAlignment) {
        case Alignment.Left:
            return 'flex-start';
        case Alignment.Center:
            return 'center';
        case Alignment.Right:
            return 'flex-end';
        default:
            return 'flex-start';
    }
}

function getTextAlign(paragraphAlignment: string): string {
    switch (paragraphAlignment) {
        case Alignment.Left:
            return 'left';
        case Alignment.Center:
            return 'center';
        case Alignment.Right:
            return 'right';
        default:
            return 'left';
    }
}

enum VAlignment {
    Top = "Top",
    Center = "Center",
    Bottom = "Bottom",
}

function getAlignItems(verticalAlignment: string): string {
    switch (verticalAlignment) {
        case VAlignment.Top:
            return 'flex-start';
        case VAlignment.Center:
            return 'center';
        case VAlignment.Bottom:
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

        let textChild = leaf.firstChild;

        if(patch.boundingBox == "Fixed"){
            if (patch.size_x != null) {
                leaf.style.width = patch.size_x + "px";
            }
            if (patch.size_y != null) {
                leaf.style.height = patch.size_y + "px";
            }
            leaf.style.display = "flex";
            leaf.style.justifyContent = getJustifyContent(patch.horizontalAlignment);
            leaf.style.alignItems = getAlignItems(patch.verticalAlignment);

        } else if (patch.boundingBox == "Auto"){
            leaf.style.width = "fit-content";
            leaf.style.height = "fit-content";
        }

        textChild.style.textAlign = getTextAlign(patch.paragraphAlignment);

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
            leaf.style.color = newValue;
        }

        let suffix = ""
        if (patch.font.variant != null) {
            if(patch.font.variant != "Regular") {
                suffix = "-" + patch.font.variant
            }
        }

        if (patch.font.family != null) {
            leaf.style.fontFamily = patch.font.family + suffix
        }

        if (patch.font.size != null) {
            leaf.style.fontSize = patch.font.size + "px"
        }

        if (patch.content != null) {
            textChild.innerHTML = snarkdown(patch.content);
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
    public transform?: number[];
    public font: FontGroup;
    public fill: ColorGroup;
    public paragraphAlignment: string;
    public horizontalAlignment: string;
    public verticalAlignment: string;
    public boundingBox : string;

    constructor(jsonMessage: any) {
        this.font = jsonMessage["font"];
        this.fill = jsonMessage["fill"];
        this.id_chain = jsonMessage["id_chain"];
        this.content = jsonMessage["content"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.paragraphAlignment = jsonMessage["paragraph_alignment"];
        this.horizontalAlignment = jsonMessage["horizontal_alignment"];
        this.verticalAlignment = jsonMessage["vertical_alignment"];
        this.boundingBox = jsonMessage["bounding_box"];
    }
}

class ColorGroup {
    Hsla?: number[];
    Rgba?: number[];
}


class FontGroup {
    public family?: string;
    public variant?: string;
    public size?: number;
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