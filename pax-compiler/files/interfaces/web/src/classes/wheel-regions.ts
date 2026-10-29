import {attachScopedWheelListener} from '../events/listeners';

export type WheelRegion = {
    id: number;
    parent_frame: number | null;
    z_index: number;
    transform: number[];
    size_x: number;
    size_y: number;
    native: boolean;
};

export type WheelRegionsPatch = {upsert: WheelRegion[]; remove: number[]};

type InstalledRegion = {element: HTMLElement; proxy: boolean; dispose: () => void};

/** Input-only leaves: no paint, canvas leases, focus targets, or touch policy. */
export class WheelRegions {
    private installed = new Map<number, InstalledRegion>();

    update(patch: WheelRegionsPatch, nativeLeaf: (id: number) => HTMLElement | undefined,
           place: (element: HTMLElement, parent: number | undefined) => void) {
        for (const id of patch.remove) this.remove(id);
        for (const region of patch.upsert) {
            const native = region.native ? nativeLeaf(region.id) : undefined;
            // Images deliberately have pointer-events:none; their vector-like
            // hit target needs a proxy, unlike selectable text and controls.
            const target = native?.style.pointerEvents === 'none' ? undefined : native;
            let entry = this.installed.get(region.id);
            if (entry && (entry.proxy !== !target || (target && entry.element !== target))) {
                this.remove(region.id);
                entry = undefined;
            }
            if (!entry) {
                const element = target ?? document.createElement('div');
                if (!target) {
                    element.dataset.paxWheelRegion = String(region.id);
                    element.setAttribute('aria-hidden', 'true');
                    Object.assign(element.style, {
                        position: 'absolute', left: '0', top: '0',
                        transformOrigin: '0 0', pointerEvents: 'auto',
                    });
                }
                entry = {element, proxy: !target, dispose: attachScopedWheelListener(element)};
                this.installed.set(region.id, entry);
            }
            if (entry.proxy) {
                Object.assign(entry.element.style, {
                    width: `${region.size_x}px`, height: `${region.size_y}px`,
                    transform: `matrix(${region.transform.join(',')})`,
                    zIndex: String(region.z_index),
                });
                // Same Frame/Scroller clip ancestry and unscrolled coordinates
                // as native leaves; the browser moves it without per-scroll work.
                place(entry.element, region.parent_frame ?? undefined);
            }
        }
    }

    private remove(id: number) {
        const entry = this.installed.get(id);
        if (!entry) return;
        entry.dispose();
        if (entry.proxy) entry.element.remove();
        this.installed.delete(id);
    }

    dispose() {
        for (const id of this.installed.keys()) this.remove(id);
    }
}
