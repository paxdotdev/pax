type SurfaceCanvasDescriptor = {
    id: string;
    key: string;
    left: number;
    top: number;
    width: number;
    height: number;
    surfaceSignature: string;
    transformSignature: string;
};

const TARGET_TILE_BACKING_DIMENSION = 1664;
// Browser-owned scrollers are now capped to 1x desired DPR in the web chassis. Let them use
// physically wider tiles than the root viewport workaround so horizontal seam crossings happen
// less often without pushing nested warm-island count back up.
const SCROLLER_TARGET_TILE_BACKING_DIMENSION = 2496;
// Empirically, iOS Safari can cap renderable surface extents to 2048 even when WebGL reports
// larger sizes. Clamp to 2048 to avoid wgpu surface-config validation failures.
const IOS_FALLBACK_MAX_BACKING_DIMENSION = 2048;
const IOS_MAX_BACKING_DIMENSION_CAP = 2048;
const IOS_SCROLLER_RENDER_DPR = 1.0;
const MIN_LOGICAL_TILE_SIZE = 256;
const TILE_OVERSCAN_COLUMNS = 1;
const TILE_OVERSCAN_ROWS = 0;
const MIN_UNTILED_RENDER_DPR = 1.0;
// Keep tiling policy at the DOM-host layer instead of in nodes or renderer callsites. The current
// opt-in is:
// 1. any non-root browser-owned scroller surface in all browsers
// 2. the root viewport on iOS Safari only, where a single browser surface can still exceed the
//    compositor's safe backing-size limits
// The layer/renderer plumbing stays unchanged when browser policy broadens or narrows this later.

export function computeLayerCanvasPlan(
    layerId: number,
    host?: HTMLElement,
): SurfaceCanvasDescriptor[] {
    return computeLayerCanvasPlanInternal(layerId, host, false);
}

export function estimateWarmLayerCanvasCount(
    layerId: number,
    host?: HTMLElement,
) {
    return computeLayerCanvasPlanInternal(layerId, host, true).length;
}

function computeLayerCanvasPlanInternal(
    layerId: number,
    host: HTMLElement | undefined,
    treatColdAsWarm: boolean,
): SurfaceCanvasDescriptor[] {
    if (layerId !== 0) {
        if (host == null) {
            return [];
        }
        // Non-root layers are reserved for browser-owned scroller islands only. If they are not
        // attached to the expected scroller host (or are still cold), return no surfaces.
        if (host.dataset.role !== "scroller-canvas-host") {
            return [];
        }
        if (!treatColdAsWarm && host.dataset.warmState !== "warm") {
            return [];
        }
    }

    if (host == null || !shouldTileLayerSurface(layerId, host)) {
        let width = host?.clientWidth ?? 0;
        let height = host?.clientHeight ?? 0;
        return [
            {
                id: String(layerId),
                key: "single",
                left: 0,
                top: 0,
                width,
                height,
                surfaceSignature: `single:${width}x${height}@${describeSurfaceHost(host)}`,
                transformSignature: "0,0",
            },
        ];
    }

    // Tiling is intentionally decided at the DOM-host layer instead of in individual nodes.
    // ScrollerUpdate patches already carry the engine-approved viewport and scroll position, so we
    // derive the active tile window from those attributes rather than wiring raw scroll position
    // directly into rendering decisions here.
    let contentWidth = Math.max(0, host.clientWidth);
    let contentHeight = Math.max(0, host.clientHeight);
    let viewportWidth = contentWidth;
    let viewportHeight = contentHeight;
    let scrollX = 0;
    let scrollY = 0;
    if (host.dataset.role === "scroller-canvas-host" || host.id === "mount") {
        viewportWidth = clampDimension(readFloat(host.dataset.viewportWidth), contentWidth);
        viewportHeight = clampDimension(readFloat(host.dataset.viewportHeight), contentHeight);
        scrollX = clampOffset(readFloat(host.dataset.approvedScrollX), contentWidth, viewportWidth);
        scrollY = clampOffset(readFloat(host.dataset.approvedScrollY), contentHeight, viewportHeight);
    }
    let horizontalScrollable = contentWidth > viewportWidth + 0.5;
    let verticalScrollable = contentHeight > viewportHeight + 0.5;
    let iosHost =
        isIOSWebKitBrowser()
        && (host.dataset.role === "scroller-canvas-host" || host.id === "mount");
    let tileSize = computeLogicalTileSize(host);
    if (contentWidth <= tileSize && contentHeight <= tileSize) {
        return [
            {
                id: String(layerId),
                key: "single",
                left: 0,
                top: 0,
                width: contentWidth,
                height: contentHeight,
                surfaceSignature: `single:${contentWidth}x${contentHeight}@${describeSurfaceHost(host)}`,
                transformSignature: "0,0",
            },
        ];
    }
    let maxColumn = Math.max(0, Math.ceil(contentWidth / tileSize) - 1);
    let maxRow = Math.max(0, Math.ceil(contentHeight / tileSize) - 1);
    // Until we have finer per-node/per-tile culling, every overscan tile multiplies the retained
    // scene replay cost. Keep the first shipping-biased tiled pass to the visible tile window and
    // only revisit overscan once tile pooling/culling is in place.
    let overscanColumns = horizontalScrollable ? TILE_OVERSCAN_COLUMNS : 0;
    let overscanRows = verticalScrollable ? TILE_OVERSCAN_ROWS : 0;
    if (iosHost) {
        overscanColumns = 0;
        overscanRows = 0;
    }
    let activeColumns = Math.min(
        maxColumn + 1,
        visibleTileSpan(viewportWidth, tileSize, !iosHost) + overscanColumns * 2,
    );
    let activeRows = Math.min(
        maxRow + 1,
        visibleTileSpan(viewportHeight, tileSize, !iosHost) + overscanRows * 2,
    );
    let startColumn = clampWindowStart(
        Math.floor(scrollX / tileSize) - overscanColumns,
        maxColumn,
        activeColumns,
    );
    let startRow = clampWindowStart(
        Math.floor(scrollY / tileSize) - overscanRows,
        maxRow,
        activeRows,
    );
    let endColumn = Math.min(maxColumn, startColumn + activeColumns - 1);
    let endRow = Math.min(maxRow, startRow + activeRows - 1);

    let descriptors: SurfaceCanvasDescriptor[] = [];
    for (let row = startRow; row <= endRow; row += 1) {
        for (let column = startColumn; column <= endColumn; column += 1) {
            let slotColumn = column - startColumn;
            let slotRow = row - startRow;
            let left = column * tileSize;
            let top = row * tileSize;
            let width = Math.max(1, Math.min(tileSize, contentWidth - left));
            let height = Math.max(1, Math.min(tileSize, contentHeight - top));
            // Keep DOM ids and renderer keys stable by viewport slot instead of absolute tile
            // index. Scroll should slide origins under an existing surface set rather than making
            // Rust churn renderers every time the visible window crosses a tile boundary.
            let key = `${slotColumn}:${slotRow}`;
            descriptors.push({
                id: `layer-${layerId}-tile-${slotColumn}-${slotRow}`,
                key,
                left,
                top,
                width,
                height,
                // Scroll can slide the active tile window without changing the keyed surface set.
                // Keep the physical surface signature separate from the tile origin so the chassis
                // can issue a cheap transform-only refresh for that case.
                surfaceSignature: [
                    "tile",
                    `${slotColumn},${slotRow}`,
                    `${width}x${height}`,
                    `${viewportWidth}x${viewportHeight}`,
                    describeSurfaceHost(host),
                ].join(":"),
                transformSignature: `${left},${top}`,
            });
        }
    }

    return descriptors;
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

let cachedIOSMaxBackingDimension: number | null = null;

function getIOSMaxBackingDimension() {
    if (!isIOSWebKitBrowser()) {
        return undefined;
    }
    if (cachedIOSMaxBackingDimension != null) {
        return cachedIOSMaxBackingDimension;
    }
    let maxDimension = IOS_FALLBACK_MAX_BACKING_DIMENSION;
    try {
        let probe = document.createElement("canvas");
        let gl =
            (probe.getContext("webgl2") as WebGL2RenderingContext | null)
            ?? (probe.getContext("webgl") as WebGLRenderingContext | null)
            ?? (probe.getContext("experimental-webgl") as WebGLRenderingContext | null);
        if (gl) {
            let maxTexture = gl.getParameter(gl.MAX_TEXTURE_SIZE) as number;
            let maxRenderbuffer = gl.getParameter(gl.MAX_RENDERBUFFER_SIZE) as number;
            let reported = Math.min(
                Number.isFinite(maxTexture) ? maxTexture : maxDimension,
                Number.isFinite(maxRenderbuffer) ? maxRenderbuffer : maxDimension,
            );
            if (Number.isFinite(reported) && reported > 0) {
                maxDimension = Math.min(IOS_MAX_BACKING_DIMENSION_CAP, reported);
            }
            let lose = (gl as WebGLRenderingContext).getExtension("WEBGL_lose_context");
            lose?.loseContext();
        }
    } catch {
        // Ignore probing failures and fall back to a conservative default.
    }
    cachedIOSMaxBackingDimension = maxDimension;
    return maxDimension;
}

function targetTileBackingDimension(host?: HTMLElement) {
    if (isIOSWebKitBrowser()) {
        return getIOSMaxBackingDimension() ?? IOS_FALLBACK_MAX_BACKING_DIMENSION;
    }
    return host?.dataset.role === "scroller-canvas-host"
        ? SCROLLER_TARGET_TILE_BACKING_DIMENSION
        : TARGET_TILE_BACKING_DIMENSION;
}

function computeLogicalTileSize(host?: HTMLElement) {
    let dpr = Math.max(1, globalThis.devicePixelRatio ?? 1);
    if (isIOSWebKitBrowser() && host?.dataset.role === "scroller-canvas-host") {
        dpr = Math.min(dpr, IOS_SCROLLER_RENDER_DPR);
    }
    return Math.max(
        MIN_LOGICAL_TILE_SIZE,
        Math.floor(targetTileBackingDimension(host) / dpr),
    );
}

function shouldTileLayerSurface(layerId: number, host: HTMLElement) {
    if (layerId === 0) {
        let tileSize = computeLogicalTileSize(host);
        // Root tiling is still an iOS viewport workaround only. Keep layer 0 anchored at #mount;
        // descendant browser-owned scroller hosts are handled by the non-root path below.
        return host.id === "mount"
            && isIOSWebKitBrowser()
            && (host.clientWidth > tileSize || host.clientHeight > tileSize);
    }

    if (host.dataset.role !== "scroller-canvas-host") {
        return false;
    }

    if (canRenderHostAsSingleSurface(host)) {
        // Moderate nested scrollers are cheaper and visually better as one warmed surface than as
        // a 2x2 tiled island. Only split them once a single surface would have to drop below a
        // roughly-native backing scale.
        return false;
    }

    // Any non-root browser-owned scroller can grow far beyond the viewport, even on desktop
    // Chrome. Keeping those surfaces tiled avoids oversized initial browser surfaces and unifies
    // root and nested scroller behavior behind the same lifecycle.
    return true;
}

function canRenderHostAsSingleSurface(host: HTMLElement) {
    let contentWidth = Math.max(1, host.clientWidth);
    let contentHeight = Math.max(1, host.clientHeight);
    let desiredDpr = Math.max(1, globalThis.devicePixelRatio ?? 1);
    let backingDimension = targetTileBackingDimension(host);
    let singleSurfaceDpr = Math.min(
        desiredDpr,
        backingDimension / contentWidth,
        backingDimension / contentHeight,
    );
    return singleSurfaceDpr >= MIN_UNTILED_RENDER_DPR;
}

export function isIOSWebKitBrowser() {
    if (typeof navigator === "undefined") {
        return false;
    }
    let userAgent = navigator.userAgent;
    let isiOS = /iPhone|iPad|iPod/i.test(userAgent);
    return isiOS && /AppleWebKit/i.test(userAgent) && !/CriOS|FxiOS|EdgiOS/i.test(userAgent);
}

function readFloat(value?: string) {
    if (value == null || value.length === 0) {
        return undefined;
    }
    let parsed = Number.parseFloat(value);
    return Number.isFinite(parsed) ? parsed : undefined;
}

function clampDimension(value: number | undefined, fallback: number) {
    if (value == null) {
        return Math.max(0, fallback);
    }
    return Math.max(0, Math.min(value, fallback));
}

function clampOffset(value: number | undefined, content: number, viewport: number) {
    if (content <= viewport) {
        return 0;
    }
    if (value == null) {
        return 0;
    }
    return Math.max(0, Math.min(value, Math.max(0, content - viewport)));
}

function clampIndex(value: number, max: number) {
    return Math.max(0, Math.min(value, max));
}

function visibleTileSpan(viewportSize: number, tileSize: number, includeExtraSlot: boolean) {
    // Reserve one extra slot so a viewport smaller than a tile can cross a tile boundary without
    // immediately growing the active surface set on the first few pixels of scroll.
    let base = Math.ceil(Math.max(0, viewportSize) / tileSize);
    let extra = includeExtraSlot ? 1 : 0;
    return Math.max(1, base + extra);
}

function clampWindowStart(value: number, maxIndex: number, windowSize: number) {
    return clampIndex(value, Math.max(0, maxIndex - windowSize + 1));
}

export type { SurfaceCanvasDescriptor };
