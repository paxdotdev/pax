import type {PaxChassisWeb} from "./types/pax-chassis-web";
import {ObjectManager} from "./pools/object-manager";
import {
    ANY_CREATE_PATCH,
    SLIDER_UPDATE_PATCH,
    BUTTON_UPDATE_PATCH,
    CHECKBOX_UPDATE_PATCH,
    DROPDOWN_UPDATE_PATCH,
    FRAME_UPDATE_PATCH,
    IMAGE_LOAD_PATCH,
    PHOTO_PICKER_UPDATE_PATCH,
    SCROLLER_UPDATE_PATCH,
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
import { PhotoPickerUpdatePatch } from "./classes/messages/photo-picker-update-patch";
import { TextboxUpdatePatch } from "./classes/messages/textbox-update-patch";
import { DropdownUpdatePatch } from "./classes/messages/dropdown-update-patch";
import { SliderUpdatePatch } from "./classes/messages/slider-update-patch";
import { NavigationPatch } from "./classes/messages/navigation-patch";
import { SetCursorPatch } from "./classes/messages/set-cursor-patch";
import { NativeImageUpdatePatch } from "./classes/messages/native-image-update-patch";
import { YoutubeVideoUpdatePatch } from "./classes/messages/youtube-video-update-patch";
import { ScreenshotPatch } from "./classes/messages/screenshot-patch";
import { NativeMaskUpdatePatch } from "./classes/messages/native-mask-update-patch";
import { isIOSWebKitBrowser } from "./classes/surface-host-policy";
import { HIDDEN_TAB_FRAME_FALLBACK_MS } from "./utils/helpers";
import { browserRouteLocation, replaceCurrentRouteHistoryState, serializeRouteLocation } from "./utils/route-location";
import { updateDocumentRouteMetadata } from "./utils/route-metadata";

import { FrameScheduler } from "./utils/frame-scheduler";

declare global { interface Window { chassis?: PaxChassisWeb; } }

let objectManager = new ObjectManager(SUPPORTED_OBJECTS);
let nativePool = new NativeElementPool(objectManager);
let initializedChassis = false;
let renderLoopStarting = false;
let renderLoopStarted = false;
let frameInProgress = false;
let hiddenTabPumpHandle: number | null = null;
let pendingAsyncInterruptFlush = false;
const frameScheduler = new FrameScheduler(new URLSearchParams(window.location.search).get("pax_suspended") === "1");

/** Suspend frame updates and drawing while retaining the mounted application.
 * Hosts may call this before Wasm finishes loading. Resuming schedules one frame;
 * elapsed-time-based animations can advance to the current time on resume.
 */
export function setSuspended(suspended: boolean) {
    frameScheduler.setSuspended(suspended);
}
let currentChassis: PaxChassisWeb | null = null;
let currentMount: Element | null = null;
let currentExtensionlessUrl: string | null = null;
let teardownEventListeners: (() => void) | null = null;
let teardownResizeHandler: (() => void) | null = null;
let teardownHiddenTabPump: (() => void) | null = null;
let teardownRouteLocationSync: (() => void) | null = null;
let liveSession: MountSession | null = null;
const perfTraceEnabled = typeof window !== "undefined" && new URLSearchParams(window.location.search).has("pax_scroll_perf");
let perfTraceSequence = 0;

export interface WebInitOptions {
    element: HTMLElement;
    cartridgeUrl: string;
    /** Application-defined bytes copied before initialization; maximum 64 KiB. */
    startupData?: Uint8Array;
    /** Canonical runnable root, e.g. "crate::App"; optional for a single root. */
    entry?: string;
    /** Standalone pages may delegate the root scroller to browser page scrolling. */
    pageScroll?: boolean;
    /** Opt into page history/title ownership. Embedded regions default to false. */
    documentRouting?: boolean;
    /** Cancels startup or closes the mounted region; cleanup remains retained. */
    signal?: AbortSignal;
}

/** Owns one mounted region. `closed` observes independently driven cleanup. */
export interface MountedApp { readonly closed: Promise<void>; close(): void; request(bytes: Uint8Array): Promise<Uint8Array>; }

class MountSession implements MountedApp {
    readonly closed: Promise<void>;
    private acknowledge!: () => void;
    private closing = false;
    private starting = true;
    private abort = () => this.close();
    private pump: (() => number) | null = null;
    readonly region: HTMLDivElement;
    constructor(readonly options: WebInitOptions) {
        this.closed = new Promise(resolve => { this.acknowledge = resolve; });
        this.region = document.createElement("div");
        this.region.className = "pax-mount";
        this.region.tabIndex = 0;
        options.element.appendChild(this.region);
        options.signal?.addEventListener("abort", this.abort, {once: true});
    }
    request(bytes: Uint8Array): Promise<Uint8Array> {
        if (this.closing || liveSession !== this || !currentChassis) return Promise.reject(new Error("application is closed"));
        if (!(bytes instanceof Uint8Array) || bytes.byteLength > 65536) return Promise.reject(new Error("request must be Uint8Array of at most 64 KiB"));
        return currentChassis.request(bytes).catch(error => { throw hostError(error); });
    }
    checkOpen() {
        if (this.closing || this.options.signal?.aborted) throw new DOMException("Pax initialization cancelled", "AbortError");
    }
    finishStartup() {
        this.starting = false;
        this.cleanupWake(this.pump?.() ?? 0);
    }
    installPump(pump: () => number) { this.pump = pump; }
    cleanupWake(pending: number) {
        if (!this.closing || this.starting || pending !== 0) return;
        if (liveSession === this) liveSession = null;
        this.acknowledge();
    }
    /** Remove only Pax's region. Idempotent and never waits on the host thread. */
    close() {
        if (this.closing) return;
        this.closing = true;
        this.options.signal?.removeEventListener("abort", this.abort);
        if (liveSession === this) disposeCurrentChassis();
        this.region.remove();
        this.cleanupWake(this.pump?.() ?? 0);
    }
}

/** Explicit browser entry. Resolves after mounting, or rejects after starting
 * cleanup. Importing this library or a cartridge never mounts an application.
 * The caller owns the container, page layout and browser event loop.
 */
export async function init_web(options: WebInitOptions): Promise<MountedApp> {
    if (liveSession !== null) throw new Error("Pax supports one live mount; await closed before mounting another");
    if (!(options.element instanceof HTMLElement) || options.element.ownerDocument !== document) {
        throw new Error("Pax requires an HTMLElement in the current document");
    }
    if (options.startupData !== undefined && (!(options.startupData instanceof Uint8Array) || options.startupData.byteLength > 65536)) {
        throw new Error("startupData must be Uint8Array of at most 64 KiB");
    }
    if (options.entry !== undefined && typeof options.entry !== "string") {
        throw new Error("entry must name a declared runnable root");
    }
    // Snapshot caller-owned bytes before module/network awaits can interleave
    // with application mutations to the original buffer.
    const session = new MountSession({...options, startupData: options.startupData?.slice()});
    liveSession = session;
    try {
        session.checkOpen();
        ensureInterfaceStylesheet(options.cartridgeUrl);
        await startRenderLoop(options.cartridgeUrl, session.region, session);
        session.checkOpen();
        return session;
    } catch (error) {
        session.close();
        throw hostError(error);
    } finally {
        session.finishStartup();
    }
}

function hostError(error: unknown): Error {
    return error instanceof Error ? error : new Error(String(error));
}

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

/** Compatibility spelling; new hosts should call init_web with explicit inputs. */
export function mount(selector_or_element: string | HTMLElement, extensionlessUrl: string): Promise<MountedApp> {
    const element = typeof selector_or_element === "string" ? document.querySelector(selector_or_element) : selector_or_element;
    return init_web({element: element as HTMLElement, cartridgeUrl: extensionlessUrl, pageScroll: true, documentRouting: true});
}

function ensureInterfaceStylesheet(extensionlessUrl: string) {
    const href = new URL("pax-interface-web.css", extensionlessUrl).href;
    let alreadyLoaded = Array.from(document.querySelectorAll('link[rel="stylesheet"]'))
        .some((link) => (link as HTMLLinkElement).href === href);
    if (alreadyLoaded) {
        return;
    }
    let link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = href;
    document.head.appendChild(link);
}

async function loadWasmModule(extensionlessUrl: string, session: MountSession): Promise<void> {
    const glueCodeUrl = new URL(`${extensionlessUrl}.js`, document.baseURI);
    const wasmUrl = new URL(`${extensionlessUrl}_bg.wasm`, document.baseURI);
    const glue = await import(glueCodeUrl.href) as typeof import("./types/pax-cartridge");
    session.checkOpen();
    const response = await fetch(wasmUrl.href, {signal: session.options.signal});
    if (!response.ok) throw new Error(`Failed to fetch WASM binary: HTTP ${response.status}`);
    await glue.default({module_or_path: await response.arrayBuffer()});
    session.checkOpen();
    session.installPump(() => glue.pax_pump_shutdowns());
    let queued = false;
    glue.pax_set_shutdown_waker(() => {
        if (queued) return;
        queued = true;
        queueMicrotask(() => {
            queued = false;
            session.cleanupWake(glue.pax_pump_shutdowns());
        });
    });
    // The explicit Wasm entry invokes this host capability only after its typed
    // initializer succeeds. JS owns the chassis from that call onward, including
    // partial attachment failures; no GC finalizer is relied on for close.
    await glue.init_web({entry: session.options.entry, startupData: session.options.startupData, mount: (chassis: PaxChassisWeb) => {
        try { session.checkOpen(); } catch (error) { chassis.free(); throw error; }
        resetHostState();
        currentChassis = chassis;
        currentMount = session.region;
        currentExtensionlessUrl = extensionlessUrl;
        window.chassis = chassis;
        attachChassis(chassis, session.region);
        return session;
    }});
}

function resetHostState() {
    nativePool.dispose();
    objectManager = new ObjectManager(SUPPORTED_OBJECTS);
    nativePool = new NativeElementPool(objectManager);
    initializedChassis = false;
    frameInProgress = false;
    pendingAsyncInterruptFlush = false;
}

function disposeCurrentChassis() {
    const chassis = currentChassis;
    // Revoke routing before running disposers, which can trigger host callbacks.
    currentChassis = null;
    currentMount = null;
    delete window.chassis;
    const disposers = [() => frameScheduler.clear(), teardownHiddenTabPump,
        teardownRouteLocationSync, teardownResizeHandler, teardownEventListeners,
        () => nativePool.dispose(), () => chassis?.free()];
    teardownHiddenTabPump = teardownRouteLocationSync = teardownResizeHandler = teardownEventListeners = null;
    renderLoopStarted = renderLoopStarting = initializedChassis = frameInProgress = pendingAsyncInterruptFlush = false;
    for (const dispose of disposers) {
        try { dispose?.(); } catch (error) { console.error("Pax host disposal failed", error); }
    }
}

async function startRenderLoop(extensionlessUrl: string, mount: Element, session: MountSession) {
    renderLoopStarting = true;
    await loadWasmModule(extensionlessUrl, session);
    session.checkOpen();
    renderLoopStarted = true;
    renderLoopStarting = false;
    const chassis = currentChassis!;
    runFrame(chassis, mount);
    frameScheduler.schedule(() => renderLoop(chassis, mount));
}

function attachChassis(chassis: PaxChassisWeb, mount: Element) {
    let propertyWakeQueued = false;
    chassis.set_property_waker(() => {
        if (propertyWakeQueued) return;
        propertyWakeQueued = true;
        queueMicrotask(() => {
            propertyWakeQueued = false;
            if (chassis !== currentChassis || mount !== currentMount || !chassis.has_pending_properties()) return;
            requestFrameFlush(chassis, mount);
        });
    });
    nativePool.attach(chassis, mount, liveSession?.options.pageScroll ?? false);
    nativePool.setPostAsyncInterruptFlush(() => requestFrameFlush(chassis, mount));
    initializeChassis(chassis, mount);
    ensureHiddenTabPump(chassis, mount);
    chassis.activate_application();
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
    let syncRouteLocation = () => {
        let url = browserRouteLocation();
        if (liveSession?.options.documentRouting) replaceCurrentRouteHistoryState(url);
        chassis.interrupt({
            "RouteChange": serializeRouteLocation(url),
        }, []);
        if (liveSession?.options.documentRouting) void updateDocumentRouteMetadata(url);
    };
    window.addEventListener("popstate", syncRouteLocation);
    window.addEventListener("hashchange", syncRouteLocation);
    teardownRouteLocationSync = () => {
        window.removeEventListener("popstate", syncRouteLocation);
        window.removeEventListener("hashchange", syncRouteLocation);
    };
    syncRouteLocation();
    let lastViewportWidth = -1;
    let lastViewportHeight = -1;
    let measureViewport = () => {
        let mountElement = mount as HTMLElement;
        let rect = mountElement.getBoundingClientRect();
        let width = rect.width;
        let height = rect.height;
        return { width, height };
    };
    let resizeHandler = () => {
        // Use the layout viewport as the authoritative app size. Do not relayout the entire scene
        // during iOS Safari toolbar collapse: delegated page scroll should reveal more of the page
        // through the browser-owned visual viewport, not by continuously changing engine layout.
        let { width, height } = measureViewport();
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
        chassis.interrupt({
            "ViewportResize": {
                "width": width,
                "height": height,
            },
        }, undefined);
    };
    window.addEventListener('resize', resizeHandler);
    let resizeObserver = typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(() => {
            resizeHandler();
        });
    resizeObserver?.observe(mount);
    teardownResizeHandler = () => {
        window.removeEventListener('resize', resizeHandler);
        resizeObserver?.disconnect();
    };
    // Initialize viewport-dependent layout before the first engine tick so native/scroller hosts do
    // not bootstrap against a transient 0x0 viewport.
    resizeHandler();
    teardownEventListeners = setupEventListeners(chassis, mount as HTMLElement);
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
    const wakeHiddenFrame = () => {
        if (!document.hidden) {
            return;
        }
        requestFrameFlush(chassis, mount);
    };

    document.addEventListener("visibilitychange", pumpHiddenFrame);
    window.addEventListener("pax-designtime-wakeup", wakeHiddenFrame);
    hiddenTabPumpHandle = window.setInterval(pumpHiddenFrame, HIDDEN_TAB_FRAME_FALLBACK_MS);
    teardownHiddenTabPump = () => {
        document.removeEventListener("visibilitychange", pumpHiddenFrame);
        window.removeEventListener("pax-designtime-wakeup", wakeHiddenFrame);
        if (hiddenTabPumpHandle !== null) {
            clearInterval(hiddenTabPumpHandle);
            hiddenTabPumpHandle = null;
        }
    };
}

function requestFrameFlush(chassis: PaxChassisWeb, mount: Element) {
    if (chassis !== currentChassis || mount !== currentMount) {
        return;
    }
    if (frameInProgress) {
        pendingAsyncInterruptFlush = true;
        return;
    }
    runFrame(chassis, mount);
}

function runFrame(chassis: PaxChassisWeb, mount: Element) {
    if (frameScheduler.suspended || frameInProgress || chassis !== currentChassis || mount !== currentMount) {
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
        if (pendingAsyncInterruptFlush && chassis === currentChassis && mount === currentMount) {
            pendingAsyncInterruptFlush = false;
            queueMicrotask(() => requestFrameFlush(chassis, mount));
        }
    }
}

function renderLoop (chassis: PaxChassisWeb, mount: Element) {
    if (chassis !== currentChassis || mount !== currentMount) {
        return;
    }
    initializeChassis(chassis, mount);
    runFrame(chassis, mount);

    if (chassis !== currentChassis || mount !== currentMount) {
        return;
    }
    frameScheduler.schedule(() => renderLoop(chassis, mount));
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
        } else if(unwrapped_msg["PhotoPickerCreate"]) {
            let msg = unwrapped_msg["PhotoPickerCreate"]
            let patch: AnyCreatePatch = objectManager.getFromPool(ANY_CREATE_PATCH);
            patch.fromPatch(msg);
            nativePool.photoPickerCreate(patch);
        } else if (unwrapped_msg["PhotoPickerUpdate"]){
            let msg = unwrapped_msg["PhotoPickerUpdate"]
            let patch: PhotoPickerUpdatePatch = objectManager.getFromPool(PHOTO_PICKER_UPDATE_PATCH);
            patch.fromPatch(msg);
            nativePool.photoPickerUpdate(patch);
        }else if (unwrapped_msg["PhotoPickerDelete"]) {
            let msg = unwrapped_msg["PhotoPickerDelete"];
            nativePool.photoPickerDelete(msg)
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
            let imagePool = nativePool;
            queueMicrotask(() => {
                void imagePool.imageLoad(patch, chassis).catch((error) => {
                    console.warn(`Failed to load Pax image "${patch.path ?? ""}"`, error);
                });
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
            if (liveSession?.options.documentRouting) nativePool.navigate(patch);
            else currentMount?.dispatchEvent(new CustomEvent("pax-navigate", {detail: {url: patch.url, target: patch.target}, bubbles: true}));
        } else if (unwrapped_msg["SetCursor"]) {
            let msg = unwrapped_msg["SetCursor"];
            let patch : SetCursorPatch = objectManager.getFromPool(SET_CURSOR_PATCH);
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
