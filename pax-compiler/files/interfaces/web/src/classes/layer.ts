import { NATIVE_OVERLAY_CLASS } from "../utils/constants";
import { ObjectManager } from "../pools/object-manager";
import { CANVAS, DIV } from "../pools/supported-objects";
import { CanvasPool, releaseCanvasBacking } from "./canvas-pool";
import { canvasPlanSignature } from "./surface-host-policy";
import type { LayerCanvasPlan, SurfaceCanvasDescriptor } from "./surface-host-policy";

let nextSurfaceGeneration = 0;
let nextSurfaceContentRevision = 0;

export class Layer {
    canvasMap?: Map<string, HTMLCanvasElement>;
    native?: HTMLDivElement;
    renderLayerId?: number;
    objectManager: ObjectManager;
    private visibleCanvasParent?: Element;
    private canvasOwner?: number | null;
    private canvases: Map<string, HTMLCanvasElement>;
    private canvasZIndex: string;
    private islandOnly: boolean;
    private canvasPlan?: LayerCanvasPlan;
    private lastPlanSignature?: string;
    private lastVisibleHost?: Element;
    private canvasPool?: CanvasPool;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
        this.canvases = new Map();
        this.canvasZIndex = "0";
        this.islandOnly = false;
    }

    build(
        parent: Element,
        renderLayerId: number,
        canvasMap: Map<string, HTMLCanvasElement>,
        canvasPool?: CanvasPool,
    ) {
        this.buildCanvases(renderLayerId, canvasMap, canvasPool);
        this.native = this.objectManager.getFromPool(DIV);
        this.native.className = NATIVE_OVERLAY_CLASS;
        this.native.style.zIndex = String(renderLayerId * 2 + 1);
        this.attachToParents(parent, parent);
    }

    buildCanvases(renderLayerId: number, canvasMap: Map<string, HTMLCanvasElement>, canvasPool?: CanvasPool) {
        this.renderLayerId = renderLayerId;
        this.canvasMap = canvasMap;
        this.canvasPool = canvasPool;
        // Non-root layers are currently reserved for browser-owned scroller islands. Avoid
        // creating root canvases for them until they are claimed by a scroller host.
        this.islandOnly = renderLayerId > 0;
        this.canvasZIndex = String(renderLayerId * 2);
    }

    setCanvasParent(parent?: Element, owner?: number | null): boolean {
        // A pooled host div can survive a change of logical scroller.
        const retiring = this.canvases.size > 0 && (this.visibleCanvasParent !== parent || this.canvasOwner !== owner);
        this.canvasOwner = owner;
        if (retiring) this.detachCanvases(true);
        this.visibleCanvasParent = parent;
        return retiring;
    }

    setCanvasZIndex(zIndex: string) {
        this.canvasZIndex = zIndex;
        this.canvases.forEach((canvas) => {
            canvas.style.zIndex = zIndex;
        });
    }

    attachToParents(canvasParent: Element, nativeParent: Element) {
        this.setCanvasParent(canvasParent);
        if (this.native != undefined && this.native.parentElement !== nativeParent) {
            nativeParent.appendChild(this.native);
        }
        this.syncCanvasLayout();
    }

    setIslandOnly(islandOnly: boolean) {
        this.islandOnly = islandOnly;
    }

    isIslandOnly() {
        return this.islandOnly;
    }

    detachCanvases(keepHost: boolean = false) {
        this.canvases.forEach((canvas, id) => {
            this.canvasMap?.delete(id);
            this.prepareCanvasForRelease(canvas);
            if (this.canvasPool) {
                this.canvasPool.release(canvas);
            } else {
                canvas.parentElement?.removeChild(canvas);
                this.objectManager.returnToPool(CANVAS, canvas);
            }
        });
        this.canvases.clear();
        if (!keepHost) {
            this.visibleCanvasParent = undefined;
        }
        this.lastPlanSignature = undefined;
        this.lastVisibleHost = undefined;
    }

    setCanvasPlan(plan?: LayerCanvasPlan) {
        this.canvasPlan = plan;
    }

    canvasLayoutPriority() {
        return this.visibleCanvasParent instanceof HTMLElement
            && this.visibleCanvasParent.dataset.viewportVisible === "false" ? 1 : 0;
    }

    retireUnusedCanvases(): boolean {
        if (this.canvasPlan == null) {
            return false;
        }
        const expectedIds = new Set(this.canvasPlan.surfaces.map(surface => surface.id));
        let retired = false;
        this.canvases.forEach((canvas, id) => {
            if (expectedIds.has(id)) {
                return;
            }
            this.canvasMap?.delete(id);
            this.prepareCanvasForRelease(canvas);
            if (this.canvasPool) {
                this.canvasPool.release(canvas);
            } else {
                canvas.parentElement?.removeChild(canvas);
                this.objectManager.returnToPool(CANVAS, canvas);
            }
            this.canvases.delete(id);
            retired = true;
        });
        if (retired) {
            this.lastPlanSignature = undefined;
            this.lastVisibleHost = undefined;
        }
        return retired;
    }

    syncCanvasLayout(enqueue?: (priority: number, install: () => void) => void) {
        if (this.renderLayerId == null || this.canvasMap == null) {
            return;
        }
        if (this.canvasPlan == null) {
            return;
        }

        let visibleHost =
            this.visibleCanvasParent instanceof HTMLElement ? this.visibleCanvasParent : undefined;
        if (
            this.islandOnly
            && (visibleHost == null || visibleHost.dataset.role !== "scroller-canvas-host")
        ) {
            this.detachCanvases();
            return;
        }
        let plan = this.canvasPlan;
        if (plan.surfaces.length === 0) {
            this.detachCanvases(true);
            return;
        }
        let planSignature = canvasPlanSignature(plan);
        if (planSignature === this.lastPlanSignature && visibleHost === this.lastVisibleHost) {
            return;
        }
        this.retireUnusedCanvases();
        let missingSurface = false;
        let remaining = plan.surfaces.length;
        const finishSurface = () => {
            if (--remaining > 0) return;
            this.lastPlanSignature = missingSurface ? undefined : planSignature;
            this.lastVisibleHost = missingSurface ? undefined : visibleHost;
        };
        const schedule = enqueue ?? ((_priority, install) => install());

        [...plan.surfaces].sort((a, b) => a.replayPriority - b.replayPriority).forEach((descriptor) => {
            schedule(descriptor.replayPriority, () => {
                let canvas = this.canvases.get(descriptor.id);
                if (canvas == null) {
                    if (this.canvasPool) {
                        const available = this.canvasPool.checkout((descriptor.backingWidth ?? 0) * (descriptor.backingHeight ?? 0));
                        if (available == null) {
                            missingSurface = true;
                            finishSurface();
                            return;
                        }
                        canvas = available;
                    } else {
                        canvas = this.objectManager.getFromPool<HTMLCanvasElement>(CANVAS);
                    }
                    // A pooled element may return to the same logical tile after its
                    // backing store was cleared. DOM identity alone is not a lease.
                    canvas.dataset.surfaceGeneration = String(++nextSurfaceGeneration);
                    if (visibleHost?.closest("[data-pax-private-tiles]")) canvas.dataset.paxBitmapFront = "";
                    else delete canvas.dataset.paxBitmapFront;
                    canvas.style.display = canvas.dataset.paxBitmapFront === undefined ? "" : "none";
                    // Object-manager reuse can reset inline styles. Reassert the
                    // WebGPU presentation gate before publishing a new assignment,
                    // including the interval before its async lease is acquired.
                    if (canvas.hasAttribute("data-pax-surface-owners")) {
                        canvas.style.visibility = "hidden";
                    }
                    delete canvas.dataset.surfaceRetiredId;
                    delete canvas.dataset.surfaceRetiredRevision;
                    canvas.style.position = "absolute";
                    canvas.style.pointerEvents = "none";
                    canvas.style.backgroundColor = "transparent";
                    this.canvases.set(descriptor.id, canvas);
                    this.canvasMap!.set(descriptor.id, canvas);
                }
                if (this.canvasPool && !this.canvasPool.reserveBacking(canvas,
                    (descriptor.backingWidth ?? 0) * (descriptor.backingHeight ?? 0))) {
                    // A resizing assignment can need more pixels while another
                    // canvas still has a live lease. Do not publish guessed sizes.
                    this.canvases.delete(descriptor.id);
                    this.canvasMap!.delete(descriptor.id);
                    this.prepareCanvasForRelease(canvas);
                    this.canvasPool.release(canvas);
                    missingSurface = true;
                    finishSurface();
                    return;
                }
                this.configureCanvas(canvas, descriptor);
                if (visibleHost != null && canvas.parentElement !== visibleHost) {
                    visibleHost.appendChild(canvas);
                }
                finishSurface();
            });
        });
    }

    public cleanUp() {
        this.canvases.forEach((canvas, id) => {
            this.canvasMap?.delete(id);
            this.prepareCanvasForRelease(canvas);
            if (this.canvasPool) {
                this.canvasPool.release(canvas);
            } else {
                canvas.parentElement?.removeChild(canvas);
                this.objectManager.returnToPool(CANVAS, canvas);
            }
        });
        this.canvases.clear();
        this.islandOnly = false;
        this.canvasPlan = undefined;
        this.canvasOwner = undefined;
        this.visibleCanvasParent = undefined;
        this.lastPlanSignature = undefined;
        this.lastVisibleHost = undefined;

        if (this.native != undefined) {
            let parent = this.native.parentElement;
            parent?.removeChild(this.native);
            this.objectManager.returnToPool(DIV, this.native);
        }
        this.renderLayerId = undefined;
    }

    private configureCanvas(
        canvas: HTMLCanvasElement,
        descriptor: SurfaceCanvasDescriptor,
    ) {
        const left = String(descriptor.left), top = String(descriptor.top);
        const width = String(descriptor.width), height = String(descriptor.height);
        if (canvas.dataset.surfaceSignature !== descriptor.surfaceSignature
            || canvas.dataset.transformSignature !== descriptor.transformSignature
            || canvas.dataset.hostSignature !== descriptor.hostSignature
            || canvas.dataset.tileOriginX !== left || canvas.dataset.tileOriginY !== top
            || canvas.dataset.logicalWidth !== width || canvas.dataset.logicalHeight !== height) {
            canvas.dataset.surfaceContentRevision = String(++nextSurfaceContentRevision);
            canvas.dataset.surfacePreparedAt = String(performance.now());
            delete canvas.dataset.surfacePreparationLatency;
            delete canvas.dataset.surfaceFirstVisibleAt;
            delete canvas.dataset.surfaceSubmittedAt;
        }
        canvas.id = descriptor.id;
        canvas.dataset.layerId = String(this.renderLayerId);
        canvas.dataset.tileKey = descriptor.key;
        canvas.dataset.tileOriginX = left;
        canvas.dataset.tileOriginY = top;
        canvas.dataset.logicalWidth = width;
        canvas.dataset.logicalHeight = height;
        if (descriptor.backingWidth != null && descriptor.backingHeight != null) {
            canvas.dataset.expectedBackingWidth = String(descriptor.backingWidth);
            canvas.dataset.expectedBackingHeight = String(descriptor.backingHeight);
        }
        canvas.dataset.densityClamped = String(!!descriptor.densityClamped);
        canvas.dataset.replayPriority = String(descriptor.replayPriority);
        canvas.dataset.surfaceSignature = descriptor.surfaceSignature;
        canvas.dataset.transformSignature = descriptor.transformSignature;
        if (descriptor.hostSignature) {
            canvas.dataset.hostSignature = descriptor.hostSignature;
        }
        // The presenter installs a private tile’s display rectangle with its bitmap.
        if (canvas.dataset.paxBitmapFront === undefined) {
            canvas.style.top = `${descriptor.top}px`;
            canvas.style.left = `${descriptor.left}px`;
            canvas.style.width = `${descriptor.width}px`;
            canvas.style.height = `${descriptor.height}px`;
        }
        // Layer ownership can move between the root surface stack and a nested scroller island.
        // Persist the chosen local z-order so later tile/layout syncs do not silently lift canvases
        // back above the native overlay inside the island host.
        canvas.style.zIndex = this.canvasZIndex;
    }

    private prepareCanvasForRelease(canvas: HTMLCanvasElement) {
        if (canvas.dataset.paxBitmapFront !== undefined) {
            canvas.dispatchEvent(new Event('pax-tile-retirement'));
        }
        if (canvas.dataset.surfaceTrace === '1') {
            canvas.dataset.surfaceRetiredId = canvas.id;
            canvas.dataset.surfaceRetiredRevision = canvas.dataset.surfaceContentRevision;
            canvas.dataset.surfaceEvent = 'retirement';
            canvas.dispatchEvent(new Event('pax-surface-trace', {bubbles: true}));
        }
        // Do not clear a canvas still held by an asynchronous GPU initializer.
        // The pool shrinks it once the final surface owner releases it.
        releaseCanvasBacking(canvas);
        canvas.style.width = "1px";
        canvas.style.height = "1px";
        canvas.id = "";
        canvas.removeAttribute("data-layer-id");
        canvas.removeAttribute("data-tile-key");
        canvas.removeAttribute("data-tile-origin-x");
        canvas.removeAttribute("data-tile-origin-y");
        canvas.removeAttribute("data-logical-width");
        canvas.removeAttribute("data-logical-height");
        canvas.removeAttribute("data-expected-backing-width");
        canvas.removeAttribute("data-expected-backing-height");
        canvas.removeAttribute("data-replay-priority");
        canvas.removeAttribute("data-surface-signature");
        canvas.removeAttribute("data-transform-signature");
        canvas.removeAttribute("data-host-signature");
        canvas.removeAttribute("data-surface-content-revision");
        canvas.removeAttribute("data-surface-prepared-at");
        canvas.removeAttribute("data-surface-preparation-latency");
        canvas.removeAttribute("data-surface-submitted-revision");
        canvas.removeAttribute("data-surface-configured-generation");
        canvas.removeAttribute("data-surface-first-visible-at");
        canvas.removeAttribute("data-surface-submitted-at");
        canvas.removeAttribute("data-surface-init-started-at");
        canvas.removeAttribute("data-surface-init-completed-at");
        canvas.removeAttribute("data-surface-queued-revision");
    }
}
