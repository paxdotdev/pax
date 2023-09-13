// @ts-ignore
import {PaxChassisWeb, wasm_memory} from '../dist/pax_chassis_web';
import {ObjectManager} from "./pools/object-manager";
import {MOUNT_ID} from "./utils/constants";
import {
    ANY_CREATE_PATCH,
    FRAME_UPDATE_PATCH,
    IMAGE_LOAD_PATCH, SCROLLER_UPDATE_PATCH,
    SUPPORTED_OBJECTS,
    TEXT_UPDATE_PATCH
} from "./pools/supported-objects";
import {NativeElementPool} from "./classes/native-element-pool";
import {AnyCreatePatch} from "./classes/messages/any-create-patch";
import {TextUpdatePatch} from "./classes/messages/text-update-patch";
import {FrameUpdatePatch} from "./classes/messages/frame-update-patch";
import {ImageLoadPatch} from "./classes/messages/image-load-patch";
import {ScrollerUpdatePatch} from "./classes/messages/scroller-update-patch";
import {setupEventListeners} from "./events/listeners";
let initializedChassis = false;
let is_mobile_device = false;

// var stats = new Stats();
// stats.showPanel( 0 ); // 0: fps, 1: ms, 2: mb, 3+: custom
// document.body.appendChild( stats.dom );


// Init-once globals for garbage collector optimization
let objectManager = new ObjectManager(SUPPORTED_OBJECTS);
let messages : any[];
let nativePool = new NativeElementPool(objectManager);
let textDecoder = new TextDecoder();


//handle {click, mouseover, ...} on {canvas element, native elements}
//for both virtual and native events, pass:
//  - global (screen) coordinates
//  - local (canvas) coordinates
//  - element offset (where within element, top-left-0,0)
//for native events, pass also an ID for the native element (e.g. DOM node) that
//can be used by engine to resolve virtual element
//This ID mechanism will also likely knock out most of the work for DOM element pooling/recycling
// @ts-ignore
async function main(wasmMod: typeof import('../dist/pax_chassis_web')) {

    is_mobile_device = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);

    let chassis = await wasmMod.PaxChassisWeb.new();

    nativePool.build(chassis);

    requestAnimationFrame(renderLoop.bind(renderLoop, chassis))
}

function renderLoop (chassis: PaxChassisWeb) {

    //stats.begin();
    nativePool.sendScrollerValues();
    nativePool.clearCanvases();

    const memorySlice = chassis.tick();
    const memory = wasm_memory();
    const memoryBuffer = new Uint8Array(memory.buffer);

    // Extract the serialized data directly from memory
    const jsonString = textDecoder.decode(memoryBuffer.subarray(memorySlice.ptr(), memorySlice.ptr() + memorySlice.len()));
    messages = JSON.parse(jsonString);

     if(!initializedChassis){
         let mount = document.querySelector("#" + MOUNT_ID)!;
         window.addEventListener('resize', () => {
             let width = window.innerWidth;
             let height = window.innerHeight;
             chassis.send_viewport_update(width, height);
             nativePool.baseOcclusionContext.updateCanvases(width, height);
         });
         setupEventListeners(chassis, mount);
         initializedChassis = true;
     }
     //@ts-ignore
    processMessages(messages, chassis, objectManager);

    // //necessary manual cleanup
    chassis.deallocate(memorySlice);
    //stats.end();
    requestAnimationFrame(renderLoop.bind(renderLoop, chassis))
}

export function processMessages(messages: any[], chassis: PaxChassisWeb, objectManager: ObjectManager) {
    messages?.forEach((unwrapped_msg) => {
        if(unwrapped_msg["TextCreate"]) {
            let msg = unwrapped_msg["TextCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.textCreate(patch);
        }else if (unwrapped_msg["TextUpdate"]){
            let msg = unwrapped_msg["TextUpdate"]
            let patch: TextUpdatePatch = objectManager.getFromPool(TEXT_UPDATE_PATCH, objectManager);
            patch.fromPatch(msg, nativePool.registeredFontFaces);
            nativePool.textUpdate(patch);
        }else if (unwrapped_msg["TextDelete"]) {
            let msg = unwrapped_msg["TextDelete"];
            nativePool.textDelete(msg)
        } else if(unwrapped_msg["FrameCreate"]) {
            let msg = unwrapped_msg["FrameCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.frameCreate(patch);
        }else if (unwrapped_msg["FrameUpdate"]){
            let msg = unwrapped_msg["FrameUpdate"]
            let patch: FrameUpdatePatch = objectManager.getFromPool(FRAME_UPDATE_PATCH);
            patch.fromPatch(msg);
            nativePool.frameUpdate(patch);
        }else if (unwrapped_msg["FrameDelete"]) {
            let msg = unwrapped_msg["FrameDelete"];
            nativePool.frameDelete(msg["id_chain"])
        }else if (unwrapped_msg["ImageLoad"]){
            let msg = unwrapped_msg["ImageLoad"];
            let patch: ImageLoadPatch = objectManager.getFromPool(IMAGE_LOAD_PATCH);
            patch.fromPatch(msg);
            nativePool.imageLoad(patch, chassis)
        }else if(unwrapped_msg["ScrollerCreate"]) {
            let msg = unwrapped_msg["ScrollerCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.scrollerCreate(patch, chassis);
        }else if (unwrapped_msg["ScrollerUpdate"]){
            let msg = unwrapped_msg["ScrollerUpdate"]
            let patch : ScrollerUpdatePatch = objectManager.getFromPool(SCROLLER_UPDATE_PATCH);
            patch.fromPatch(msg);
            nativePool.scrollerUpdate(patch);
        }else if (unwrapped_msg["ScrollerDelete"]) {
            let msg = unwrapped_msg["ScrollerDelete"];
            nativePool.scrollerDelete(msg)
        }
    })
}


// Wasm + TS Bootstrapping boilerplate
async function load() {
    // @ts-ignore
    main(await import('../dist/pax_chassis_web'));
}
load().then();