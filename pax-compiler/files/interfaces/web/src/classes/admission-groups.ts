import type {PaxChassisWeb} from '../types/pax-chassis-web';

export interface GroupResources {
    admissionNode(id: number): HTMLElement | undefined;
    firstContentState(id: number): 'ready' | 'text' | 'image' | 'missing';
}

/** A group can measure while hidden. Its native visibility changes in the
 * same publication as its canvas image and the captured hit geometry. */
export class AdmissionGroups {
    private parked = new Map<number, {node: HTMLElement; inert: boolean}>();

    constructor(private chassis: PaxChassisWeb, private resources: GroupResources,
        private limitOpening = false) {}

    advance(batches: any[][]) {
        const candidates = this.chassis.ready_visible_groups();
        if (!candidates.length) return;
        const updating = new Set<number>();
        for (const batch of batches) for (const m of batch) {
            const text = m.TextUpdate;
            if (m.TextCreate) updating.add(m.TextCreate.id);
            if (m.NativeImageCreate) updating.add(m.NativeImageCreate.id);
            if (text && (text.content != null || text.style != null ||
                text.style_link != null || text.measure_generation != null)) updating.add(text.id);
            if (m.NativeImageUpdate?.url != null) updating.add(m.NativeImageUpdate.id);
        }
        const deadline = this.limitOpening ? performance.now() + 0.5 : Infinity;
        for (const [id, epoch, resources] of candidates) {
            const ready = resources.every(n => {
                if (updating.has(n)) return false;
                const state = this.resources.firstContentState(n);
                if (state === 'missing') {
                    throw new Error(`Pax group ${id} (publication ${epoch}) is waiting for native content ${n} with no element or pending create`);
                }
                return state === 'ready';
            });
            if (ready) {
                const opened = this.chassis.open_visible_group(id, epoch);
                if (opened && this.limitOpening && performance.now() >= deadline) break;
            }
        }
    }

    snapshot(messages: any[]): Uint32Array {
        const ids = new Set(this.parked.keys());
        for (const message of messages) {
            const patch = message[Object.keys(message)[0]];
            if (typeof patch?.id === 'number') ids.add(patch.id);
        }
        return this.chassis.hidden_native_group_members(Uint32Array.from(ids));
    }

    publish(hidden: Iterable<number>) {
        const pending = new Set(hidden);
        for (const [id, state] of this.parked) {
            if (pending.has(id) && this.resources.admissionNode(id) === state.node) continue;
            state.node.removeAttribute('data-pax-group-parked');
            if (!state.inert) state.node.inert = false;
            this.parked.delete(id);
        }
        for (const id of pending) {
            const node = this.resources.admissionNode(id);
            if (!node || this.parked.has(id)) continue;
            this.parked.set(id, {node, inert: node.inert});
            node.setAttribute('data-pax-group-parked', '');
            node.inert = true;
        }
    }

}
