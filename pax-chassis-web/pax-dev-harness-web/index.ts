
// const rust = import('./dist/pax_chassis_web');
import {PaxChassisWeb} from './dist/pax_chassis_web';

const NATIVE_OVERLAY_ID = "native-overlay";
const NATIVE_ELEMENT_CLASS = "native-element";
const CANVAS_ID = "canvas";
const CLIPPING_LAYER_ID = "clipping-layer";
const SVG_NAMESPACE = "http://www.w3.org/2000/svg";

//handle {click, mouseover, ...} on {canvas element, native elements}
//for both virtual and native events, pass:
//  - global (screen) coordinates
//  - local (canvas) coordinates
//  - element offset (where within element, top-left-0,0)
//for native events, pass also an ID for the native element (e.g. DOM node) that
//can be used by engine to resolve virtual element
//This ID mechanism will also likely knock out most of the work for DOM element pooling/recycling

function main(wasmMod: typeof import('./dist/pax_chassis_web')) {
    let mount = document.querySelector("#mount"); // TODO: make more general; see approach used by Vue & React

    //Create layer for native (DOM) rendering
    let nativeLayer = document.createElement("div");
    nativeLayer.id = NATIVE_OVERLAY_ID;

    //Create canvas element for piet drawing.
    //Note that width and height are set by the chassis each frame.
    let canvas = document.createElement("canvas");
    canvas.id = CANVAS_ID;

    //Create clipping layer (SVG) for native element clipping.
    //Note that width and height are set by the chassis each frame.
    let clippingLayer = document.createElementNS(SVG_NAMESPACE, "svg");
    clippingLayer.id = CLIPPING_LAYER_ID;

    let helloClip = document.createElementNS(SVG_NAMESPACE,"clipPath");
    // helloClip.setAttributeNS(SVG_NAMESPACE, "clipPathUnits", "objectBoundingBox");

    helloClip.id = "hello-clip";

    //<circle cx="100" cy="100" r="40"/> 
    let helloClipContents = document.createElementNS(SVG_NAMESPACE,"circle");
    helloClipContents.setAttributeNS(null,"cx", "0");
    helloClipContents.setAttributeNS(null,"cy", "0");
    helloClipContents.setAttributeNS(null,"r", "500");


    // let helloContents = document.createElementNS(SVG_NAMESPACE,"circle");
    // helloContents.setAttributeNS(null,"cx", "200");
    // helloContents.setAttributeNS(null,"cy", "200");
    // helloContents.setAttributeNS(null,"r", "500");
    // helloContents.setAttributeNS(null,"fill", "red");
    // helloContents.setAttributeNS(null,"color", "red");
    // helloContents.setAttributeNS(null,"stroke", "red");

    helloClip.appendChild(helloClipContents);
    clippingLayer.appendChild(helloClip);
    // clippingLayer.appendChild(helloContents);

    //Attach layers to mount
    //FIRST-APPLIED IS LOWEST

    mount?.appendChild(clippingLayer);
    mount?.appendChild(canvas);
    mount?.appendChild(nativeLayer);
    

    let chassis = wasmMod.PaxChassisWeb.new();

    requestAnimationFrame(renderLoop.bind(renderLoop, chassis))
}

function getStringIdFromClippingId(id_chain: number[]) {
    return "clip_" + id_chain.join("_");
}

function escapeHtml(content: string){
    return new Option(content).innerHTML;
}

function renderLoop (chassis: PaxChassisWeb) {
     let messages : string = chassis.tick();
     messages = JSON.parse(messages);

     // @ts-ignore
     processMessages(messages);
     requestAnimationFrame(renderLoop.bind(renderLoop, chassis))
     
}

    

class NativeElementPool {
    private textNodes : any = {};
    private clippingNodes : any = {};

    textCreate(patch: AnyCreatePatch) {
        console.assert(patch.id_chain != null);
        console.assert(patch.clipping_ids != null);

        // Native elements + clipping:
        // Consider a native element `C` underneath two clipping elements `A` and `B`
        //  A
        //  |
        //  B
        //  | 
        //  C
        // We must create a chain of nodes representing the clipping hierarchy, where our actual
        // native element is the leaf (terminus) of that chain (C)
        // The root (beginning, A) of the chain will have the dynamic transform and size _of the native element_ applied to it
        // Each nested node will simply fill its container 100%, with no additional dynamic transform
        // Any "native operations", e.g. updating text content, happen to the terminus/leaf node (C)
        //
        // Clip paths are assigned by CSS selector only (no lookup into `clippingNodes`).  Clipping ID is strictly
        // derivable from a clipping `id_chain`

        let runningChain = document.createElement("div")
        runningChain.setAttribute("class", NATIVE_ELEMENT_CLASS)

        patch.clipping_ids.forEach((id_chain) => {
            // console.log("assinging clipping id", id_chain)
            let newNode = document.createElement("div")
            newNode.setAttribute("class", "clipping-container")
            let path = `url(#${getStringIdFromClippingId(id_chain).replace("\"", "")})`;
            // newNode.style.clipPath = path;

            newNode.appendChild(runningChain)
            runningChain = newNode
        });        

        let nativeNode = document.createElement("div");
        console.assert(this.textNodes["id_chain"] == null);
        
        nativeNode.setAttribute("class", NATIVE_ELEMENT_CLASS)


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

        let selector = "." + NATIVE_ELEMENT_CLASS;
        let leaf = root.matches(selector) ? root : root.querySelector(selector);

        if (patch.content != null) {
            //Note: applied to LEAF
            leaf.innerText = patch.content;
        }
        if (patch.size_x != null) {
            root.style.width = patch.size_x + "px";
        }
        if (patch.size_y != null) {
            root.style.height = patch.size_y + "px";
        }
        if (patch.transform != null) {
            root.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
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
        let newNode = document.createElementNS(SVG_NAMESPACE, "clipPath");
        let innerNode = document.createElementNS(SVG_NAMESPACE, "rect");
        innerNode.setAttributeNS(SVG_NAMESPACE, "x", "0");
        innerNode.setAttributeNS(SVG_NAMESPACE, "y", "0");
        newNode.id = getStringIdFromClippingId(patch.id_chain);

        newNode.appendChild(innerNode);

        console.assert(this.clippingNodes["id_chain"] === undefined);
        // @ts-ignore
        this.clippingNodes[patch.id_chain] = newNode;

        let clippingLayer = document.querySelector("#" + CLIPPING_LAYER_ID);
        clippingLayer?.appendChild(newNode);
    }

    frameUpdate(patch: FrameUpdatePatch) {
        //@ts-ignore
        let existingNode = this.clippingNodes[patch.id_chain].firstChild;
        console.assert(existingNode !== undefined);

        if (patch.size_x != null) {

            existingNode.setAttributeNS(SVG_NAMESPACE, "width", patch.size_x);
        }
        if (patch.size_y != null) {

            existingNode.setAttributeNS(SVG_NAMESPACE, "height", patch.size_y);
            
        }
        if (patch.transform != null) {
            existingNode.setAttributeNS(SVG_NAMESPACE, "x", patch.transform[4]);
            existingNode.setAttributeNS(SVG_NAMESPACE, "y", patch.transform[5]);
            // existingNode.setAttributeNS(SVG_NAMESPACE, "transform", packAffineCoeffsIntoMatrix2DString(patch.transform));
            // existingNode.x = patch.transform[5];
            // existingNode.style.y = patch.transform[6];
            // existingNode.style.transform = packAffineCoeffsIntoMatrix2DString(patch.transform);
        }

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
    public id_chain: number[];
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    constructor(jsonMessage: any) {
        this.id_chain = jsonMessage["id_chain"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
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


// Wasm + TS Bootstrapping boilerplate
async function load() {
    main(await import('./dist/pax_chassis_web'));
}

load().then();