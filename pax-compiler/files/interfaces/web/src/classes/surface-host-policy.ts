type SurfaceCanvasDescriptor = {
    id: string;
    key: string;
    left: number;
    top: number;
    width: number;
    height: number;
    replayPriority: number;
    surfaceSignature: string;
    transformSignature: string;
    hostSignature?: string;
    // Rust's allocation contract. Never infer a separate DPR in the DOM host.
    backingWidth?: number;
    backingHeight?: number;
    densityClamped?: boolean;
};

type LayerCanvasPlan = {
    layerId: number;
    active: boolean;
    densityPending?: boolean;
    surfaces: SurfaceCanvasDescriptor[];
};

// Admission and DOM installation must agree on what makes a plan different.
// Exclude densityPending: it controls reobservation, not installed geometry.
export function canvasPlanSignature(plan: LayerCanvasPlan): string {
    const parts = [String(plan.layerId), plan.active ? "1" : "0", String(plan.surfaces.length)];
    for (const descriptor of plan.surfaces) {
        parts.push(
            `${descriptor.id}|${descriptor.key}|${descriptor.left},${descriptor.top},${descriptor.width},${descriptor.height}`
            + `|${descriptor.surfaceSignature}|${descriptor.transformSignature}|${descriptor.hostSignature ?? ""}`
            + `|priority=${descriptor.replayPriority}`,
            `backing=${descriptor.backingWidth}x${descriptor.backingHeight}|clamped=${!!descriptor.densityClamped}`,
        );
    }
    return parts.join(";");
}

export function describeSurfaceHost(host?: Element | null) {
    // This tags a physical DOM host for retained-surface reuse. It is not a semantic node id.
    if (host == null) {
        return "none";
    }
    let role = host.getAttribute("data-role");
    let scrollerId = host.getAttribute("data-scroller-id");
    if (role != null && scrollerId != null) {
        return `${role}:${scrollerId}`;
    }
    return role
        ?? host.getAttribute("pax_id")
        ?? host.className
        ?? host.tagName.toLowerCase();
}

export function isIOSWebKitBrowser() {
    if (typeof navigator === "undefined") {
        return false;
    }
    let userAgent = navigator.userAgent;
    let isiOS = /iPhone|iPad|iPod/i.test(userAgent);
    return isiOS && /AppleWebKit/i.test(userAgent) && !/CriOS|FxiOS|EdgiOS/i.test(userAgent);
}


export type { SurfaceCanvasDescriptor, LayerCanvasPlan };
