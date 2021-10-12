
// const rust = import('./dist/carbon_chassis_web');
import {CarbonChassisWeb} from './dist/carbon_chassis_web';

//
// async function getWasm() {
//
// }
// console.log("JS loaded");
// rust
//   .then(m => m.run())
//   .catch(console.error);

const MIXED_MODE_LAYER_ID = "mixed-mode-layer";
const MIXED_MODE_ELEMENT_CLASS = "mixed-mode-element";

function main(wasmMod: typeof import('./dist/carbon_chassis_web')) {
    console.log("All modules loaded");

    let mount = document.querySelector("#mount"); // TODO: make more general; see approach used by Vue & React

    //Create layer for mixed-mode rendering
    let mixedModeLayer = document.createElement("div");
    mixedModeLayer.id = MIXED_MODE_LAYER_ID;

    //Create canvas element for piet drawing
    let canvas = document.createElement("canvas");
    canvas.id = "canvas";

    //Attach canvas to mount: first-applied is lowest
    mount?.appendChild(canvas);
    mount?.appendChild(mixedModeLayer);


    // <canvas id="canvas"></canvas>
    let ccw = wasmMod.CarbonChassisWeb.new();


    requestAnimationFrame(renderLoop.bind(renderLoop, ccw))
}

function renderLoop (ccw: CarbonChassisWeb) {
     let messages = ccw.tick();
     processMessages(messages);
     requestAnimationFrame(renderLoop.bind(renderLoop, ccw))
}

function processMessages(messages: any[]) {
    // console.log("Got messages", messages);

    //TODO:  mount relative+absolute layer on top of render context
    //       create DOM elements (pooled) for each supported message type
    messages.forEach((msg) => {
        // console.log("Trying to pack: ", packAffineCoeffsIntoMatrix3DString(msg.transform));
        switch(msg.kind) {
            case "TextMessage":
                let span = getOrCreateSpan(msg.id);
                span.innerText = msg.content;
                span.style.backgroundColor = "red";
                span.style.transform = packAffineCoeffsIntoMatrix3DString(msg.transform);
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
        //column 0
        coeffs[0],
        coeffs[1],
        0,
        0,

        //column 1
        coeffs[2],
        coeffs[3],
        0,
        0,

        //column 2
        0,
        0,
        1,
        0,

        //column 3
        coeffs[4],
        coeffs[5],
        0,
        1
    ].join(",") + ")";
}

//TODO:  handle removal, recycling if needed
let spanPool : {[id:string]:HTMLSpanElement} = {}
function getOrCreateSpan(id: number) : HTMLSpanElement {
    return spanPool[id] || (()=>{
        spanPool[id] = document.createElement("span");
        spanPool[id].setAttribute("class", MIXED_MODE_ELEMENT_CLASS)
        let mixedModeLayer = document.querySelector("#" + MIXED_MODE_LAYER_ID);
        mixedModeLayer?.appendChild(spanPool[id]);
        return spanPool[id];
    })()
}



//TODO:  traverse through render_message_queue after each engine tick
//       render those messages as appropriate


//TODO:  should we port the request_animation_frame => tick logic
//       to live in ts instead of rust?
//       1. it's far cleaner to invoke rAF from TS
//       2. it should make it clean/clear how to pass data (tick() returns the MQ,
//          ... Can even receive an MQ for inbounds/input `tick(inbound_mq)`)



// Wasm + TS Bootstrapping boilerplate
async function load() {
    main(await import('./dist/carbon_chassis_web'));
}

load().then();