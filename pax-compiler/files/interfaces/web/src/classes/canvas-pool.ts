import { CANVAS } from "../pools/supported-objects";
import { ObjectManager } from "../pools/object-manager";

const DEFAULT_POOL_HOST_ROLE = "canvas-pool-host";

export class CanvasPool {
    private free: Array<{ canvas: HTMLCanvasElement; releasedAt: number }> = [];
    private allocated = 0;
    private maxCanvases: number;
    private objectManager: ObjectManager;
    private host?: HTMLDivElement;
    // Delay reuse of recently released canvases to avoid re-binding WebGPU surfaces too fast.
    private reuseCooldownMs: number;

    constructor(objectManager: ObjectManager, maxCanvases: number, reuseCooldownMs: number = 0) {
        this.objectManager = objectManager;
        this.maxCanvases = Math.max(1, Math.floor(maxCanvases));
        this.reuseCooldownMs = Math.max(0, reuseCooldownMs);
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
        mount.appendChild(host);
        this.host = host;
    }

    private nowMs() {
        if (typeof performance !== "undefined" && typeof performance.now === "function") {
            return performance.now();
        }
        return Date.now();
    }

    checkout(): HTMLCanvasElement | null {
        if (this.free.length > 0) {
            let now = this.nowMs();
            let eligibleIndex = this.free.findIndex(
                (entry) => now - entry.releasedAt >= this.reuseCooldownMs,
            );
            if (eligibleIndex >= 0) {
                let entry = this.free.splice(eligibleIndex, 1)[0];
                return entry.canvas;
            }
        }
        if (this.allocated >= this.maxCanvases) {
            // If we're at capacity, fall back to the oldest released canvas even if it's still
            // cooling down.
            let fallback = this.free.shift();
            return fallback?.canvas ?? null;
        }
        this.allocated += 1;
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
        this.free.push({ canvas, releasedAt: this.nowMs() });
    }
}
