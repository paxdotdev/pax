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
import { replaceCurrentRouteHistoryState, serializeRouteLocation } from "./utils/route-location";
import { updateDocumentRouteMetadata } from "./utils/route-metadata";

import { FrameScheduler } from "./utils/frame-scheduler";

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
let pendingReloadRequest: PrepareAppRevision | null = null;
let reloadInProgress = false;
let reloadInFlightBuildId: string | null = null;
let reloadRetryBuildId: string | null = null;
let reloadRetryAttempt = 0;
let reloadRetryDelayMs = 500;
let reloadRetryHandle: number | null = null;
const RELOAD_RETRY_MAX_DELAY_MS = 5_000;
const ACTIVATION_PREFLIGHT_TIMEOUT_MS = 15_000;
const ACTIVATION_POLL_INTERVAL_MS = 25;
const perfTraceEnabled = typeof window !== "undefined" && new URLSearchParams(window.location.search).has("pax_scroll_perf");
let perfTraceSequence = 0;

type PrepareAppRevision = {
    logic_revision_id: string;
    execution_mode: "compiled-artifact" | "interpreted-module";
    artifact: {
        kind: string;
        location: string;
    };
};

type AppRevisionActivationStatus =
    | "unknown"
    | "preparing"
    | "prepared"
    | "committing"
    | "committed"
    | "rejected";

class ReloadActivationError extends Error {
    constructor(message: string, readonly retryable: boolean) {
        super(message);
    }
}

function waitForActivationPoll(): Promise<void> {
    return new Promise((resolve) => window.setTimeout(resolve, ACTIVATION_POLL_INTERVAL_MS));
}

function hasSupersedingReload(logicRevisionId: string): boolean {
    return pendingReloadRequest != null
        && pendingReloadRequest.logic_revision_id !== logicRevisionId;
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

export function mount(selector_or_element: string | Element, extensionlessUrl: string) {
    if (renderLoopStarting || renderLoopStarted) {
        console.warn("Pax render loop already started; ignoring duplicate mount() call.");
        return;
    }

    ensureInterfaceStylesheet(extensionlessUrl);

    let mount: Element;
    if (typeof selector_or_element === "string") {
        mount = document.querySelector(selector_or_element) as Element;
    } else {
        mount = selector_or_element;
    }

    // Update to pass wasmUrl to bootstrap function
    if (mount) {
        currentMount = mount;
        currentExtensionlessUrl = extensionlessUrl;
        startRenderLoop(extensionlessUrl, mount).then();
    } else {
        console.error("Unable to find mount element");
    }
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

async function loadWasmModule(
    extensionlessUrl: string,
    reloadCacheKey?: string,
): Promise<{ chassis: PaxChassisWeb }> {
    try {
        const glueCodeUrl = new URL(`${extensionlessUrl}.js`, document.baseURI);
        const wasmUrl = new URL(`${extensionlessUrl}_bg.wasm`, document.baseURI);
        if (reloadCacheKey != null) {
            glueCodeUrl.searchParams.set("pax-reload", reloadCacheKey);
            wasmUrl.searchParams.set("pax-reload", reloadCacheKey);
        }
        const glueCodeModule = await import(glueCodeUrl.href) as typeof import("./types/pax-cartridge");

        const wasmBinary = await fetch(wasmUrl.href, reloadCacheKey == null ? undefined : {cache: "no-store"});
        if (!wasmBinary.ok) {
            throw new Error(`Failed to fetch WASM binary: HTTP ${wasmBinary.status}`);
        }
        const wasmArrayBuffer = await wasmBinary.arrayBuffer();
        await glueCodeModule.default({module_or_path: wasmArrayBuffer});

        let chassis = await glueCodeModule.pax_init();
        window.chassis = chassis;

        return { chassis };
    } catch (err) {
        throw new Error(`Failed to load WASM module: ${err}`);
    }
}

function resetHostState() {
    objectManager = new ObjectManager(SUPPORTED_OBJECTS);
    nativePool = new NativeElementPool(objectManager);
    initializedChassis = false;
    frameInProgress = false;
    pendingAsyncInterruptFlush = false;
}

function disposeCurrentChassis() {
    frameScheduler.clear();
    teardownHiddenTabPump?.();
    teardownHiddenTabPump = null;
    teardownRouteLocationSync?.();
    teardownRouteLocationSync = null;
    teardownResizeHandler?.();
    teardownResizeHandler = null;
    teardownEventListeners?.();
    teardownEventListeners = null;
    nativePool.dispose();
    currentChassis?.free();
    currentChassis = null;
    renderLoopStarted = false;
    renderLoopStarting = false;
    initializedChassis = false;
    frameInProgress = false;
    pendingAsyncInterruptFlush = false;
}

async function startRenderLoop(extensionlessUrl: string, mount: Element) {
    if (renderLoopStarting || renderLoopStarted) {
        return;
    }
    renderLoopStarting = true;
    try {
        let {chassis} = await loadWasmModule(extensionlessUrl);
        resetHostState();
        currentChassis = chassis;
        currentMount = mount;
        currentExtensionlessUrl = extensionlessUrl;
        attachChassis(chassis, mount);
        renderLoopStarted = true;
        renderLoopStarting = false;
        frameScheduler.schedule(() => renderLoop(chassis, mount));
    } catch (error) {
        renderLoopStarting = false;
        console.error("Failed to load or instantiate Wasm module:", error);
    }
}

function attachChassis(chassis: PaxChassisWeb, mount: Element) {
    nativePool.attach(chassis, mount);
    nativePool.setPostAsyncInterruptFlush(() => requestFrameFlush(chassis, mount));
    initializeChassis(chassis, mount);
    ensureHiddenTabPump(chassis, mount);
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
        let url = new URL(window.location.href);
        replaceCurrentRouteHistoryState(url);
        chassis.interrupt({
            "RouteChange": serializeRouteLocation(url),
        }, []);
        void updateDocumentRouteMetadata(url);
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
        let root = document.documentElement;
        let mountElement = mount as HTMLElement;
        let rect = mountElement.getBoundingClientRect();
        let width = rect.width || mountElement.clientWidth || root.clientWidth || window.innerWidth || 0;
        let height = rect.height || mountElement.clientHeight || root.clientHeight || window.innerHeight || 0;
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
    teardownEventListeners = setupEventListeners(chassis);
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
        processReloadRequests(chassis);
    } finally {
        frameInProgress = false;
        if (pendingAsyncInterruptFlush && chassis === currentChassis && mount === currentMount) {
            pendingAsyncInterruptFlush = false;
            queueMicrotask(() => requestFrameFlush(chassis, mount));
        }
    }
}

function takePreparedRevisions(chassis: PaxChassisWeb): PrepareAppRevision[] {
    let value = (chassis as any).take_prepare_app_revisions?.();
    if (!Array.isArray(value)) {
        return [];
    }
    return value as PrepareAppRevision[];
}

function normalizedArtifactLocation(location: string): string {
    return new URL(location, document.baseURI).href;
}

function beginReloadAttempt(buildId: string): number {
    if (reloadRetryBuildId !== buildId) {
        reloadRetryBuildId = buildId;
        reloadRetryAttempt = 0;
        reloadRetryDelayMs = 500;
    }
    return reloadRetryAttempt++;
}

function clearReloadRetry(buildId: string) {
    if (reloadRetryBuildId !== buildId) {
        return;
    }
    reloadRetryBuildId = null;
    reloadRetryAttempt = 0;
    reloadRetryDelayMs = 500;
    if (reloadRetryHandle != null) {
        window.clearTimeout(reloadRetryHandle);
        reloadRetryHandle = null;
    }
}

function retryReloadAfterDelay(request: PrepareAppRevision) {
    if (pendingReloadRequest != null && pendingReloadRequest.logic_revision_id !== request.logic_revision_id) {
        clearReloadRetry(request.logic_revision_id);
        return;
    }
    pendingReloadRequest = request;
    const delay = reloadRetryDelayMs;
    reloadRetryDelayMs = Math.min(reloadRetryDelayMs * 2, RELOAD_RETRY_MAX_DELAY_MS);
    console.warn(`Retrying Pax cartridge ${request.logic_revision_id} in ${delay}ms`);
    reloadRetryHandle = window.setTimeout(() => {
        reloadRetryHandle = null;
        void reloadMountedApp();
    }, delay);
}

function processReloadRequests(chassis: PaxChassisWeb) {
    let requests = takePreparedRevisions(chassis);
    if (requests.length === 0) {
        return;
    }
    let request = requests[requests.length - 1];
    if (request.execution_mode !== "compiled-artifact" || request.artifact.kind !== "web-cartridge") {
        console.warn("Ignoring unsupported Pax logic revision", request);
        return;
    }
    if (
        pendingReloadRequest?.logic_revision_id === request.logic_revision_id
        || reloadInFlightBuildId === request.logic_revision_id
    ) {
        return;
    }
    if (
        currentExtensionlessUrl != null
        && normalizedArtifactLocation(currentExtensionlessUrl) === normalizedArtifactLocation(request.artifact.location)
    ) {
        chassis.activate_app_revision(request.logic_revision_id);
        return;
    }
    pendingReloadRequest = request;
    if (reloadRetryHandle != null) {
        window.clearTimeout(reloadRetryHandle);
        reloadRetryHandle = null;
    }
    if (!reloadInProgress) {
        queueMicrotask(() => void reloadMountedApp());
    }
}

async function reloadMountedApp() {
    if (reloadInProgress) {
        return;
    }
    reloadInProgress = true;
    try {
        while (pendingReloadRequest != null) {
            let request = pendingReloadRequest;
            pendingReloadRequest = null;
            let mount = currentMount;
            if (!mount) {
                clearReloadRetry(request.logic_revision_id);
                continue;
            }
            reloadInFlightBuildId = request.logic_revision_id;
            const attempt = beginReloadAttempt(request.logic_revision_id);
            let candidateChassis: PaxChassisWeb | null = null;
            let finalCommitStarted = false;
            try {
                let { chassis } = await loadWasmModule(
                    request.artifact.location,
                    `${request.logic_revision_id}-${attempt}`,
                );
                candidateChassis = chassis;
                // `loadWasmModule` exposes the newest allocation for debugging,
                // but the mounted chassis remains authoritative until commit.
                (window as any).chassis = currentChassis;
                if (hasSupersedingReload(request.logic_revision_id)) {
                    // A newer candidate superseded this one while its module was
                    // loading. Never replace the last known good chassis with a
                    // revision the coordinator can no longer activate.
                    (chassis as any).free?.();
                    candidateChassis = null;
                    clearReloadRetry(request.logic_revision_id);
                    continue;
                }

                const preflightDeadline = performance.now() + ACTIVATION_PREFLIGHT_TIMEOUT_MS;
                let requested = false;
                while (!requested) {
                    if (hasSupersedingReload(request.logic_revision_id)) {
                        throw new ReloadActivationError(
                            `Pax logic revision ${request.logic_revision_id} was superseded before preflight`,
                            false,
                        );
                    }
                    requested = chassis.request_app_revision_activation(request.logic_revision_id);
                    if (requested) {
                        break;
                    }
                    if (performance.now() >= preflightDeadline) {
                        throw new ReloadActivationError(
                            `Timed out preparing Pax logic revision ${request.logic_revision_id}`,
                            false,
                        );
                    }
                    await waitForActivationPoll();
                }

                while (true) {
                    const status = chassis.poll_app_revision_activation(
                        request.logic_revision_id,
                    ) as AppRevisionActivationStatus;
                    if (status === "prepared") {
                        break;
                    }
                    if (status === "rejected") {
                        throw new ReloadActivationError(
                            `Design server rejected Pax logic revision ${request.logic_revision_id}`,
                            false,
                        );
                    }
                    if (hasSupersedingReload(request.logic_revision_id)) {
                        chassis.cancel_app_revision_activation(request.logic_revision_id);
                        throw new ReloadActivationError(
                            `Pax logic revision ${request.logic_revision_id} was superseded during preflight`,
                            false,
                        );
                    }
                    if (performance.now() >= preflightDeadline) {
                        chassis.cancel_app_revision_activation(request.logic_revision_id);
                        throw new ReloadActivationError(
                            `Timed out validating Pax logic revision ${request.logic_revision_id}`,
                            false,
                        );
                    }
                    await waitForActivationPoll();
                }

                if (!chassis.activate_app_revision(request.logic_revision_id)) {
                    chassis.cancel_app_revision_activation(request.logic_revision_id);
                    throw new ReloadActivationError(
                        `Could not start final commit for Pax logic revision ${request.logic_revision_id}`,
                        false,
                    );
                }
                finalCommitStarted = true;

                // There is deliberately no timeout after this boundary. The
                // server may have committed just before a disconnect, so the
                // candidate must reconnect and replay final commit until the
                // matching authoritative manifest arrives.
                while (true) {
                    const status = chassis.poll_app_revision_activation(
                        request.logic_revision_id,
                    ) as AppRevisionActivationStatus;
                    if (status === "committed") {
                        break;
                    }
                    if (status === "rejected") {
                        throw new ReloadActivationError(
                            `Design server rejected final commit for Pax logic revision ${request.logic_revision_id}`,
                            false,
                        );
                    }
                    await waitForActivationPoll();
                }

                disposeCurrentChassis();
                resetHostState();
                currentChassis = chassis;
                candidateChassis = null;
                currentMount = mount;
                currentExtensionlessUrl = request.artifact.location;
                (window as any).chassis = chassis;
                attachChassis(chassis, mount);
                renderLoopStarted = true;
                frameScheduler.schedule(() => renderLoop(chassis, mount));
                clearReloadRetry(request.logic_revision_id);
            } catch (error) {
                console.error(`Failed to reload Pax cartridge ${request.logic_revision_id}:`, error);
                if (!finalCommitStarted) {
                    candidateChassis?.cancel_app_revision_activation(request.logic_revision_id);
                }
                candidateChassis?.free();
                (window as any).chassis = currentChassis;
                const retryable = !(error instanceof ReloadActivationError) || error.retryable;
                if (retryable && pendingReloadRequest == null) {
                    retryReloadAfterDelay(request);
                    return;
                }
                clearReloadRetry(request.logic_revision_id);
            } finally {
                if (reloadInFlightBuildId === request.logic_revision_id) {
                    reloadInFlightBuildId = null;
                }
            }
        }
    } finally {
        reloadInProgress = false;
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
            nativePool.navigate(patch)
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
