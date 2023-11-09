import type {PaxChassisWeb, InitOutput, initSync} from "./types/pax-chassis-web";

// @ts-ignore
import {ObjectManager} from "./pools/object-manager";
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
import "./styles/pax-web.css";

let objectManager = new ObjectManager(SUPPORTED_OBJECTS);
let messages : any[];
let nativePool = new NativeElementPool(objectManager);
let textDecoder = new TextDecoder();
let isMobile = false;
let initializedChassis = false;

export async function mount(selector_or_element: string | Element, extensionlessUrl: string) {

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
        let _io = glueCodeModule.initSync(wasmArrayBuffer);

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
        await nativePool.build(chassis, isMobile, mount);
        requestAnimationFrame(renderLoop.bind(renderLoop, chassis, mount, get_latest_memory));
    } catch (error) {
        console.error("Failed to load or instantiate Wasm module:", error);
    }
}

async function renderLoop (chassis: PaxChassisWeb, mount: Element, get_latest_memory: ()=>any) {
    nativePool.sendScrollerValues();
    nativePool.setCanvasDpi();

    chassis.clear();
    const memorySliceSpec = chassis.tick();
    const latestMemory : WebAssembly.Memory = get_latest_memory();
    const memoryBuffer = new Uint8Array(latestMemory.buffer);

    // Extract the serialized data directly from memory
    const jsonString = textDecoder.decode(memoryBuffer.subarray(memorySliceSpec.ptr(), memorySliceSpec.ptr() + memorySliceSpec.len()));
    messages = JSON.parse(jsonString);

    //@ts-ignore
    await processMessages(messages, chassis, objectManager);
    //necessary manual cleanup
    chassis.deallocate(memorySliceSpec);

    if(!initializedChassis){
        let resizeHandler = () => {
            let width = mount.clientWidth;
            let height = mount.clientHeight;
            chassis.send_viewport_update(width, height, window.devicePixelRatio);
            nativePool.baseOcclusionContext.updateCanvases(width, height);
        };
        window.addEventListener('resize', resizeHandler);
        requestAnimationFrame(() => {
            resizeHandler(); //Fire once manually to init viewport size & occlusion context
        });

        setupEventListeners(chassis, mount);
        initializedChassis = true;
    }


    requestAnimationFrame(renderLoop.bind(renderLoop, chassis, mount, get_latest_memory))
}

export async function processMessages(messages: any[], chassis: PaxChassisWeb, objectManager: ObjectManager) {
    for(const unwrapped_msg of messages) {
        if(unwrapped_msg["TextCreate"]) {
            let msg = unwrapped_msg["TextCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            await nativePool.textCreate(patch);
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
            await nativePool.imageLoad(patch, chassis)
        }else if(unwrapped_msg["ScrollerCreate"]) {
            let msg = unwrapped_msg["ScrollerCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            await nativePool.scrollerCreate(patch, chassis);
        }else if (unwrapped_msg["ScrollerUpdate"]){
            let msg = unwrapped_msg["ScrollerUpdate"]
            let patch : ScrollerUpdatePatch = objectManager.getFromPool(SCROLLER_UPDATE_PATCH);
            patch.fromPatch(msg);
            nativePool.scrollerUpdate(patch);
        }else if (unwrapped_msg["ScrollerDelete"]) {
            let msg = unwrapped_msg["ScrollerDelete"];
            nativePool.scrollerDelete(msg)
        }
    }
}

