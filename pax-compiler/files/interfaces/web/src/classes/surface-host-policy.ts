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
};

type LayerCanvasPlan = {
    layerId: number;
    active: boolean;
    surfaces: SurfaceCanvasDescriptor[];
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
const IOS_MAX_BACKING_AREA_CAP = IOS_MAX_BACKING_DIMENSION_CAP * IOS_MAX_BACKING_DIMENSION_CAP;
const IOS_SCROLLER_RENDER_DPR = 1.0;
const MIN_LOGICAL_TILE_SIZE = 256;
const TILE_OVERSCAN_COLUMNS = 0;
const TILE_OVERSCAN_ROWS = 0;
const MIN_UNTILED_RENDER_DPR = 1.0;
const PREWARM_VIEWPORT_PAD_X_MULTIPLIER = 1.0;
const PREWARM_VIEWPORT_PAD_Y_MULTIPLIER = 1.5;
const PREWARM_VIEWPORT_PAD_MIN_X = 512;
const PREWARM_VIEWPORT_PAD_MIN_Y = 512;
const FIREFOX_PREWARM_VIEWPORT_PAD_Y_MULTIPLIER = 3.0;
const FIREFOX_PREWARM_VIEWPORT_PAD_MIN_Y = 1536;
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
                replayPriority: 0,
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
    let tileDimensions = computeLogicalTileDimensions(
        host,
        contentWidth,
        contentHeight,
        horizontalScrollable,
        verticalScrollable,
    );
    if (contentWidth <= tileDimensions.width && contentHeight <= tileDimensions.height) {
        return [
            {
                id: String(layerId),
                key: "single",
                left: 0,
                top: 0,
                width: contentWidth,
                height: contentHeight,
                replayPriority: 0,
                surfaceSignature: `single:${contentWidth}x${contentHeight}@${describeSurfaceHost(host)}`,
                transformSignature: "0,0",
            },
        ];
    }
    let maxColumn = Math.max(0, Math.ceil(contentWidth / tileDimensions.width) - 1);
    let maxRow = Math.max(0, Math.ceil(contentHeight / tileDimensions.height) - 1);
    let prewarmPadYMultiplier = isFirefoxBrowser()
        ? FIREFOX_PREWARM_VIEWPORT_PAD_Y_MULTIPLIER
        : PREWARM_VIEWPORT_PAD_Y_MULTIPLIER;
    let prewarmPadMinY = isFirefoxBrowser()
        ? FIREFOX_PREWARM_VIEWPORT_PAD_MIN_Y
        : PREWARM_VIEWPORT_PAD_MIN_Y;
    let padX = horizontalScrollable
        ? Math.max(viewportWidth * PREWARM_VIEWPORT_PAD_X_MULTIPLIER, PREWARM_VIEWPORT_PAD_MIN_X)
        : 0;
    let padY = verticalScrollable
        ? Math.max(viewportHeight * prewarmPadYMultiplier, prewarmPadMinY)
        : 0;
    let paddedViewportWidth = viewportWidth + padX * 2;
    let paddedViewportHeight = viewportHeight + padY * 2;
    let paddedScrollX = clampOffset(scrollX - padX, contentWidth, paddedViewportWidth);
    let paddedScrollY = clampOffset(scrollY - padY, contentHeight, paddedViewportHeight);
    // Mirror the engine-side web defaults for budget estimation. Overscan remains axis-gated so a
    // vertical scroller does not pay for horizontal warm columns, and vice versa.
    let overscanColumns = horizontalScrollable ? TILE_OVERSCAN_COLUMNS : 0;
    let overscanRows = verticalScrollable ? TILE_OVERSCAN_ROWS : 0;
    if (iosHost) {
        overscanColumns = 0;
        overscanRows = 0;
    }
    let activeColumns = Math.min(
        maxColumn + 1,
        visibleTileSpan(paddedViewportWidth, tileDimensions.width, true) + overscanColumns * 2,
    );
    let activeRows = Math.min(
        maxRow + 1,
        visibleTileSpan(paddedViewportHeight, tileDimensions.height, true) + overscanRows * 2,
    );
    let startColumn = clampWindowStart(
        Math.floor(paddedScrollX / tileDimensions.width) - overscanColumns,
        maxColumn,
        activeColumns,
    );
    let startRow = clampWindowStart(
        Math.floor(paddedScrollY / tileDimensions.height) - overscanRows,
        maxRow,
        activeRows,
    );
    let endColumn = Math.min(maxColumn, startColumn + activeColumns - 1);
    let endRow = Math.min(maxRow, startRow + activeRows - 1);

    let descriptors: SurfaceCanvasDescriptor[] = [];
    for (let row = startRow; row <= endRow; row += 1) {
        for (let column = startColumn; column <= endColumn; column += 1) {
            let slotColumn = positiveModulo(column, activeColumns);
            let slotRow = positiveModulo(row, activeRows);
            let left = column * tileDimensions.width;
            let top = row * tileDimensions.height;
            let width = Math.max(1, Math.min(tileDimensions.width, contentWidth - left));
            let height = Math.max(1, Math.min(tileDimensions.height, contentHeight - top));
            let replayPriority = tileReplayPriority(
                left,
                top,
                width,
                height,
                scrollX,
                scrollY,
                viewportWidth,
                viewportHeight,
                tileDimensions.width,
                tileDimensions.height,
            );
            // Keep DOM ids and renderer keys stable by physical ring slot. Overlapping content
            // tiles retain their canvas/context across tile-window shifts; only the entering slot
            // is reassigned to a new content origin.
            let key = `${slotColumn}:${slotRow}`;
            descriptors.push({
                id: `layer-${layerId}-tile-${slotColumn}-${slotRow}`,
                key,
                left,
                top,
                width,
                height,
                replayPriority,
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

    descriptors.sort((left, right) => {
        let keyOrder = left.key.localeCompare(right.key);
        if (keyOrder !== 0) {
            return keyOrder;
        }
        return left.transformSignature.localeCompare(right.transformSignature);
    });

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

function effectiveRenderDpr(host?: HTMLElement) {
    let dpr = Math.max(1, globalThis.devicePixelRatio ?? 1);
    if (isIOSWebKitBrowser() && host?.dataset.role === "scroller-canvas-host") {
        dpr = Math.min(dpr, IOS_SCROLLER_RENDER_DPR);
    }
    return dpr;
}

function tileBackingLimits(host?: HTMLElement) {
    let dimension = targetTileBackingDimension(host);
    let area = dimension * dimension;
    if (isIOSWebKitBrowser()) {
        area = Math.min(area, IOS_MAX_BACKING_AREA_CAP);
    }
    return {
        width: dimension,
        height: dimension,
        area,
    };
}

function computeLogicalTileSize(host?: HTMLElement) {
    let dpr = effectiveRenderDpr(host);
    return Math.max(
        MIN_LOGICAL_TILE_SIZE,
        Math.floor(targetTileBackingDimension(host) / dpr),
    );
}

function computeLogicalTileDimensions(
    host: HTMLElement | undefined,
    contentWidth: number,
    contentHeight: number,
    horizontalScrollable: boolean,
    verticalScrollable: boolean,
) {
    let dpr = effectiveRenderDpr(host);
    let targetTileSize = computeLogicalTileSize(host);
    let limits = tileBackingLimits(host);
    let maxWidth = Math.max(1, Math.floor(limits.width / dpr));
    let maxHeight = Math.max(1, Math.floor(limits.height / dpr));
    let width = Math.max(1, Math.min(targetTileSize, maxWidth));
    let height = Math.max(1, Math.min(targetTileSize, maxHeight));

    if (verticalScrollable && !horizontalScrollable) {
        width = Math.max(1, Math.min(contentWidth, maxWidth));
        height = constrainAxisByArea(height, width, dpr, limits.area);
    } else if (horizontalScrollable && !verticalScrollable) {
        height = Math.max(1, Math.min(contentHeight, maxHeight));
        width = constrainAxisByArea(width, height, dpr, limits.area);
    } else {
        [width, height] = constrainTileArea(width, height, dpr, limits.area);
    }

    return { width, height };
}

function backingExtent(logicalExtent: number, dpr: number) {
    return Math.max(1, Math.ceil(Math.max(1, logicalExtent) * Math.max(1, dpr)));
}

function backingArea(width: number, height: number, dpr: number) {
    return backingExtent(width, dpr) * backingExtent(height, dpr);
}

function constrainAxisByArea(axis: number, crossAxis: number, dpr: number, maxArea: number) {
    let crossBacking = backingExtent(crossAxis, dpr);
    let maxAxis = Math.max(1, Math.floor(maxArea / crossBacking / Math.max(1, dpr)));
    return Math.max(1, Math.min(axis, maxAxis));
}

function constrainTileArea(width: number, height: number, dpr: number, maxArea: number) {
    let area = backingArea(width, height, dpr);
    if (area <= maxArea) {
        return [width, height];
    }

    let scale = Math.min(1, Math.sqrt(maxArea / area));
    width = Math.max(1, Math.floor(width * scale));
    height = Math.max(1, Math.floor(height * scale));
    if (backingArea(width, height, dpr) > maxArea) {
        height = constrainAxisByArea(height, width, dpr, maxArea);
    }
    if (backingArea(width, height, dpr) > maxArea) {
        width = constrainAxisByArea(width, height, dpr, maxArea);
    }
    return [width, height];
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
    let desiredDpr = effectiveRenderDpr(host);
    let limits = tileBackingLimits(host);
    let singleSurfaceDpr = Math.min(
        desiredDpr,
        limits.width / contentWidth,
        limits.height / contentHeight,
        Math.sqrt(limits.area / (contentWidth * contentHeight)),
    );
    return singleSurfaceDpr >= MIN_UNTILED_RENDER_DPR;
}

function tileReplayPriority(
    tileLeft: number,
    tileTop: number,
    tileWidth: number,
    tileHeight: number,
    viewportLeft: number,
    viewportTop: number,
    viewportWidth: number,
    viewportHeight: number,
    tileStepWidth: number,
    tileStepHeight: number,
) {
    let tileRight = tileLeft + tileWidth;
    let tileBottom = tileTop + tileHeight;
    let viewportRight = viewportLeft + viewportWidth;
    let viewportBottom = viewportTop + viewportHeight;
    if (
        tileRight >= viewportLeft
        && tileLeft <= viewportRight
        && tileBottom >= viewportTop
        && tileTop <= viewportBottom
    ) {
        return 0;
    }
    let dx = tileRight < viewportLeft
        ? viewportLeft - tileRight
        : tileLeft > viewportRight
            ? tileLeft - viewportRight
            : 0;
    let dy = tileBottom < viewportTop
        ? viewportTop - tileBottom
        : tileTop > viewportBottom
            ? tileTop - viewportBottom
            : 0;
    let xPriority = dx > 0 ? Math.ceil(dx / Math.max(1, tileStepWidth)) : 0;
    let yPriority = dy > 0 ? Math.ceil(dy / Math.max(1, tileStepHeight)) : 0;
    return Math.max(1, Math.max(xPriority, yPriority) + 1);
}

export function isIOSWebKitBrowser() {
    if (typeof navigator === "undefined") {
        return false;
    }
    let userAgent = navigator.userAgent;
    let isiOS = /iPhone|iPad|iPod/i.test(userAgent);
    return isiOS && /AppleWebKit/i.test(userAgent) && !/CriOS|FxiOS|EdgiOS/i.test(userAgent);
}

function isFirefoxBrowser() {
    if (typeof navigator === "undefined") {
        return false;
    }
    return /Firefox\//i.test(navigator.userAgent) && !/FxiOS/i.test(navigator.userAgent);
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

function positiveModulo(value: number, modulus: number) {
    return ((value % modulus) + modulus) % modulus;
}

function clampWindowStart(value: number, maxIndex: number, windowSize: number) {
    return clampIndex(value, Math.max(0, maxIndex - windowSize + 1));
}

export type { SurfaceCanvasDescriptor, LayerCanvasPlan };
