
// const rust = import('./dist/pax_chassis_web');
import {PaxChassisWeb} from './dist/pax_chassis_web';

const NATIVE_OVERLAY_ID = "native-overlay";
const NATIVE_ELEMENT_CLASS = "native-element";

//handle {click, mouseover, ...} on {canvas element, native elements}
//for both virtual and native events, pass:
//  - global (screen) coordinates
//  - local (canvas) coordinates
//  - element offset (where within element, top-left-0,0)
//for native events, pass also an ID for the native element (e.g. DOM node) that
//can be used by engine to resolve virtual element
//This ID mechanism will also likely knock out most of the work for DOM element pooling/recycling

function main(wasmMod: typeof import('./dist/pax_chassis_web')) {
    console.log("All modules loaded");

    let mount = document.querySelector("#mount"); // TODO: make more general; see approach used by Vue & React

    //Create layer for mixed-mode rendering
    let mixedModeLayer = document.createElement("div");
    mixedModeLayer.id = NATIVE_OVERLAY_ID;

    //Create canvas element for piet drawing
    let canvas = document.createElement("canvas");
    canvas.id = "canvas";

    //Attach canvas to mount: first-applied is lowest
    mount?.appendChild(canvas);
    mount?.appendChild(mixedModeLayer);

    // <canvas id="canvas"></canvas>
    let chassis = wasmMod.PaxChassisWeb.new();

    requestAnimationFrame(renderLoop.bind(renderLoop, chassis))
}

function escapeHtml(content: string){
    return new Option(content).innerHTML;
}

function renderLoop (chassis: PaxChassisWeb) {
     let messages : string = chassis.tick();
     messages = JSON.parse(messages);
     // @ts-ignore
     processMessages(messages);
     //messages.length > 0 && Math.random() < 0.05 &&  console.log(messages);
     requestAnimationFrame(renderLoop.bind(renderLoop, chassis))
}
let doneOnce = false;


class NativeElementPool {
    private textNodes : any = {};
    private clippingNodes : any = {};



    textCreate(patch: AnyCreatePatch) {
        console.assert(patch.id_chain != null);
        let newNode = document.createElement("div");
        console.assert(this.textNodes["id_chain"] === undefined);
        // @ts-ignore
        this.textNodes[patch.id_chain] = newNode;
        newNode.setAttribute("class", NATIVE_ELEMENT_CLASS)

        //TODO: instead of nativeLayer, get a reference to the correct clipping container
        let nativeLayer = document.querySelector("#" + NATIVE_OVERLAY_ID);
        nativeLayer?.appendChild(newNode);
    }

    textUpdate(patch: TextUpdatePatch) {
        //@ts-ignore
        window.textNodes = this.textNodes;
        // @ts-ignore
        let existingNode = this.textNodes[patch.id_chain];
        console.assert(existingNode !== undefined);

        if (patch.content != null) {
            existingNode.innerText = patch.content;
        }
        if (patch.size_x != null) {
            existingNode.style.width = patch.size_x + "px";
        }
        if (patch.size_y != null) {
            existingNode.style.height = patch.size_y + "px";
        }
        if (patch.transform != null) {
            existingNode.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }
        //     span.innerText = msg.content;
        //
        //     span.style.transform = packAffineCoeffsIntoMatrix3DString(msg.transform);
        //     span.style.backgroundColor = "red";
        //     span.style.width = msg.bounds[0] + "px";
        //     span.style.height = msg.bounds[1] + "px";

    }

    textDelete(id_chain: number[]) {
        let oldNode = this.textNodes.get(id_chain);
        console.assert(oldNode !== undefined);
        this.textNodes.delete(id_chain);

        //TODO: instead of nativeLayer, get a reference to the correct clipping container
        let nativeLayer = document.querySelector("#" + NATIVE_OVERLAY_ID);
        nativeLayer?.removeChild(oldNode);
    }


    frameCreate(patch: AnyCreatePatch) {
        console.assert(patch.id_chain != null);
        let newNode = document.createElement("div");
        console.assert(this.textNodes["id_chain"] === undefined);
        // @ts-ignore
        this.textNodes[patch.id_chain] = newNode;
        newNode.setAttribute("class", NATIVE_ELEMENT_CLASS)

        //TODO: instead of nativeLayer, get a reference to the correct clipping container
        let nativeLayer = document.querySelector("#" + NATIVE_OVERLAY_ID);
        nativeLayer?.appendChild(newNode);
    }

    frameUpdate(patch: FrameUpdatePatch) {
        //@ts-ignore
        window.textNodes = this.textNodes;
        // @ts-ignore
        let existingNode = this.textNodes[patch.id_chain];
        console.assert(existingNode !== undefined);

        if (patch.size_x != null) {
            existingNode.style.width = patch.size_x + "px";
        }
        if (patch.size_y != null) {
            existingNode.style.height = patch.size_y + "px";
        }
        if (patch.transform != null) {
            existingNode.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }
        //     span.innerText = msg.content;
        //
        //     span.style.transform = packAffineCoeffsIntoMatrix3DString(msg.transform);
        //     span.style.backgroundColor = "red";
        //     span.style.width = msg.bounds[0] + "px";
        //     span.style.height = msg.bounds[1] + "px";

    }

    frameDelete(id_chain: number[]) {
        let oldNode = this.textNodes.get(id_chain);
        console.assert(oldNode !== undefined);
        this.textNodes.delete(id_chain);

        //TODO: instead of nativeLayer, get a reference to the correct clipping container
        let nativeLayer = document.querySelector("#" + NATIVE_OVERLAY_ID);
        nativeLayer?.removeChild(oldNode);
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


            // track an "upsert frame" while updating properties, filling sparse
            // Option<>al structs with new values.  Expose this sparse struct
            // for message-passing (the upsert frame happens to be exactly the message struct)
            // if (!span.style.transform) {
            //     span.innerText = msg.content;
            //
            //     span.style.transform = packAffineCoeffsIntoMatrix3DString(msg.transform);
            //     span.style.backgroundColor = "red";
            //     span.style.width = msg.bounds[0] + "px";
            //     span.style.height = msg.bounds[1] + "px";
            // }
        }else if (unwrapped_msg["TextDelete"]) {
            let msg = unwrapped_msg["TextDelete"];

            nativePool.frameDelete(msg["id_chain"])
        } else if(unwrapped_msg["FrameCreate"]) {
            let msg = unwrapped_msg["FrameCreate"]

            let id_chain = msg["id_chain"];
            let clipping_ids = msg["clipping_ids"];

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
        coeffs[0],
        coeffs[1],
        0,
        0,
        //begin column 1
        coeffs[2],
        coeffs[3],
        0,
        0,
        //begin column 2
        0,
        0,
        1,
        0,
        //begin column 3
        coeffs[4],
        coeffs[5],
        0,
        1
    ].join(",") + ")";
}


// Wasm + TS Bootstrapping boilerplate
async function load() {
    main(await import('./dist/pax_chassis_web'));
}

load().then();