import type {PaxChassisWeb} from "./types/pax-chassis-web";
import {ObjectManager} from "./pools/object-manager";
import {
    ANY_CREATE_PATCH,
    SLIDER_UPDATE_PATCH,
    BUTTON_UPDATE_PATCH,
    CHECKBOX_UPDATE_PATCH,
    DROPDOWN_UPDATE_PATCH,
    FRAME_UPDATE_PATCH,
    IMAGE_LOAD_PATCH, OCCLUSION_UPDATE_PATCH, SCROLLER_UPDATE_PATCH,
    SUPPORTED_OBJECTS,
    TEXTBOX_UPDATE_PATCH,
    TEXT_UPDATE_PATCH,
    RADIOSET_UPDATE_PATCH,
    EVENT_BLOCKER_UPDATE_PATCH,
} from "./pools/supported-objects";
import {NativeElementPool} from "./classes/native-element-pool";
import {AnyCreatePatch} from "./classes/messages/any-create-patch";
import {TextUpdatePatch} from "./classes/messages/text-update-patch";
import {CheckboxUpdatePatch} from "./classes/messages/checkbox-update-patch";
import {FrameUpdatePatch} from "./classes/messages/frame-update-patch";
import {RadioSetUpdatePatch} from "./classes/messages/radio-set-update-patch";
import {EventBlockerUpdatePatch} from "./classes/messages/event-blocker-update-patch";
import {ImageLoadPatch} from "./classes/messages/image-load-patch";
import {ScrollerUpdatePatch} from "./classes/messages/scroller-update-patch";
import {setupEventListeners} from "./events/listeners";
import "./styles/pax-web.css";
import { OcclusionUpdatePatch } from "./classes/messages/occlusion-update-patch";
import { ButtonUpdatePatch } from "./classes/messages/button-update-patch";
import { TextboxUpdatePatch } from "./classes/messages/textbox-update-patch";
import { DropdownUpdatePatch } from "./classes/messages/dropdown-update-patch";
import { SliderUpdatePatch } from "./classes/messages/slider-update-patch";

let objectManager = new ObjectManager(SUPPORTED_OBJECTS);
let messages : any[];
let nativePool = new NativeElementPool(objectManager);
let textDecoder = new TextDecoder();
let initializedChassis = false;

export function mount(selector_or_element: string | Element, extensionlessUrl: string) {

    //Inject CSS
    let link = document.createElement('link')
    link.rel = 'stylesheet'
    link.href = 'pax-cartridge.css'
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
        const glueCodeModule = await import(`${extensionlessUrl}.js`) as typeof import("./types/pax-cartridge");

        const wasmBinary = await fetch(`${extensionlessUrl}_bg.wasm`);
        const wasmArrayBuffer = await wasmBinary.arrayBuffer();
        await glueCodeModule.default(wasmArrayBuffer);

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
        nativePool.attach(chassis, mount);
        requestAnimationFrame(renderLoop.bind(renderLoop, chassis, mount, get_latest_memory));
    } catch (error) {
        console.error("Failed to load or instantiate Wasm module:", error);
    }
}

function renderLoop (chassis: PaxChassisWeb, mount: Element, get_latest_memory: ()=>any) {
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
        };
        window.addEventListener('resize', resizeHandler);
        resizeHandler();//Fire once manually to init viewport size & occlusion context
        setupEventListeners(chassis);
        initializedChassis = true;
    }

    processMessages(messages, chassis, objectManager);

    //draw canvas elements
    chassis.render();

    //necessary manual cleanup
    chassis.deallocate(memorySliceSpec);

    requestAnimationFrame(renderLoop.bind(renderLoop, chassis, mount, get_latest_memory))
}

export function processMessages(messages: any[], chassis: PaxChassisWeb, objectManager: ObjectManager) {
    messages?.forEach((unwrapped_msg) => {
        if(unwrapped_msg["OcclusionUpdate"]) {
            let msg = unwrapped_msg["OcclusionUpdate"]
            let patch: OcclusionUpdatePatch = objectManager.getFromPool(OCCLUSION_UPDATE_PATCH);
            patch.fromPatch(msg);
            nativePool.occlusionUpdate(patch);
        } else if(unwrapped_msg["ButtonCreate"]) {
            let msg = unwrapped_msg["ButtonCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.buttonCreate(patch);
        } else if (unwrapped_msg["ButtonUpdate"]){
            let msg = unwrapped_msg["ButtonUpdate"]
            let patch: ButtonUpdatePatch = objectManager.getFromPool(BUTTON_UPDATE_PATCH, objectManager);
            patch.fromPatch(msg, nativePool.registeredFontFaces);
            nativePool.buttonUpdate(patch);
        }else if (unwrapped_msg["ButtonDelete"]) {
            let msg = unwrapped_msg["ButtonDelete"];
            nativePool.buttonDelete(msg)
        } else if(unwrapped_msg["SliderCreate"]) {
            let msg = unwrapped_msg["SliderCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.sliderCreate(patch);
        } else if (unwrapped_msg["SliderUpdate"]){
            let msg = unwrapped_msg["SliderUpdate"]
            let patch: SliderUpdatePatch = objectManager.getFromPool(SLIDER_UPDATE_PATCH, objectManager);
            patch.fromPatch(msg);
            nativePool.sliderUpdate(patch);
        }else if (unwrapped_msg["SliderDelete"]) {
            let msg = unwrapped_msg["SliderDelete"];
            nativePool.sliderDelete(msg)
        }else if(unwrapped_msg["CheckboxCreate"]) {
            let msg = unwrapped_msg["CheckboxCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.checkboxCreate(patch);
        } else if (unwrapped_msg["CheckboxUpdate"]){
            let msg = unwrapped_msg["CheckboxUpdate"]
            let patch: CheckboxUpdatePatch = objectManager.getFromPool(CHECKBOX_UPDATE_PATCH, objectManager);
            patch.fromPatch(msg);
            nativePool.checkboxUpdate(patch);
        }else if (unwrapped_msg["CheckboxDelete"]) {
            let msg = unwrapped_msg["CheckboxDelete"];
            nativePool.checkboxDelete(msg)
        } else if(unwrapped_msg["TextboxCreate"]) {
            let msg = unwrapped_msg["TextboxCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.textboxCreate(patch);
        } else if (unwrapped_msg["TextboxUpdate"]){
            let msg = unwrapped_msg["TextboxUpdate"]
            let patch: TextboxUpdatePatch = objectManager.getFromPool(TEXTBOX_UPDATE_PATCH, objectManager);
            patch.fromPatch(msg, nativePool.registeredFontFaces);
            nativePool.textboxUpdate(patch);
        }else if (unwrapped_msg["TextboxDelete"]) {
            let msg = unwrapped_msg["TextboxDelete"];
            nativePool.textboxDelete(msg)
        }else if(unwrapped_msg["RadioSetCreate"]) {
            let msg = unwrapped_msg["RadioSetCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.radioSetCreate(patch);
        } else if (unwrapped_msg["RadioSetUpdate"]){
            let msg = unwrapped_msg["RadioSetUpdate"]
            let patch: RadioSetUpdatePatch = objectManager.getFromPool(RADIOSET_UPDATE_PATCH, objectManager);
            patch.fromPatch(msg, nativePool.registeredFontFaces);
            nativePool.radioSetUpdate(patch);
        }else if (unwrapped_msg["RadioSetDelete"]) {
            let msg = unwrapped_msg["RadioSetDelete"];
            nativePool.radioSetDelete(msg)
        } else if(unwrapped_msg["DropdownCreate"]) {
            let msg = unwrapped_msg["DropdownCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.dropdownCreate(patch);
        } else if (unwrapped_msg["DropdownUpdate"]){
            let msg = unwrapped_msg["DropdownUpdate"]
            let patch: DropdownUpdatePatch = objectManager.getFromPool(DROPDOWN_UPDATE_PATCH, objectManager);
            patch.fromPatch(msg, nativePool.registeredFontFaces);
            nativePool.dropdownUpdate(patch);
        } else if (unwrapped_msg["DropdownDelete"]) {
            let msg = unwrapped_msg["DropdownDelete"];
            nativePool.dropdownDelete(msg)
        } else if(unwrapped_msg["TextCreate"]) {
            let msg = unwrapped_msg["TextCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.textCreate(patch);
        } else if (unwrapped_msg["TextUpdate"]){
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
            nativePool.frameDelete(msg)
        } else if(unwrapped_msg["EventBlockerCreate"]) {
            let msg = unwrapped_msg["EventBlockerCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.eventBlockerCreate(patch);
        } else if (unwrapped_msg["EventBlockerUpdate"]){
            let msg = unwrapped_msg["EventBlockerUpdate"]
            let patch: EventBlockerUpdatePatch = objectManager.getFromPool(EVENT_BLOCKER_UPDATE_PATCH);
            patch.fromPatch(msg);
            nativePool.eventBlockerUpdate(patch);
        } else if (unwrapped_msg["EventBlockerDelete"]) {
            let msg = unwrapped_msg["EventBlockerDelete"];
            nativePool.eventBlockerDelete(msg)
        } else if (unwrapped_msg["ImageLoad"]){
            let msg = unwrapped_msg["ImageLoad"];
            let patch: ImageLoadPatch = objectManager.getFromPool(IMAGE_LOAD_PATCH);
            patch.fromPatch(msg);
            nativePool.imageLoad(patch, chassis)
        }else if(unwrapped_msg["ScrollerCreate"]) {
            let msg = unwrapped_msg["ScrollerCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.scrollerCreate(patch);
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

