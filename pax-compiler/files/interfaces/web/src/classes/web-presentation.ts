import type {PaxChassisWeb} from '../types/pax-chassis-web';
import type {AdmissionGroups} from './admission-groups';

type Tile = {
    canvas: HTMLCanvasElement;
    image: ImageBitmap;
    id: string;
    generation: string;
    content: string;
};

type Revision = {
    tiles: Map<HTMLCanvasElement, Tile>;
    messages: any[];
    scrollObservations: Map<string, any>;
};

// One tick can replace an arbitrarily large branch. Above this watermark
// pace further feedback against GPU progress before calling its producer.
const feedbackMessageWatermark = 2048;
const immediate = new Set(['ImageLoad', 'Navigate', 'SetCursor', 'Screenshot']);

function revision(): Revision {
    return {tiles: new Map(), messages: [], scrollObservations: new Map()};
}

// Only the trailing run of observations can be replaced. Everything before
// the last ordering barrier is ready to pass directly to the native pool.
function flushObservations(batch: Revision) {
    for (const message of batch.scrollObservations.values()) batch.messages.push(message);
    batch.scrollObservations.clear();
}

// These two producers report sampled presentation state. Authored scroll
// commands and structural patches must retain their order, including a move
// to the root (an explicitly null parent_frame).
const observationFields = new Set(['id', 'subtree_depth', 'presentation_scroll_x', 'presentation_scroll_y',
    'presentation_projection', 'presented_bounds', 'presented_clip_bounds']);
function scrollObservationKey(patch: any): string | undefined {
    if (!patch || patch.id == null || 'parent_frame' in patch || patch.subtree_depth !== 0) return;
    if (Object.keys(patch).some(k => patch[k] != null && !observationFields.has(k))) return;
    if (patch.presented_bounds && patch.presented_clip_bounds && patch.presentation_projection
        && patch.presentation_scroll_x == null && patch.presentation_scroll_y == null) return 'geometry/' + patch.id;
    if ((patch.presentation_scroll_x != null || patch.presentation_scroll_y != null)
        && patch.presentation_projection == null
        && patch.presented_bounds == null && patch.presented_clip_bounds == null) return 'offset/' + patch.id;
}

function current(tile: Tile) {
    const canvas = tile.canvas;
    return canvas.isConnected && canvas.id === tile.id
        && canvas.dataset.surfaceGeneration === tile.generation
        && canvas.dataset.surfaceContentRevision === tile.content;
}

type DisplayedTile = {
    element: HTMLCanvasElement;
    output: ImageBitmapRenderingContext;
    content?: string;
};

// The ordinary renderer determines which tiles need new pixels. An image is
// transferred out of its private swap chain at submission, before another turn
// can overwrite it. Sealing detaches the ordered native prefix and input view.
export class WebPresentation {
    private working = revision();
    private sealed?: {batch: Revision; retained: boolean; fence?: bigint; coverage: string; hidden?: Uint32Array};
    private armed = false;
    private disposed = false;
    private fronts = new Map<HTMLCanvasElement, DisplayedTile>();
    private retired = new Set<DisplayedTile>();

    // The GPU renders offscreen. Keep the displayed bitmap independent from
    // its recyclable submission endpoint so a layer can change owners while
    // the browser is still showing the previous scene.
    private display(tile: Tile) {
        const source = tile.canvas;
        let front = this.fronts.get(source);
        if (!front) {
            const element = source.ownerDocument.createElement('canvas');
            const output = element.getContext('bitmaprenderer');
            if (!output) throw new Error('Missing bitmap presentation context');
            element.className = source.className;
            element.dataset.paxDisplayedTile = source.id;
            element.dataset.layerId = source.dataset.layerId;
            element.dataset.tileKey = source.dataset.tileKey;
            element.style.position = 'absolute';
            element.style.pointerEvents = 'none';
            element.style.zIndex = source.style.zIndex;
            source.after(element);
            // A native delete can disconnect the host before its lease retires.
            source.addEventListener('pax-tile-retirement', this.retire, {once: true});
            front = {element, output};
            this.fronts.set(source, front);
        }
        const {element, output} = front;
        if (front.content !== tile.content) {
            const {tileOriginX, tileOriginY, logicalWidth, logicalHeight} = source.dataset;
            element.style.left = tileOriginX + 'px';
            element.style.top = tileOriginY + 'px';
            element.style.width = logicalWidth + 'px';
            element.style.height = logicalHeight + 'px';
            front.content = tile.content;
        }
        if (element.width !== tile.image.width) element.width = tile.image.width;
        if (element.height !== tile.image.height) element.height = tile.image.height;
        output.transferFromImageBitmap(tile.image);
    }

    private retire = (event: Event) => {
        const source = event.target as HTMLCanvasElement;
        const front = this.fronts.get(source);
        if (!front) return;
        this.fronts.delete(source);
        if (this.armed) this.retired.add(front);
        else front.element.remove();
    };

    constructor(private root: Element, private chassis: PaxChassisWeb,
        private apply: (messages: any[]) => void, private coverage: () => string | undefined,
        readScroll: () => [number, number, number][], private groups?: AdmissionGroups) {
        root.setAttribute('data-pax-private-tiles', '');
        root.addEventListener('pax-tile-submission', this.submitted);
        chassis.set_displayed_scroll_reader(readScroll);
    }

    private submitted = (event: Event) => {
        if (this.disposed) return;
        const canvas = event.target;
        if (!(canvas instanceof HTMLCanvasElement) || canvas.closest('[data-pax-private-tiles]') !== this.root) return;
        const image = (event as CustomEvent<ImageBitmap>).detail;
        const tile: Tile = {canvas, image, id: canvas.id,
            generation: canvas.dataset.surfaceGeneration!, content: canvas.dataset.surfaceContentRevision!};
        event.preventDefault();
        if (!this.armed) {
            this.display(tile);
            return;
        }
        const batch = this.sealed && this.sealed.fence === undefined ? this.sealed.batch : this.working;
        batch.tiles.get(canvas)?.image.close();
        batch.tiles.set(canvas, tile);
    };

    receive(messages: any[]) {
        if (this.disposed) return;
        if (!this.armed) {
            this.apply(messages);
            return;
        }
        for (const message of messages) {
            if (immediate.has(Object.keys(message)[0])) {
                // Resource requests can unblock a future frame. Navigation and
                // cursor changes belong to the originating interaction.
                this.apply([message]);
                continue;
            }
            const key = scrollObservationKey(message.ScrollerUpdate);
            const observations = this.working.scrollObservations;
            if (key === undefined) {
                if (observations.size) flushObservations(this.working);
                this.working.messages.push(message);
                continue;
            }
            const previous = observations.get(key)?.ScrollerUpdate;
            if (previous) {
                if (previous.presentation_scroll_x != null) message.ScrollerUpdate.presentation_scroll_x ??= previous.presentation_scroll_x;
                if (previous.presentation_scroll_y != null) message.ScrollerUpdate.presentation_scroll_y ??= previous.presentation_scroll_y;
                observations.delete(key);
            }
            observations.set(key, message);
        }
    }

    canSettleFeedback() {
        if (this.working.messages.length + this.working.scrollObservations.size < feedbackMessageWatermark) return true;
        const fence = this.chassis.web_submission_fence();
        return fence !== undefined && this.chassis.web_submission_complete(fence);
    }

    get hasPendingFrame() { return !!this.sealed; }
    get hasRetainedFrame() { return this.sealed?.retained ?? false; }
    get awaitingFence() { return this.sealed?.fence !== undefined; }
    get pendingFence() { return this.sealed?.fence; }
    get tracksGroups() { return !!this.groups; }

    advanceGroups() {
        this.groups?.advance(this.sealed ? [this.working.messages, this.sealed.batch.messages] : [this.working.messages]);
    }

    // Detach native messages and hit geometry while the retained tile scenes
    // still describe that pose. New input remains live during GPU submission.
    retain(): boolean {
        if (!this.armed || this.sealed || this.disposed) return false;
        const coverage = this.coverage();
        if (coverage === undefined || !this.chassis.retain_visible_frame()) return false;
        this.seal(coverage);
        return true;
    }

    private seal(coverage: string, fence?: bigint) {
        this.sealed = {batch: this.working, retained: fence === undefined, fence, coverage,
            hidden: this.groups?.snapshot(this.working.messages)};
        this.working = revision();
    }

    // During bootstrap native work still applies directly. Once armed, admit
    // another animation sample only after this revision reaches the display.
    settled(): boolean {
        if (this.sealed || this.disposed) return false;
        const coverage = this.coverage();
        if (coverage === undefined) return !this.armed;
        if (!this.armed) {
            if (this.chassis.prepare_displayed_input()) {
                this.chassis.publish_displayed_input();
                this.armed = true;
            }
            return true;
        }
        const fence = this.chassis.web_submission_fence();
        if (fence === undefined || !this.chassis.prepare_displayed_input()) return false;
        this.seal(coverage, fence);
        return true;
    }

    publish(submitRetained?: () => boolean) {
        const sealed = this.sealed;
        if (!sealed) return false;
        if (sealed.fence === undefined && this.coverage() === sealed.coverage) {
            if (!submitRetained?.()) return false;
            sealed.fence = this.chassis.web_submission_fence();
        }
        if (sealed.fence !== undefined && !this.chassis.web_submission_complete(sealed.fence)) return false;
        // Surface reassignment can occur during browser-owned scrolling. Carry
        // the native prefix forward and let the renderer supply replacement
        // coverage. A retired tile can never write into a newer assignment.
        if (sealed.fence === undefined || this.coverage() !== sealed.coverage || [...sealed.batch.tiles.values()].some(tile => !current(tile))) {
            const later = this.working;
            this.sealed = undefined;
            this.working = sealed.batch;
            flushObservations(later);
            this.receive(later.messages);
            for (const [canvas, tile] of sealed.batch.tiles) {
                if (!current(tile) || later.tiles.has(canvas)) {
                    tile.image.close();
                    sealed.batch.tiles.delete(canvas);
                }
            }
            for (const [canvas, tile] of later.tiles) this.working.tiles.set(canvas, tile);
            return false;
        }
        flushObservations(sealed.batch);
        this.apply(sealed.batch.messages);
        if (sealed.hidden) this.groups?.publish(sealed.hidden);
        // Structural native work can retire an island and release its canvas.
        for (const tile of sealed.batch.tiles.values()) {
            if (current(tile)) this.display(tile);
            else tile.image.close();
        }
        for (const front of this.retired) front.element.remove();
        this.retired.clear();
        this.chassis.publish_displayed_input();
        this.sealed = undefined;
        return true;
    }

    dispose() {
        this.disposed = true;
        this.groups?.publish([]);
        this.root.removeEventListener('pax-tile-submission', this.submitted);
        for (const [source, front] of this.fronts) {
            source.removeEventListener('pax-tile-retirement', this.retire);
            front.element.remove();
        }
        for (const front of this.retired) front.element.remove();
        this.fronts.clear();
        this.retired.clear();
        this.root.removeAttribute('data-pax-private-tiles');
        for (const batch of [this.working, this.sealed?.batch]) {
            if (batch) for (const tile of batch.tiles.values()) tile.image.close();
        }
        this.sealed = undefined;
        this.working = revision();
    }
}
