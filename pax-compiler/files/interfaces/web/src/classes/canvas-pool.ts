import { CANVAS } from "../pools/supported-objects";
import { ObjectManager } from "../pools/object-manager";

const DEFAULT_POOL_HOST_ROLE = "canvas-pool-host";

function tileDebugEnabled() {
    return typeof window !== "undefined"
        && new URLSearchParams(window.location.search).has("pax_tile_debug");
}

export class CanvasPool {
    private free: HTMLCanvasElement[] = [];
    private allocated = 0;
    private maxCanvases: number;
    private objectManager: ObjectManager;
    private host?: HTMLDivElement;

    constructor(objectManager: ObjectManager, maxCanvases: number, private onAvailable: () => void = () => {}) {
        this.objectManager = objectManager;
        this.maxCanvases = Math.max(1, Math.floor(maxCanvases));
    }

    attach(mount: Element) {
        if (this.host) {
            return;
        }
        let host = document.createElement("div");
        host.dataset.role = DEFAULT_POOL_HOST_ROLE;
        host.style.position = "absolute";
        host.style.top = "0";
        host.style.left = "0";
        host.style.width = "1px";
        host.style.height = "1px";
        host.style.overflow = "hidden";
        host.style.pointerEvents = "none";
        host.style.visibility = "hidden";
        host.addEventListener("pax-surface-released", event => {
            const canvas = event.target as HTMLCanvasElement;
            if (this.free.includes(canvas) && !canvasSurfaceIsOwned(canvas)) {
                releaseCanvasBacking(canvas);
                // The event can originate inside a Rust destructor. Re-enter the
                // chassis only after its borrow/initialization stack has unwound.
                queueMicrotask(() => this.onAvailable());
            }
        });
        mount.appendChild(host);
        this.host = host;
    }

    private debugLog(event: string, data: Record<string, unknown>) {
        if (tileDebugEnabled()) {
            console.debug(`[pax-canvas-pool] ${event}`, data);
        }
    }

    checkout(): HTMLCanvasElement | null {
        // A released element is not reusable until all async initializers and
        // renderers have dropped their ownership, even at pool capacity.
        const eligibleIndex = this.free.findIndex(canvas => !canvasSurfaceIsOwned(canvas));
        if (eligibleIndex >= 0) {
            return this.free.splice(eligibleIndex, 1)[0];
        }
        if (this.allocated >= this.maxCanvases) {
            this.debugLog("checkout-miss", {
                allocated: this.allocated, maxCanvases: this.maxCanvases, free: this.free.length,
            });
            return null;
        }
        this.allocated += 1;
        this.debugLog("checkout-new", {
            allocated: this.allocated,
            maxCanvases: this.maxCanvases,
            free: this.free.length,
        });
        return this.objectManager.getFromPool(CANVAS);
    }

    release(canvas: HTMLCanvasElement) {
        if (this.host) {
            if (canvas.parentElement !== this.host) {
                canvas.parentElement?.removeChild(canvas);
                this.host.appendChild(canvas);
            }
        } else {
            canvas.parentElement?.removeChild(canvas);
        }
        releaseCanvasBacking(canvas);
        this.free.push(canvas);
        this.debugLog("release", {
            allocated: this.allocated,
            maxCanvases: this.maxCanvases,
            free: this.free.length,
        });
    }
}

// Shared with the WebGPU backend's BrowserCanvasLease. Piet has no async owner.
export function canvasSurfaceIsOwned(canvas: HTMLCanvasElement): boolean {
    return Number(canvas.getAttribute("data-pax-surface-owners") ?? "0") > 0;
}

export function releaseCanvasBacking(canvas: HTMLCanvasElement) {
    if (!canvasSurfaceIsOwned(canvas)) {
        // Dropping wgpu's web surface leaves the browser context configured.
        // Retire it before this element can display another row; resizing alone
        // can leave the previous presentation visible during an animated entry.
        // Only leased WebGPU canvases have this marker. Do not acquire a GPU
        // context on an unused canvas or a canvas belonging to the Piet fallback.
        if (canvas.hasAttribute("data-pax-surface-owners")) {
            const context = canvas.getContext("webgpu");
            if (context && "unconfigure" in context && typeof context.unconfigure === "function") {
                context.unconfigure();
            }
        }
        canvas.width = 1;
        canvas.height = 1;
    }
}
