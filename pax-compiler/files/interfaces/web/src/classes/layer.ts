import { NATIVE_OVERLAY_CLASS } from "../utils/constants";
import { ObjectManager } from "../pools/object-manager";
import { CANVAS, DIV } from "../pools/supported-objects";
import type { CanvasPool } from "./canvas-pool";
import type { LayerCanvasPlan, SurfaceCanvasDescriptor } from "./surface-host-policy";

export class Layer {
    canvasMap?: Map<string, HTMLCanvasElement>;
    native?: HTMLDivElement;
    occlusionLayerId?: number;
    objectManager: ObjectManager;
    private visibleCanvasParent?: Element;
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
        occlusionLayerId: number,
        canvasMap: Map<string, HTMLCanvasElement>,
        canvasPool?: CanvasPool,
    ) {
        this.occlusionLayerId = occlusionLayerId;
        this.canvasMap = canvasMap;
        this.canvasPool = canvasPool;
        // Non-root layers are currently reserved for browser-owned scroller islands. Avoid
        // creating root canvases for them until they are claimed by a scroller host.
        this.islandOnly = occlusionLayerId > 0;
        this.canvasZIndex = String(occlusionLayerId * 2);
        this.native = this.objectManager.getFromPool(DIV);
        this.native.className = NATIVE_OVERLAY_CLASS;
        this.native.style.zIndex = String(occlusionLayerId * 2 + 1);
        this.attachToParents(parent, parent);
    }

    setCanvasZIndex(zIndex: string) {
        this.canvasZIndex = zIndex;
        this.canvases.forEach((canvas) => {
            canvas.style.zIndex = zIndex;
        });
    }

    attachToParents(canvasParent: Element, nativeParent: Element) {
        this.visibleCanvasParent = canvasParent;
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

    syncCanvasLayout() {
        if (this.occlusionLayerId == null || this.canvasMap == null) {
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
        let planSignature = this.computePlanSignature(plan);
        if (planSignature === this.lastPlanSignature && visibleHost === this.lastVisibleHost) {
            return;
        }
        let expectedIds = new Set(plan.surfaces.map((descriptor) => descriptor.id));
        let missingSurface = false;

        plan.surfaces.forEach((descriptor) => {
            let canvas = this.canvases.get(descriptor.id);
            if (canvas == null) {
                if (this.canvasPool) {
                    canvas = this.canvasPool.checkout();
                    if (canvas == null) {
                        missingSurface = true;
                        return;
                    }
                } else {
                    canvas = this.objectManager.getFromPool(CANVAS);
                }
                canvas.style.position = "absolute";
                canvas.style.pointerEvents = "none";
                canvas.style.backgroundColor = "transparent";
                this.canvases.set(descriptor.id, canvas);
                this.canvasMap!.set(descriptor.id, canvas);
            }
            this.configureCanvas(canvas, descriptor);
            if (visibleHost != null && canvas.parentElement !== visibleHost) {
                visibleHost.appendChild(canvas);
            }
        });

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
        });

        if (missingSurface) {
            this.lastPlanSignature = undefined;
            this.lastVisibleHost = undefined;
        } else {
            this.lastPlanSignature = planSignature;
            this.lastVisibleHost = visibleHost;
        }
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
        this.lastPlanSignature = undefined;
        this.lastVisibleHost = undefined;

        if (this.native != undefined) {
            let parent = this.native.parentElement;
            parent?.removeChild(this.native);
            this.objectManager.returnToPool(DIV, this.native);
        }
        this.occlusionLayerId = undefined;
    }

    private configureCanvas(canvas: HTMLCanvasElement, descriptor: SurfaceCanvasDescriptor) {
        canvas.id = descriptor.id;
        canvas.dataset.layerId = String(this.occlusionLayerId);
        canvas.dataset.tileKey = descriptor.key;
        canvas.dataset.tileOriginX = String(descriptor.left);
        canvas.dataset.tileOriginY = String(descriptor.top);
        canvas.dataset.logicalWidth = String(descriptor.width);
        canvas.dataset.logicalHeight = String(descriptor.height);
        canvas.dataset.replayPriority = String(descriptor.replayPriority);
        canvas.dataset.surfaceSignature = descriptor.surfaceSignature;
        canvas.dataset.transformSignature = descriptor.transformSignature;
        if (descriptor.hostSignature) {
            canvas.dataset.hostSignature = descriptor.hostSignature;
        }
        canvas.style.top = `${descriptor.top}px`;
        canvas.style.left = `${descriptor.left}px`;
        canvas.style.width = `${descriptor.width}px`;
        canvas.style.height = `${descriptor.height}px`;
        // Layer ownership can move between the root surface stack and a nested scroller island.
        // Persist the chosen local z-order so later tile/layout syncs do not silently lift canvases
        // back above the native overlay inside the island host.
        canvas.style.zIndex = this.canvasZIndex;
    }

    private prepareCanvasForRelease(canvas: HTMLCanvasElement) {
        // WebKit will more reliably release GPU resources if the backing store shrinks before
        // detaching the element from the DOM.
        canvas.width = 1;
        canvas.height = 1;
        canvas.style.width = "1px";
        canvas.style.height = "1px";
        canvas.id = "";
        canvas.removeAttribute("data-layer-id");
        canvas.removeAttribute("data-tile-key");
        canvas.removeAttribute("data-tile-origin-x");
        canvas.removeAttribute("data-tile-origin-y");
        canvas.removeAttribute("data-logical-width");
        canvas.removeAttribute("data-logical-height");
        canvas.removeAttribute("data-replay-priority");
        canvas.removeAttribute("data-surface-signature");
        canvas.removeAttribute("data-transform-signature");
        canvas.removeAttribute("data-host-signature");
    }

    private computePlanSignature(plan: LayerCanvasPlan): string {
        let parts = [plan.layerId.toString(), plan.active ? "1" : "0", plan.surfaces.length.toString()];
        plan.surfaces.forEach((descriptor) => {
            parts.push(
                `${descriptor.id}|${descriptor.left},${descriptor.top},${descriptor.width},${descriptor.height}`
                + `|${descriptor.surfaceSignature}|${descriptor.transformSignature}|${descriptor.hostSignature ?? ""}`,
            );
        });
        return parts.join(";");
    }
}
