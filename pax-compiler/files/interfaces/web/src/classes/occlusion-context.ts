import {Layer} from "./layer";
import {ObjectManager} from "../pools/object-manager";
import {ARRAY, DIV, LAYER} from "../pools/supported-objects";

import { CLIPPING_CONTAINER, NATIVE_LEAF_CLASS } from "../utils/constants";
import { affineMultiply } from "../utils/helpers";
import type { NativeMaskEntry } from "./messages/native-mask-update-patch";

const SVG_NS = "http://www.w3.org/2000/svg";

export class OcclusionLayerManager {
    private layers?: Layer[];
    private canvasMap?: Map<string, HTMLCanvasElement>;
    public parent?: Element;
    private objectManager: ObjectManager;
    private containers: Map<number, Container>;
    private parentFrameHosts: Map<number, Element>;
    private scrollerIslandHosts: Map<number, ScrollerIslandHosts>;
    private layerIslandOwners: Map<number, number>;
    private pendingLayerIslandClaims: Map<number, number>;
    private effects: SvgEffectManager;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
        this.containers = new Map();
        this.parentFrameHosts = new Map();
        this.scrollerIslandHosts = new Map();
        this.layerIslandOwners = new Map();
        this.pendingLayerIslandClaims = new Map();
        this.effects = new SvgEffectManager();
    }

    attach(parent: Element, canvasMap: Map<string, HTMLCanvasElement>) {
        this.layers = this.objectManager.getFromPool(ARRAY);
        this.parent = parent;
        this.canvasMap = canvasMap;
        this.effects.attach(parent);
        this.growTo(0);
    }

    growTo(newOcclusionLayerId: number) {
        let occlusionLayerCount = newOcclusionLayerId + 1;
        if(this.layers!.length < occlusionLayerCount) {
            for(let i = this.layers!.length; i < occlusionLayerCount; i++) {
                let newLayer: Layer = this.objectManager.getFromPool(LAYER, this.objectManager);
                newLayer.build(
                    this.parent!,
                    i,
                    this.canvasMap!,
                );
                this.layers!.push(newLayer);
                this.applyPendingLayerIslandClaim(i);
            }
        }
    }

    shrinkTo(layerCount: number){
        if(this.layers === undefined){
            return
        }
        // Rust publishes `ShrinkLayersTo` as the exact number of logical layers that should be
        // alive, not the maximum retained layer id. Historically `OcclusionUpdate` calls also grew
        // the layer stack as a side effect. Now layer sizing is driven by this message alone, so
        // handle both growth and shrinkage here.
        if (layerCount > 0) {
            this.growTo(layerCount - 1);
        }
        while (this.layers.length > layerCount) {
            let i = this.layers.length - 1;
            this.layerIslandOwners.delete(i);
            this.objectManager.returnToPool(LAYER, this.layers[i]);
            this.layers.pop();
        }
    }

    addElement(
        element: HTMLElement,
        parent_container: number | undefined,
        occlusionLayerId: number,
    ) {
        this.growTo(occlusionLayerId);
        let ownershipChanged = false;
        if (parent_container != null) {
            ownershipChanged = this.claimLayerForParentFrame(occlusionLayerId, parent_container);
        }
        let attach_point = this.getOrCreateContainer(parent_container, occlusionLayerId);
        if (!attach_point.contains(element)) {
            attach_point.appendChild(element);
        }
        refreshLeafOpacities(element);
        return ownershipChanged;
    }

    updateElementMask(
        element: HTMLElement,
        id: number,
        entries: NativeMaskEntry[],
        sizeX: number | undefined,
        sizeY: number | undefined,
    ) {
        this.effects.updateMask(id, entries, sizeX ?? 0, sizeY ?? 0);
        if (entries.length === 0) {
            element.classList.remove(MASKED_NATIVE_LEAF_CLASS);
            clearMaskStyles(element);
            refreshLeafOpacities(element);
            return;
        }

        let maskValue = `url(#${nativeMaskId(id)})`;
        element.classList.add(MASKED_NATIVE_LEAF_CLASS);
        applyMaskStyles(element, maskValue);
        refreshLeafOpacities(element);
    }

    // If a div for the container referenced already exists, returns it. if not,
    // create it (and all non-existent parents)
    getOrCreateContainer(id: number | undefined, occlusionLayerId: number) {
        let layer = this.layers![occlusionLayerId]!.native!;
        if (id == undefined) {
            return layer;
        }

        let host = this.parentFrameHosts.get(id);
        if (host != null) {
            return host;
        }

        let elem = layer.querySelector(`[data-container-id="${id}"]`);
        if (elem != undefined) {
            return elem!;
        }

        let container = this.containers.get(id);
        if (container == null) {
            throw new Error("something referenced a container that doesn't exist");
        }
        let new_container: HTMLDivElement = this.objectManager.getFromPool(DIV);
        new_container.dataset.containerId = id.toString();
        new_container.setAttribute("class", CLIPPING_CONTAINER);
        applyContainerStyles(
            new_container,
            container.clipPathValue(),
            container.styles.opacity,
        );

        let parent_container = this.getOrCreateContainer(container.parentFrame, occlusionLayerId);
        parent_container.appendChild(new_container);
        return new_container;
    }

    addContainer(id: number, parentId: number | undefined) {
        this.containers.set(id, new Container(id, parentId));
    }

    registerParentFrameHost(id: number, host: Element) {
        this.parentFrameHosts.set(id, host);
    }

    registerScrollerIslandHosts(id: number, canvasHost: Element, nativeHost: Element) {
        this.scrollerIslandHosts.set(id, { canvasHost, nativeHost });
        let ownershipChanged = false;
        this.pendingLayerIslandClaims.forEach((ownerId, layerId) => {
            if (ownerId === id) {
                ownershipChanged = this.applyPendingLayerIslandClaim(layerId) || ownershipChanged;
            }
        });
        return ownershipChanged;
    }

    claimLayerForScrollerIsland(layerId: number, scrollerId: number) {
        if (layerId === 0) {
            // The root layer stays on the top-level surface stack. Reparenting it into a
            // browser-owned scroller host couples the app viewport to that host and can create DOM
            // cycles with the root native overlay.
            return false;
        }
        this.pendingLayerIslandClaims.set(layerId, scrollerId);
        const layer = this.layers?.[layerId];
        if (layer) {
            layer.setIslandOnly(true);
            if (!this.scrollerIslandHosts.get(scrollerId)) {
                layer.detachCanvases();
            }
        }
        return this.applyPendingLayerIslandClaim(layerId);
    }

    private applyPendingLayerIslandClaim(layerId: number) {
        let scrollerId = this.pendingLayerIslandClaims.get(layerId);
        if (scrollerId == null) {
            return false;
        }
        let island = this.scrollerIslandHosts.get(scrollerId);
        let layer = this.layers?.[layerId];
        if (layer) {
            layer.setIslandOnly(true);
        }
        if (island == null || layer == null) {
            layer?.detachCanvases();
            return false;
        }
        let existingOwner = this.layerIslandOwners.get(layerId);
        if (existingOwner === scrollerId) {
            this.pendingLayerIslandClaims.delete(layerId);
            return false;
        }
        // `content_layer_id` is published directly by Rust for browser-owned scroller islands.
        // Treat that explicit claim as authoritative even if provisional parent-frame routing
        // attached the layer elsewhere first; message ordering can otherwise leave nested canvas
        // layers stranded outside their intended host.
        layer.attachToParents(island.canvasHost, island.nativeHost);
        layer.setCanvasZIndex("0");
        if (layer.native) {
            layer.native.style.zIndex = "1";
        }
        this.layerIslandOwners.set(layerId, scrollerId);
        this.pendingLayerIslandClaims.delete(layerId);
        return true;
    }

    unregisterParentFrameHost(id: number, host?: Element) {
        let current = this.parentFrameHosts.get(id);
        if (current == null) {
            return;
        }
        if (host == null || current === host) {
            this.parentFrameHosts.delete(id);
        }
    }

    unregisterScrollerIslandHosts(
        id: number,
        canvasHost?: Element,
        nativeHost?: Element,
    ) {
        let current = this.scrollerIslandHosts.get(id);
        if (current == null) {
            return false;
        }
        if (
            (canvasHost != null && current.canvasHost !== canvasHost)
            || (nativeHost != null && current.nativeHost !== nativeHost)
        ) {
            return false;
        }
        this.scrollerIslandHosts.delete(id);
        let ownershipChanged = false;
        this.layerIslandOwners.forEach((ownerId, layerId) => {
            if (ownerId !== id) {
                return;
            }
            let layer = this.layers?.[layerId];
            if (layer == null || this.parent == null) {
                this.layerIslandOwners.delete(layerId);
                return;
            }
            layer.setIslandOnly(true);
            layer.attachToParents(this.parent, this.parent);
            layer.setCanvasZIndex(String(layerId * 2));
            if (layer.native) {
                layer.native.style.zIndex = String(layerId * 2 + 1);
            }
            this.layerIslandOwners.delete(layerId);
            ownershipChanged = true;
        });
        return ownershipChanged;
    }

    estimateScrollerIslandOwnedLayers(id: number, fallbackLayerId?: number) {
        let layerIds: number[] = [];
        this.layerIslandOwners.forEach((ownerId, layerId) => {
            if (ownerId === id) {
                layerIds.push(layerId);
            }
        });
        if (layerIds.length === 0 && fallbackLayerId != null) {
            layerIds.push(fallbackLayerId);
        }
        layerIds.sort((left, right) => left - right);
        return layerIds;
    }

    updateContainer(id: number, styles: Partial<ContainerStyle>) {
        let container = this.containers.get(id);
        if (container == null) {
            throw new Error("tried to update non existent container");
        }
        container.update(styles);
        this.effects.updateClipPath(container.clipPathId(), container.clipPathData());
        this.applyContainerClipPath(id);
    }

    updateContainerParent(id: number, new_parent_id: number | undefined) {
        const container = this.containers.get(id);
        if (container == null) {
            throw new Error(`Container with id ${id} does not exist`);
        }

        container.parentFrame = new_parent_id;

        this.layers!.forEach((layer, layerIndex) => {
            const currentElement = layer.native!.querySelector(`[data-container-id="${id}"]`) as HTMLElement;
            if (currentElement) {
                const newParentElement = this.getOrCreateContainer(new_parent_id, layerIndex);
                if (currentElement.parentElement !== newParentElement) {
                    newParentElement.appendChild(currentElement);
                    refreshLeafOpacities(currentElement);
                }
            }
        });
    }

    removeContainer(id: number) {
        let container = this.containers.get(id);
        if (container == null) {
            throw new Error(`tried to delete non-existent container with id ${id}`);
        }
        this.containers.delete(id);
        this.effects.updateClipPath(container.clipPathId(), undefined);

        let existing_layer_instantiations = document.querySelectorAll(`[data-container-id="${id}"]`);
        existing_layer_instantiations.forEach((elem, _key, _parent) => {
            let parent = elem.parentElement;
            if (elem.children.length > 0) {
                throw new Error(`tried to remove container width id ${id} while children still present`);
            }
            parent!.removeChild(elem);
            this.objectManager.returnToPool(DIV, elem);
        })
    }

    cleanUp(){
        if(this.layers != undefined){
            this.layers.forEach((layer) => {
                this.objectManager.returnToPool(LAYER, layer);
            });
        }
        this.containers.clear();
        this.parentFrameHosts.clear();
        this.scrollerIslandHosts.clear();
        this.layerIslandOwners.clear();
        this.pendingLayerIslandClaims.clear();
        this.effects.cleanUp();
        this.canvasMap = undefined;
        this.parent = undefined;
    }

    syncLayerCanvasLayouts() {
        this.layers?.forEach((layer) => layer.syncCanvasLayout());
    }

    setRootPageScrollMode(active: boolean) {
        let rootNative = this.layers?.[0]?.native;
        if (rootNative == null) {
            return;
        }
        if (active) {
            // Root page-scroll delegation needs the browser to account for overflow that extends
            // past the initial viewport-sized overlay. Relax paint containment only for the root
            // native overlay while this mode is active so nested overlays keep their tighter
            // containment defaults.
            rootNative.style.overflow = "visible";
        } else {
            rootNative.style.overflow = "";
        }
    }

    private applyContainerClipPath(id: number) {
        let container = this.containers.get(id);
        if (container == null) {
            return;
        }
        let clipValue = container.clipPathValue();
        document
            .querySelectorAll(`[data-container-id="${id}"]`)
            .forEach((elem) =>
                {
                    applyContainerStyles(
                        elem as HTMLElement,
                        clipValue,
                        container.styles.opacity,
                    );
                    refreshLeafOpacities(elem as HTMLElement);
                },
            );
    }

    private claimLayerForParentFrame(layerId: number, parentFrameId: number) {
        if (layerId === 0) {
            return false;
        }
        let layer = this.layers?.[layerId];
        // Non-root layers are dedicated to browser-owned scroller islands. Avoid routing them to
        // ancestor frame hosts; they should only attach once their owning scroller claims them.
        if (layer?.isIslandOnly()) {
            return false;
        }
        let ownerFrameId: number | undefined = parentFrameId;
        let island: ScrollerIslandHosts | undefined;
        while (ownerFrameId != null) {
            island = this.scrollerIslandHosts.get(ownerFrameId);
            if (island != null) {
                break;
            }
            ownerFrameId = this.containers.get(ownerFrameId)?.parentFrame;
        }
        if (island == null) {
            return false;
        }
        let existingOwner = this.layerIslandOwners.get(layerId);
        if (existingOwner === ownerFrameId) {
            return false;
        }
        if (existingOwner != null && existingOwner !== ownerFrameId) {
            return false;
        }
        if (layer == null) {
            return false;
        }
        layer.setIslandOnly(true);
        // Keep layer ownership scoped to descendant content layers only. The root layer stays on
        // the top-level surface stack, while nested island layers move into the browser-owned
        // canvas/native hosts that correspond to their parent frame.
        layer.attachToParents(island.canvasHost, island.nativeHost);
        layer.setCanvasZIndex("0");
        if (layer.native) {
            layer.native.style.zIndex = "1";
        }
        this.layerIslandOwners.set(layerId, ownerFrameId!);
        return true;
    }
}

type ScrollerIslandHosts = {
    canvasHost: Element;
    nativeHost: Element;
};

class Container {
    id: number;
    parentFrame: number | undefined;
    styles: ContainerStyle;

    constructor(id: number, parentId: number | undefined) {
        this.parentFrame = parentId;
        this.styles = new ContainerStyle();
        this.id = id;
    }

    update(patch: Partial<ContainerStyle>) {
        this.styles = {...this.styles, ...patch};
    }

    clipPathId() {
        return `pax-container-clip-${this.id}`;
    }

    clipPathValue() {
        if (!this.styles.clipContent) {
            return "none";
        }
        return `url(#${this.clipPathId()})`;
    }

    clipPathData() {
        if (!this.styles.clipContent) {
            return undefined;
        }
        if (this.styles.clipPath && this.styles.clipPath.length > 0) {
            return this.styles.clipPath;
        }
        return getRectClipPathData(this.styles.width!, this.styles.height!, this.styles.transform!);
    }
}

class SvgEffectManager {
    private root?: SVGSVGElement;
    private defs?: SVGDefsElement;

    attach(parent: Element) {
        this.root = document.createElementNS(SVG_NS, "svg");
        this.root.setAttribute("width", "0");
        this.root.setAttribute("height", "0");
        this.root.setAttribute("aria-hidden", "true");
        this.root.style.position = "absolute";
        this.root.style.width = "0";
        this.root.style.height = "0";
        this.root.style.pointerEvents = "none";

        this.defs = document.createElementNS(SVG_NS, "defs");
        this.root.appendChild(this.defs);
        parent.appendChild(this.root);
    }

    updateClipPath(id: string, pathData: string | undefined) {
        if (!this.defs) {
            return;
        }

        if (!pathData || pathData.length === 0) {
            this.removeElement(id);
            return;
        }

        let clipPath = this.getOrCreateClipPath(id);
        let path = this.getOrCreatePathChild(clipPath);
        if (path.getAttribute("d") !== pathData) {
            path.setAttribute("d", pathData);
        }
    }

    updateMask(id: number, entries: NativeMaskEntry[], sizeX: number, sizeY: number) {
        if (!this.defs) {
            return;
        }

        if (!Number.isFinite(sizeX) || !Number.isFinite(sizeY) || sizeX <= 0 || sizeY <= 0) {
            this.removeElement(nativeMaskId(id));
            this.removeUnusedMaskClips(id, new Set());
            return;
        }

        if (entries.length === 0) {
            this.removeElement(nativeMaskId(id));
            this.removeUnusedMaskClips(id, new Set());
            return;
        }

        let mask = this.getOrCreateMask(id);
        mask.setAttribute("x", "0");
        mask.setAttribute("y", "0");
        mask.setAttribute("width", `${sizeX}`);
        mask.setAttribute("height", `${sizeY}`);
        let backdrop = this.getOrCreateBackdrop(mask);
        backdrop.setAttribute("x", "0");
        backdrop.setAttribute("y", "0");
        backdrop.setAttribute("width", `${sizeX}`);
        backdrop.setAttribute("height", `${sizeY}`);

        Array.from(mask.children)
            .filter((child) => child !== backdrop)
            .forEach((child) => child.remove());

        let desiredClipIds = new Set<string>();
        entries.forEach((entry, index) => {
            let path = document.createElementNS(SVG_NS, "path");
            path.setAttribute("d", entry.path);
            path.setAttribute("fill", "black");
            path.setAttribute("fill-opacity", `${entry.opacity ?? 1}`);
            mask.appendChild(this.wrapWithClips(id, index, path, entry.clips, desiredClipIds));
        });
        this.removeUnusedMaskClips(id, desiredClipIds);
    }

    cleanUp() {
        this.root?.remove();
        this.root = undefined;
        this.defs = undefined;
    }

    private wrapWithClips(
        id: number,
        entryIndex: number,
        element: SVGElement,
        clips: string[],
        desiredClipIds: Set<string>,
    ) {
        let current: SVGElement = element;
        [...clips].reverse().forEach((clipPathData, clipIndex) => {
            let clipPathId = nativeMaskClipId(id, entryIndex, clipIndex);
            desiredClipIds.add(clipPathId);
            this.updateClipPath(clipPathId, clipPathData);

            let group = document.createElementNS(SVG_NS, "g");
            group.setAttribute("clip-path", `url(#${clipPathId})`);
            group.appendChild(current);
            current = group;
        });
        return current;
    }

    private getOrCreateClipPath(id: string) {
        let existing = this.defs?.querySelector<SVGClipPathElement>(`#${CSS.escape(id)}`);
        if (existing) {
            return existing;
        }

        let clipPath = document.createElementNS(SVG_NS, "clipPath");
        clipPath.setAttribute("id", id);
        clipPath.setAttribute("clipPathUnits", "userSpaceOnUse");
        this.defs?.appendChild(clipPath);
        return clipPath;
    }

    private getOrCreatePathChild(parent: SVGElement) {
        let existing = parent.querySelector<SVGPathElement>(":scope > path");
        if (existing) {
            return existing;
        }

        let path = document.createElementNS(SVG_NS, "path");
        parent.appendChild(path);
        return path;
    }

    private getOrCreateMask(id: number) {
        let existing = this.defs?.querySelector<SVGMaskElement>(`#${CSS.escape(nativeMaskId(id))}`);
        if (existing) {
            return existing;
        }

        let mask = document.createElementNS(SVG_NS, "mask");
        mask.setAttribute("id", nativeMaskId(id));
        mask.setAttribute("maskUnits", "userSpaceOnUse");
        mask.setAttribute("maskContentUnits", "userSpaceOnUse");
        mask.setAttribute("mask-type", "luminance");
        this.defs?.appendChild(mask);
        return mask;
    }

    private getOrCreateBackdrop(mask: SVGMaskElement) {
        let existing = mask.querySelector<SVGRectElement>(':scope > rect[data-role="backdrop"]');
        if (existing) {
            return existing;
        }

        let backdrop = document.createElementNS(SVG_NS, "rect");
        backdrop.dataset.role = "backdrop";
        backdrop.setAttribute("fill", "white");
        mask.appendChild(backdrop);
        return backdrop;
    }

    private removeUnusedMaskClips(id: number, desiredIds: Set<string>) {
        this.defs
            ?.querySelectorAll(`[id^="${nativeMaskClipPrefix(id)}"]`)
            .forEach((elem) => {
                if (!desiredIds.has(elem.id)) {
                    elem.remove();
                }
            });
    }

    private removeElement(id: string) {
        this.defs?.querySelector(`#${CSS.escape(id)}`)?.remove();
    }
}

const MASKED_NATIVE_LEAF_CLASS = "masked-native-leaf";

function nativeMaskId(id: number) {
    return `pax-native-mask-${id}`;
}

function nativeMaskClipPrefix(id: number) {
    return `pax-native-mask-${id}-clip-`;
}

function nativeMaskClipId(id: number, entryIndex: number, clipIndex: number) {
    return `${nativeMaskClipPrefix(id)}${entryIndex}-${clipIndex}`;
}

function applyContainerClipPath(element: HTMLElement, value: string) {
    element.style.clipPath = value;
    (element.style as any).webkitClipPath = value;
}

function applyContainerStyles(element: HTMLElement, clipPath: string, opacity: number) {
    applyContainerClipPath(element, clipPath);
    element.dataset.paxContainerOpacity = `${opacity}`;
    element.style.opacity = "1";
}

export function setLeafLocalOpacity(element: HTMLElement, opacity: number) {
    element.dataset.paxLocalOpacity = `${opacity}`;
    applyEffectiveLeafOpacity(element);
}

function applyEffectiveLeafOpacity(element: HTMLElement) {
    let opacity = readOpacityValue(element.dataset.paxLocalOpacity, 1);
    let current = element.parentElement;
    while (current) {
        if (current.classList.contains(CLIPPING_CONTAINER)) {
            opacity *= readOpacityValue(
                (current as HTMLElement).dataset.paxContainerOpacity,
                1,
            );
        }
        current = current.parentElement;
    }
    element.style.opacity = `${Math.min(Math.max(opacity, 0), 1)}`;
}

function refreshLeafOpacities(root: ParentNode) {
    if (root instanceof HTMLElement && root.classList.contains(NATIVE_LEAF_CLASS)) {
        applyEffectiveLeafOpacity(root);
    }
    root.querySelectorAll?.(`.${NATIVE_LEAF_CLASS}`).forEach((leaf) => {
        applyEffectiveLeafOpacity(leaf as HTMLElement);
    });
}

function readOpacityValue(value: string | undefined, fallback: number) {
    if (value == null) {
        return fallback;
    }
    let parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
}

function clearMaskStyles(element: HTMLElement) {
    element.style.maskImage = "";
    element.style.maskRepeat = "";
    element.style.maskPosition = "";
    (element.style as any).webkitMaskImage = "";
    (element.style as any).webkitMaskRepeat = "";
    (element.style as any).webkitMaskPosition = "";
}

function applyMaskStyles(element: HTMLElement, maskValue: string) {
    clearMaskStyles(element);
    // Blink/WebKit can incorrectly cull transformed native text under SVG masks
    // when the generic mask path is combined with the browser's paint containment
    // optimizations. Prefer the prefixed mask-image path on those engines.
    if (prefersWebkitMaskProperties()) {
        (element.style as any).webkitMaskImage = maskValue;
        (element.style as any).webkitMaskRepeat = "no-repeat";
        (element.style as any).webkitMaskPosition = "0px 0px";
        return;
    }

    element.style.maskImage = maskValue;
    element.style.maskRepeat = "no-repeat";
    element.style.maskPosition = "0px 0px";
}

function prefersWebkitMaskProperties() {
    if (typeof navigator === "undefined") {
        return false;
    }
    let userAgent = navigator.userAgent;
    return /AppleWebKit/i.test(userAgent) && !/Firefox/i.test(userAgent);
}

function getRectClipPathData(width: number, height: number, transform: number[]) {
    let point0 = affineMultiply([0, 0], transform);
    let point1 = affineMultiply([width, 0], transform);
    let point2 = affineMultiply([width, height], transform);
    let point3 = affineMultiply([0, height], transform);
    return `M${point0[0]},${point0[1]}L${point1[0]},${point1[1]}L${point2[0]},${point2[1]}L${point3[0]},${point3[1]}Z`;
}

export class ContainerStyle {
    clipContent: boolean;
    transform: number[];
    width: number;
    height: number;
    opacity: number;
    clipPath?: string;

    constructor() {
        this.clipContent = true;
        this.transform = [0, 0, 0, 0, 0, 0];
        this.width = 0;
        this.height = 0;
        this.opacity = 1;
        this.clipPath = undefined;
    }
}
