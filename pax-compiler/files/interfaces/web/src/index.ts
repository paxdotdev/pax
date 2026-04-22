import type {PaxChassisWeb} from "./types/pax-chassis-web";
import {ObjectManager} from "./pools/object-manager";
import {
    ANY_CREATE_PATCH,
    SLIDER_UPDATE_PATCH,
    BUTTON_UPDATE_PATCH,
    CHECKBOX_UPDATE_PATCH,
    DROPDOWN_UPDATE_PATCH,
    FRAME_UPDATE_PATCH,
    IMAGE_LOAD_PATCH, SCROLLER_UPDATE_PATCH,
    SUPPORTED_OBJECTS,
    TEXTBOX_UPDATE_PATCH,
    TEXT_UPDATE_PATCH,
    RADIO_LIST_UPDATE_PATCH,
    EVENT_BLOCKER_UPDATE_PATCH,
    NAVIGATION_PATCH,
    NATIVE_IMAGE_UPDATE_PATCH,
    SET_CURSOR_PATCH,
    SCREENSHOT_PATCH,
    NATIVE_MASK_UPDATE_PATCH,
} from "./pools/supported-objects";
import {NativeElementPool} from "./classes/native-element-pool";
import {AnyCreatePatch} from "./classes/messages/any-create-patch";
import {TextUpdatePatch} from "./classes/messages/text-update-patch";
import {CheckboxUpdatePatch} from "./classes/messages/checkbox-update-patch";
import {FrameUpdatePatch} from "./classes/messages/frame-update-patch";
import {RadioListUpdatePatch} from "./classes/messages/radio-list-update-patch";
import {EventBlockerUpdatePatch} from "./classes/messages/event-blocker-update-patch";
import {ImageLoadPatch} from "./classes/messages/image-load-patch";
import {ScrollerUpdatePatch} from "./classes/messages/scroller-update-patch";
import {setupEventListeners} from "./events/listeners";
import "./styles/pax-web.css";
import { ButtonUpdatePatch } from "./classes/messages/button-update-patch";
import { TextboxUpdatePatch } from "./classes/messages/textbox-update-patch";
import { DropdownUpdatePatch } from "./classes/messages/dropdown-update-patch";
import { SliderUpdatePatch } from "./classes/messages/slider-update-patch";
import { NavigationPatch } from "./classes/messages/navigation-patch";
import { NativeImageUpdatePatch } from "./classes/messages/native-image-update-patch";
import { YoutubeVideoUpdatePatch } from "./classes/messages/youtube-video-update-patch";
import { ScreenshotPatch } from "./classes/messages/screenshot-patch";
import { NativeMaskUpdatePatch } from "./classes/messages/native-mask-update-patch";
import { isIOSWebKitBrowser } from "./classes/surface-host-policy";
import { HIDDEN_TAB_FRAME_FALLBACK_MS } from "./utils/helpers";

let objectManager = new ObjectManager(SUPPORTED_OBJECTS);
let nativePool = new NativeElementPool(objectManager);
let initializedChassis = false;
let renderLoopStarting = false;
let renderLoopStarted = false;
let frameInProgress = false;
let hiddenTabPumpHandle: number | null = null;
let pendingAsyncInterruptFlush = false;
const perfTraceEnabled = typeof window !== "undefined" && new URLSearchParams(window.location.search).has("pax_scroll_perf");
let perfTraceSequence = 0;

function withProfileMeasure<T>(name: string, fn: () => T): T {
    if (!perfTraceEnabled || typeof performance === "undefined") {
        return fn();
    }
    let sequence = perfTraceSequence += 1;
    let startMark = `pax:${name}:${sequence}:start`;
    let endMark = `pax:${name}:${sequence}:end`;
    performance.mark(startMark);
    try {
        return fn();
    } finally {
        performance.mark(endMark);
        performance.measure(`pax:${name}`, startMark, endMark);
        performance.clearMarks(startMark);
        performance.clearMarks(endMark);
    }
}

export function mount(selector_or_element: string | Element, extensionlessUrl: string) {
    if (renderLoopStarting || renderLoopStarted) {
        console.warn("Pax render loop already started; ignoring duplicate mount() call.");
        return;
    }

    //Inject CSS
    let link = document.createElement('link')
    link.rel = 'stylesheet'
    link.href = 'pax-interface-web.css'
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

async function loadWasmModule(extensionlessUrl: string): Promise<{ chassis: PaxChassisWeb }> {
    try {
        const glueCodeModule = await import(`${extensionlessUrl}.js`) as typeof import("./types/pax-cartridge");

        const wasmBinary = await fetch(`${extensionlessUrl}_bg.wasm`);
        const wasmArrayBuffer = await wasmBinary.arrayBuffer();
        await glueCodeModule.default({module_or_path: wasmArrayBuffer});

        let chassis = await glueCodeModule.pax_init();
        window.chassis = chassis;

        return { chassis };
    } catch (err) {
        throw new Error(`Failed to load WASM module: ${err}`);
    }
}

async function startRenderLoop(extensionlessUrl: string, mount: Element) {
    if (renderLoopStarting || renderLoopStarted) {
        return;
    }
    renderLoopStarting = true;
    try {
        let {chassis} = await loadWasmModule(extensionlessUrl);
        nativePool.attach(chassis, mount);
        nativePool.setPostAsyncInterruptFlush(() => requestFrameFlush(chassis, mount));
        initializeChassis(chassis, mount);
        ensureHiddenTabPump(chassis, mount);
        renderLoopStarted = true;
        renderLoopStarting = false;
        requestAnimationFrame(renderLoop.bind(renderLoop, chassis, mount));
    } catch (error) {
        renderLoopStarting = false;
        console.error("Failed to load or instantiate Wasm module:", error);
    }
}

function initializeChassis(chassis: PaxChassisWeb, mount: Element) {
    if (initializedChassis) {
        return;
    }
    chassis.interrupt({
        "BrowserConfig": {
            "allow_scroller_vector_layers": true,
            "allow_nested_scroller_vector_layers": true,
        },
    }, []);
    let lastViewportWidth = -1;
    let lastViewportHeight = -1;
    let resizeHandler = () => {
        let root = document.documentElement;
        // Use the layout viewport as the authoritative app size. Do not relayout the entire scene
        // during iOS Safari toolbar collapse: delegated page scroll should reveal more of the page
        // through the browser-owned visual viewport, not by continuously changing engine layout.
        let width = window.innerWidth ?? root.clientWidth ?? mount.clientWidth;
        let height = window.innerHeight ?? root.clientHeight ?? mount.clientHeight;
        if (
            nativePool.hasActivePageScrollDelegation()
            && Math.abs(width - lastViewportWidth) <= 0.5
            && Math.abs(height - lastViewportHeight) > 0.5
        ) {
            // iOS Safari reports toolbar collapse as a height-only resize. If we relay that into
            // engine viewport changes while the root scroller is delegated to page scroll, the
            // scene relayout snaps the scroller back to an earlier position mid-gesture.
            return;
        }
        if (Math.abs(width - lastViewportWidth) <= 0.5 && Math.abs(height - lastViewportHeight) <= 0.5) {
            return;
        }
        lastViewportWidth = width;
        lastViewportHeight = height;
        chassis.send_viewport_update(width, height);
    };
    window.addEventListener('resize', resizeHandler);
    // Initialize viewport-dependent layout before the first engine tick so native/scroller hosts do
    // not bootstrap against a transient 0x0 viewport.
    resizeHandler();
    setupEventListeners(chassis);
    initializedChassis = true;
}

function ensureHiddenTabPump(chassis: PaxChassisWeb, mount: Element) {
    if (hiddenTabPumpHandle !== null) {
        return;
    }

    // Background tabs can throttle requestAnimationFrame heavily enough that designtime websocket
    // requests appear to hang. Keep a lightweight timer-driven pump alive so `pax dev` stays
    // responsive even when the inspected tab is not frontmost.
    const pumpHiddenFrame = () => {
        if (!document.hidden) {
            return;
        }
        requestFrameFlush(chassis, mount);
    };

    document.addEventListener("visibilitychange", pumpHiddenFrame);
    window.addEventListener("pax-designtime-wakeup", () => {
        if (!document.hidden) {
            return;
        }
        requestFrameFlush(chassis, mount);
    });
    hiddenTabPumpHandle = window.setInterval(pumpHiddenFrame, HIDDEN_TAB_FRAME_FALLBACK_MS);
}

function requestFrameFlush(chassis: PaxChassisWeb, mount: Element) {
    if (frameInProgress) {
        pendingAsyncInterruptFlush = true;
        return;
    }
    runFrame(chassis, mount);
}

function runFrame(chassis: PaxChassisWeb, mount: Element) {
    if (frameInProgress) {
        return;
    }
    frameInProgress = true;
    try {
        initializeChassis(chassis, mount);
        withProfileMeasure("renderLoopFrame", () => {
            nativePool.sampleFrameInputs();
            const messages = withProfileMeasure("tick", () => chassis.tick());
            withProfileMeasure("processMessages", () => {
                processMessages(messages, chassis, objectManager);
            });
            withProfileMeasure("syncRenderSurfaceLayoutsPhase", () => {
                nativePool.syncRenderSurfaceLayouts();
            });
            // draw canvas elements
            withProfileMeasure("render", () => {
                chassis.render();
            });
        });
    } finally {
        frameInProgress = false;
        if (pendingAsyncInterruptFlush) {
            pendingAsyncInterruptFlush = false;
            queueMicrotask(() => requestFrameFlush(chassis, mount));
        }
    }
}

function renderLoop (chassis: PaxChassisWeb, mount: Element) {
    initializeChassis(chassis, mount);
    runFrame(chassis, mount);

    requestAnimationFrame(renderLoop.bind(renderLoop, chassis, mount));
}


export function processMessages(messages: any[], chassis: PaxChassisWeb, objectManager: ObjectManager) {
    if (messages.length === 0) {
        return;
    }
    messages?.forEach((unwrapped_msg) => {
        if(unwrapped_msg["ShrinkLayersTo"] !== undefined) {
            let layers_needed = unwrapped_msg["ShrinkLayersTo"];
            nativePool.layers.shrinkTo(layers_needed);
        } else if(unwrapped_msg["NativeMaskUpdate"]) {
            let msg = unwrapped_msg["NativeMaskUpdate"];
            let patch: NativeMaskUpdatePatch = objectManager.getFromPool(NATIVE_MASK_UPDATE_PATCH);
            patch.fromPatch(msg);
            nativePool.nativeMaskUpdate(patch);
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
        }else if(unwrapped_msg["RadioListCreate"]) {
            let msg = unwrapped_msg["RadioListCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.radioListCreate(patch);
        } else if (unwrapped_msg["RadioListUpdate"]){
            let msg = unwrapped_msg["RadioListUpdate"]
            let patch: RadioListUpdatePatch = objectManager.getFromPool(RADIO_LIST_UPDATE_PATCH, objectManager);
            patch.fromPatch(msg, nativePool.registeredFontFaces);
            nativePool.radioListUpdate(patch);
        }else if (unwrapped_msg["RadioListDelete"]) {
            let msg = unwrapped_msg["RadioListDelete"];
            nativePool.radioListDelete(msg)
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
        }  else if(unwrapped_msg["NativeImageCreate"]) {
            let msg = unwrapped_msg["NativeImageCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.nativeImageCreate(patch);
        } else if (unwrapped_msg["NativeImageUpdate"]){
            let msg = unwrapped_msg["NativeImageUpdate"]
            let patch: NativeImageUpdatePatch = objectManager.getFromPool(NATIVE_IMAGE_UPDATE_PATCH, objectManager);
            patch.fromPatch(msg);
            nativePool.nativeImageUpdate(patch);
        }else if (unwrapped_msg["NativeImageDelete"]) {
            let msg = unwrapped_msg["NativeImageDelete"];
            nativePool.nativeImageDelete(msg)
        }else if(unwrapped_msg["YoutubeVideoCreate"]) {
            let msg = unwrapped_msg["YoutubeVideoCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.youtubeVideoCreate(patch);
        } else if (unwrapped_msg["YoutubeVideoUpdate"]){
            let msg = unwrapped_msg["YoutubeVideoUpdate"]
            let patch: YoutubeVideoUpdatePatch = objectManager.getFromPool(NATIVE_IMAGE_UPDATE_PATCH, objectManager);
            patch.fromPatch(msg);
            nativePool.youtubeVideoUpdate(patch);
        }else if (unwrapped_msg["YoutubeVideoDelete"]) {
            let msg = unwrapped_msg["YoutubeVideoDelete"];
            nativePool.youtubeVideoDelete(msg)
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
            queueMicrotask(async () => {
                await nativePool.imageLoad(patch, chassis);
            });
        }else if(unwrapped_msg["ScrollerCreate"]) {
            let msg = unwrapped_msg["ScrollerCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.scrollerCreate(patch);
        } else if (unwrapped_msg["ScrollerUpdate"]){
            let msg = unwrapped_msg["ScrollerUpdate"]
            let patch : ScrollerUpdatePatch = objectManager.getFromPool(SCROLLER_UPDATE_PATCH);
            patch.fromPatch(msg);
            nativePool.scrollerUpdate(patch);
        } else if (unwrapped_msg["ScrollerDelete"]) {
            let msg = unwrapped_msg["ScrollerDelete"];
            nativePool.scrollerDelete(msg)
        } else if (unwrapped_msg["Navigate"]) {
            let msg = unwrapped_msg["Navigate"];
            let patch : NavigationPatch = objectManager.getFromPool(NAVIGATION_PATCH);
            patch.fromPatch(msg);
            nativePool.navigate(patch)
        } else if (unwrapped_msg["SetCursor"]) {
            let msg = unwrapped_msg["SetCursor"];
            let patch : NavigationPatch = objectManager.getFromPool(SET_CURSOR_PATCH);
            patch.fromPatch(msg);
            nativePool.setCursor(patch)
        } else if (unwrapped_msg["Screenshot"]) {
            let msg = unwrapped_msg["Screenshot"];
            let patch: ScreenshotPatch = objectManager.getFromPool(SCREENSHOT_PATCH);
            patch.fromPatch(msg);
            nativePool.screenshot(patch, chassis);
        }
    });
}
