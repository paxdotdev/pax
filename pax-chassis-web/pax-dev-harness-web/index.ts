
// const rust = import('./dist/pax_chassis_web');
import {PaxChassisWeb} from './dist/pax_chassis_web';

const MOUNT_ID = "mount";
const NATIVE_OVERLAY_ID = "native-overlay";
const CANVAS_ID = "canvas";
const CLIPPING_CONTAINER_ID = "clipping-container";

const NATIVE_ROOT_CLASS = "native-root";
const NATIVE_CLIPPING_CLASS = "native-clipping";
const NATIVE_TRANSFORM_CLASS = "native-transform";
const NATIVE_LEAF_CLASS = "native-leaf";

const SVG_NAMESPACE = "http://www.w3.org/2000/svg";

const CLIP_PREFIX = "clip"
const SVG_PREFIX = "svg"


//handle {click, mouseover, ...} on {canvas element, native elements}
//for both virtual and native events, pass:
//  - global (screen) coordinates
//  - local (canvas) coordinates
//  - element offset (where within element, top-left-0,0)
//for native events, pass also an ID for the native element (e.g. DOM node) that
//can be used by engine to resolve virtual element
//This ID mechanism will also likely knock out most of the work for DOM element pooling/recycling

function main(wasmMod: typeof import('./dist/pax_chassis_web')) {
    let mount = document.querySelector("#" + MOUNT_ID); // TODO: make more general; see approach used by Vue & React

    //Create layer for native (DOM) rendering
    let nativeLayer = document.createElement("div");
    nativeLayer.id = NATIVE_OVERLAY_ID;

    //Create canvas element for piet drawing.
    //Note that width and height are set by the chassis each frame.
    let canvas = document.createElement("canvas");
    canvas.id = CANVAS_ID;

    //Create clipping layer (SVG) for native element clipping.
    let clippingContainer = document.createElementNS(SVG_NAMESPACE, "svg");
    clippingContainer.id = CLIPPING_CONTAINER_ID;
    
    
    //Attach layers to mount
    //FIRST-APPLIED IS LOWEST
    mount?.appendChild(clippingContainer);
    mount?.appendChild(canvas);
    mount?.appendChild(nativeLayer);
    

    let chassis = wasmMod.PaxChassisWeb.new();

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

    //  document.querySelectorAll('.native-clipping').forEach((node : any)=>{ 
    //     //  let r = node.style.clipPath + "";
    //     //  node.style.clipPath = "";
    //     //  node.offsetWidth;
    //     //  node.style.clipPath = r; 
    //     let y = node.offsetWidth
    //     let x = node.getBoundingClientRect();
    //     if(Math.random() < .01)
    //         console.log(x)
    // })

//     document.querySelectorAll('rect').forEach((node : any)=>{ 
//         let r = node.getAttribute("transform");
//         node.setAttributeNS(null, "transform", "");
//         node.offsetWidth;
//         node.setAttributeNS(null, "transform", r);

//    })

// document.querySelectorAll('path').forEach((node : any)=>{ 
//     let r = node.getAttribute("d");
//     node.setAttributeNS(null, "d", "");
//     node.offsetWidth;
//     node.setAttributeNS(null, "d", r);
// })
     requestAnimationFrame(renderLoop.bind(renderLoop, chassis))
     
}

    

class NativeElementPool {
    private textNodes : any = {};
    private clippingNodes : any = {};
    private clippingValueCache : any = {};

    textCreate(patch: AnyCreatePatch) {
        console.assert(patch.id_chain != null);
        console.assert(patch.clipping_ids != null);

        //`mount`
        //  `svg-container`
        //    `svg_9_9` transform (CSS matrix3d, not SVG transform)
        //        `clip_9_9` clipPath container
        //            `rect` or `path`, actual clipping area
        //  `canvas`
        //  `native-overlay`
        //      `root` absolute size
        //      `clip-host` (or `scroll-host`)
        //      `clip-host` (or `scroll-host`)
        //      `transform-host`
        //      ..
        //      `leaf`

        let runningChain = document.createElement("div")
        runningChain.setAttribute("class", NATIVE_LEAF_CLASS)

        let transformNode = document.createElement("div");
        transformNode.setAttribute("class", NATIVE_TRANSFORM_CLASS);
        transformNode.appendChild(runningChain);
        runningChain = transformNode;


        //TODO: handle scrollhosts just like cliphosts
        patch.clipping_ids.forEach((id_chain) => {
            let newNode = document.createElement("div")
            newNode.setAttribute("class", NATIVE_CLIPPING_CLASS)
            let path = `url(#${getStringIdFromClippingId(CLIP_PREFIX, id_chain)})`;
            newNode.style.clipPath = path;///"url(#hello-clip)";

            newNode.appendChild(runningChain)
            runningChain = newNode
        });        

        
        
        let rootNode = document.createElement("div");
        rootNode.setAttribute("class", NATIVE_ROOT_CLASS);
        rootNode.appendChild(runningChain);
        runningChain = rootNode;
        
        //TODO: instead of nativeLayer, get a reference to the correct clipping container
        let nativeLayer = document.querySelector("#" + NATIVE_OVERLAY_ID);

        nativeLayer?.appendChild(runningChain);

        // @ts-ignore
        this.textNodes[patch.id_chain] = runningChain;
    }

    textUpdate(patch: TextUpdatePatch) {

        //@ts-ignore
        window.textNodes = this.textNodes;
        // @ts-ignore
        let root = this.textNodes[patch.id_chain];
        console.assert(root !== undefined);

        let leaf_selector = "." + NATIVE_LEAF_CLASS;
        let transform_selector = "." + NATIVE_TRANSFORM_CLASS;
        let clip_selector = "." + NATIVE_CLIPPING_CLASS;
        
        let leaf = root.matches(leaf_selector) ? root : root.querySelector(leaf_selector);
        let clip = root.matches(clip_selector) ? root : root.querySelector(clip_selector);
        let transform = root.matches(transform_selector) ? root : root.querySelector(transform_selector);
        
        //Note: applied to ROOT
        if (patch.size_x != null) {
            root.style.width = patch.size_x + "px";
        }
        if (patch.size_y != null) {
            root.style.height = patch.size_y + "px";
        }
        if (patch.transform != null) {
            let transformString = packAffineCoeffsIntoMatrix3DString(patch.transform);
            // root.style.transform = transformString
            transform.style.transform = transformString;
            // root.style.transform = "";
        }

        //Note: applied to LEAF
        if (patch.content != null) {
            leaf.innerText = patch.content;
        }
    }

    textDelete(id_chain: number[]) {
        
        let oldNode = this.textNodes.get(id_chain);
        console.assert(oldNode !== undefined);
        this.textNodes.delete(id_chain);

        let nativeLayer = document.querySelector("#" + NATIVE_OVERLAY_ID);
        nativeLayer?.removeChild(oldNode);
    }

    frameCreate(patch: AnyCreatePatch) {
        console.assert(patch.id_chain != null);
        console.assert(this.clippingNodes["id_chain"] === undefined);

        //  `svg-container`
        //    `svg_9_9` transform (CSS matrix3d, not SVG transform)
        //        `clip_9_9` clipPath container
        //            `rect` or `path`, actual clipping area

        // let newSvgHost = document.createElementNS(SVG_NAMESPACE, "svg");
        // newSvgHost.id = getStringIdFromClippingId(SVG_PREFIX, patch.id_chain);

        let newClipPath = document.createElementNS(SVG_NAMESPACE, "clipPath");
        newClipPath.id = getStringIdFromClippingId(CLIP_PREFIX,patch.id_chain);

        let newRawPath = document.createElementNS(SVG_NAMESPACE, "path");
        let command = "M 0 0";
        newRawPath.setAttributeNS(null, "d", command);

        // @ts-ignore
        this.clippingNodes[patch.id_chain] = newClipPath;

        let clippingContainer = document.querySelector("#" + CLIPPING_CONTAINER_ID);

        newClipPath.appendChild(newRawPath);
        clippingContainer?.appendChild(newClipPath);
    }

    frameUpdate(patch: FrameUpdatePatch) {
        //@ts-ignore
        let svgHost = this.clippingNodes[patch.id_chain];
        console.assert(svgHost !== undefined);

        let rawPath = svgHost.querySelector('path');

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
            let command = getQuadClipPathCommand(cacheContainer.size_x!, cacheContainer.size_y!, cacheContainer.transform!);
            rawPath.setAttributeNS(null, "d", command);
        }
        //@ts-ignore
        this.clippingValueCache[patch.id_chain] = cacheContainer;
    }

    frameDelete(id_chain: number[]) {
        // let oldNode = this.textNodes.get(id_chain);
        // console.assert(oldNode !== undefined);
        // this.textNodes.delete(id_chain);

        // //TODO: instead of nativeLayer, get a reference to the correct clipping container
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
    constructor(jsonMessage: any) {
        this.id_chain = jsonMessage["id_chain"];
        this.content = jsonMessage["content"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
    }
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


function getQuadClipPathCommand(width: number, height: number, transform: number[]) {
    let point0 = affineMultiply([0, 0], transform);
    let point1 = affineMultiply([width, 0], transform);
    let point2 = affineMultiply([width, height], transform);
    let point3 = affineMultiply([0, height], transform);

    let command = `M ${point0[0]} ${point0[1]} L ${point1[0]} ${point1[1]} L ${point2[0]} ${point2[1]} L ${point3[0]} ${point3[1]} Z`
    return command;
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
            nativePool.frameDelete(msg["id_chain"])
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