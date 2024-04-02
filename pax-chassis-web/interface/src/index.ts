import type {PaxChassisWeb, InitOutput, initSync} from "./types/pax-chassis-web";

// @ts-ignore
import {ObjectManager} from "./pools/object-manager";
import {
    ANY_CREATE_PATCH,
    BUTTON_UPDATE_PATCH,
    CHECKBOX_UPDATE_PATCH,
    FRAME_UPDATE_PATCH,
    IMAGE_LOAD_PATCH, OCCLUSION_UPDATE_PATCH, SCROLLER_UPDATE_PATCH,
    SUPPORTED_OBJECTS,
    TEXTBOX_UPDATE_PATCH,
    TEXT_UPDATE_PATCH
} from "./pools/supported-objects";
import {NativeElementPool} from "./classes/native-element-pool";
import {AnyCreatePatch} from "./classes/messages/any-create-patch";
import {TextUpdatePatch} from "./classes/messages/text-update-patch";
import {CheckboxUpdatePatch} from "./classes/messages/checkbox-update-patch";
import {FrameUpdatePatch} from "./classes/messages/frame-update-patch";
import {ImageLoadPatch} from "./classes/messages/image-load-patch";
import {ScrollerUpdatePatch} from "./classes/messages/scroller-update-patch";
import {setupEventListeners} from "./events/listeners";
import "./styles/pax-web.css";
import { OcclusionUpdatePatch } from "./classes/messages/occlusion-update-patch";
import { ButtonUpdatePatch } from "./classes/messages/button-update-patch";
import { TextboxUpdatePatch } from "./classes/messages/textbox-update-patch";

let objectManager = new ObjectManager(SUPPORTED_OBJECTS);
let messages : any[];
let nativePool = new NativeElementPool(objectManager);
let textDecoder = new TextDecoder();
let isMobile = false;
let initializedChassis = false;

export function mount(selector_or_element: string | Element, extensionlessUrl: string) {

    //Inject CSS
    let link = document.createElement('link')
    link.rel = 'stylesheet'
    link.href = 'pax-chassis-web-interface.css'
    document.head.appendChild(link)

    let mount: Element;
    if (typeof selector_or_element === "string") {
        mount = document.querySelector(selector_or_element) as Element;
    } else {
        mount = selector_or_element;
    }

    // Update to pass wasmUrl to bootstrap function
    if (mount) {
        startRenderLoop(extensionlessUrl, mount).then();
    } else {
        console.error("Unable to find mount element");
    }
}

async function loadWasmModule(extensionlessUrl: string): Promise<{ chassis: PaxChassisWeb, get_latest_memory: ()=>any }> {
    try {
        const glueCodeModule = await import(`${extensionlessUrl}.js`) as typeof import("./types/pax-chassis-web");

        const wasmBinary = await fetch(`${extensionlessUrl}_bg.wasm`);
        const wasmArrayBuffer = await wasmBinary.arrayBuffer();
        let _io = await glueCodeModule.default(wasmArrayBuffer);

        let chassis = glueCodeModule.PaxChassisWeb.new();
        let get_latest_memory = glueCodeModule.wasm_memory;

        return { chassis, get_latest_memory };
    } catch (err) {
        throw new Error(`Failed to load WASM module: ${err}`);
    }
}

async function startRenderLoop(extensionlessUrl: string, mount: Element) {
    try {
        let {chassis, get_latest_memory} = await loadWasmModule(extensionlessUrl);
        isMobile = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
        nativePool.build(chassis, isMobile, mount);
        requestAnimationFrame(renderLoop.bind(renderLoop, chassis, mount, get_latest_memory));
    } catch (error) {
        console.error("Failed to load or instantiate Wasm module:", error);
    }
}

function renderLoop (chassis: PaxChassisWeb, mount: Element, get_latest_memory: ()=>any) {
    nativePool.sendScrollerValues();
    nativePool.clearCanvases();

    const memorySliceSpec = chassis.tick();
    const latestMemory : WebAssembly.Memory = get_latest_memory();
    const memoryBuffer = new Uint8Array(latestMemory.buffer);

    // Extract the serialized data directly from memory
    const jsonString = textDecoder.decode(memoryBuffer.subarray(memorySliceSpec.ptr(), memorySliceSpec.ptr() + memorySliceSpec.len()));
    messages = JSON.parse(jsonString);

    if(!initializedChassis){
        let resizeHandler = () => {
            let width = mount.clientWidth;
            let height = mount.clientHeight;
            chassis.send_viewport_update(width, height);
            nativePool.baseOcclusionContext.updateCanvases(width, height);
        };
        window.addEventListener('resize', resizeHandler);
        resizeHandler();//Fire once manually to init viewport size & occlusion context
        setupEventListeners(chassis);
        initializedChassis = true;
    }

    //@ts-ignore
    processMessages(messages, chassis, objectManager);

    //draw canvas elements
    chassis.render();

    //necessary manual cleanup
    chassis.deallocate(memorySliceSpec);

    let jsonS = JSON.stringify(objectManager);
    let sizeInBytes = new TextEncoder().encode(jsonS).length;
    console.log("Object manager", sizeInBytes);

    jsonS = JSON.stringify(messages);
    sizeInBytes = new TextEncoder().encode(jsonS).length;
    console.log("messages size", sizeInBytes);

    jsonS = JSON.stringify(nativePool);
    sizeInBytes = new TextEncoder().encode(jsonS).length;
    console.log("nativePool size", sizeInBytes);

    requestAnimationFrame(renderLoop.bind(renderLoop, chassis, mount, get_latest_memory))
}

export function processMessages(messages: any[], chassis: PaxChassisWeb, objectManager: ObjectManager) {
    messages?.forEach((unwrapped_msg) => {
        if(unwrapped_msg["OcclusionUpdate"]) {
            let msg = unwrapped_msg["OcclusionUpdate"]
            let patch: OcclusionUpdatePatch = objectManager.getFromPool(OCCLUSION_UPDATE_PATCH);
            patch.fromPatch(msg);
            nativePool.occlusionUpdate(patch);
            objectManager.returnToPool(OCCLUSION_UPDATE_PATCH, patch);
        } else if(unwrapped_msg["TextCreate"]) {
            let msg = unwrapped_msg["TextCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.textCreate(patch);
            objectManager.returnToPool(ANY_CREATE_PATCH, patch);
        } else if (unwrapped_msg["TextUpdate"]){
            let msg = unwrapped_msg["TextUpdate"]
            let patch: TextUpdatePatch = objectManager.getFromPool(TEXT_UPDATE_PATCH, objectManager);
            patch.fromPatch(msg, nativePool.registeredFontFaces);
            nativePool.textUpdate(patch);
            objectManager.returnToPool(TEXT_UPDATE_PATCH, patch);
        }else if (unwrapped_msg["TextDelete"]) {
            let msg = unwrapped_msg["TextDelete"];
            nativePool.textDelete(msg)
        }
    })
}

