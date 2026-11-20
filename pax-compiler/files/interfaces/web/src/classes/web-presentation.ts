import type {PaxChassisWeb} from '../types/pax-chassis-web';

type Tile = {
    canvas: HTMLCanvasElement;
    output: ImageBitmapRenderingContext;
    image: ImageBitmap;
    id: string;
    generation: string;
    content: string;
};

type Revision = {
    tiles: Map<HTMLCanvasElement, Tile>;
    messages: any[];
    payloadBytes: number;
};

// Qualification limits cover all retained native work for a presentation.
// They bound retained work; time spent inside a DOM setter still needs measuring.
const maxNativeMessages = 2048;
const maxPayloadBytes = 4 * 1024 * 1024;
const immediate = new Set(['ImageLoad', 'Navigate', 'SetCursor', 'Screenshot']);

function revision(): Revision {
    return {tiles: new Map(), messages: [], payloadBytes: 0};
}

function current(tile: Tile) {
    const canvas = tile.canvas;
    return canvas.isConnected && canvas.id === tile.id
        && canvas.dataset.surfaceGeneration === tile.generation
        && canvas.dataset.surfaceContentRevision === tile.content;
}

function display(tile: Tile) {
    if (tile.canvas.width !== tile.image.width) tile.canvas.width = tile.image.width;
    if (tile.canvas.height !== tile.image.height) tile.canvas.height = tile.image.height;
    tile.output.transferFromImageBitmap(tile.image);
    tile.canvas.style.removeProperty('visibility');
}

// The ordinary renderer determines which tiles need new pixels. An image is
// transferred out of its private swap chain at submission, before another turn
// can overwrite it. Sealing detaches the ordered native prefix and input view.
export class WebPresentation {
    private working = revision();
    private sealed?: {batch: Revision; fence: bigint; coverage: string};
    private armed = false;
    private disposed = false;
    private failed = false;

    get stopped() { return this.failed || this.disposed; }

    constructor(private root: Element, private chassis: PaxChassisWeb,
        private apply: (messages: any[]) => void, private coverage: () => string | undefined,
        readScroll: () => [number, number, number][]) {
        root.setAttribute('data-pax-private-tiles', '');
        root.addEventListener('pax-tile-submission', this.submitted);
        chassis.set_displayed_scroll_reader(readScroll);
    }

    private submitted = (event: Event) => {
        if (this.stopped) return;
        const canvas = event.target;
        if (!(canvas instanceof HTMLCanvasElement) || canvas.closest('[data-pax-private-tiles]') !== this.root) return;
        const image = (event as CustomEvent<ImageBitmap>).detail;
        const output = canvas.getContext('bitmaprenderer');
        if (!output) throw new Error('Missing bitmap presentation context');
        const tile: Tile = {canvas, image, output, id: canvas.id,
            generation: canvas.dataset.surfaceGeneration!, content: canvas.dataset.surfaceContentRevision!};
        event.preventDefault();
        if (!this.armed) {
            display(tile);
            return;
        }
        this.working.tiles.get(canvas)?.image.close();
        this.working.tiles.set(canvas, tile);
    };

    receive(messages: any[]) {
        if (this.stopped) return;
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
            // The chassis hands off fresh, exclusively owned JS values. UTF-16
            // length accounts for strings without retaining a second wire copy.
            const bytes = 2 * JSON.stringify(message).length;
            if (this.working.messages.length + (this.sealed?.batch.messages.length ?? 0) >= maxNativeMessages
                || this.working.payloadBytes + (this.sealed?.batch.payloadBytes ?? 0) + bytes > maxPayloadBytes) {
                this.failed = true;
                this.clearBatches();
                throw new Error('Web presentation qualification exceeded its native batch budget');
            }
            this.working.messages.push(message);
            this.working.payloadBytes += bytes;
        }
    }

    get awaitingFence() { return !!this.sealed; }

    // During bootstrap native work still applies directly. Once armed, admit
    // another animation sample only after this revision reaches the display.
    settled(): boolean {
        if (this.sealed || this.stopped) return false;
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
        this.sealed = {batch: this.working, fence, coverage};
        this.working = revision();
        return true;
    }

    publish() {
        const sealed = this.sealed;
        if (!sealed || !this.chassis.web_submission_complete(sealed.fence)) return false;
        // Surface reassignment can occur during browser-owned scrolling. Carry
        // the native prefix forward and let the renderer supply replacement
        // coverage. A retired tile can never write into a newer assignment.
        if (this.coverage() !== sealed.coverage || [...sealed.batch.tiles.values()].some(tile => !current(tile))) {
            this.working.messages = sealed.batch.messages.concat(this.working.messages);
            this.working.payloadBytes += sealed.batch.payloadBytes;
            for (const [canvas, tile] of sealed.batch.tiles) {
                if (current(tile) && !this.working.tiles.has(canvas)) this.working.tiles.set(canvas, tile);
                else tile.image.close();
            }
            this.sealed = undefined;
            return false;
        }
        this.apply(sealed.batch.messages);
        for (const tile of sealed.batch.tiles.values()) display(tile);
        this.chassis.publish_displayed_input();
        this.sealed = undefined;
        return true;
    }

    dispose() {
        this.disposed = true;
        this.root.removeEventListener('pax-tile-submission', this.submitted);
        this.root.removeAttribute('data-pax-private-tiles');
        this.clearBatches();
    }

    private clearBatches() {
        for (const batch of [this.working, this.sealed?.batch]) {
            if (batch) for (const tile of batch.tiles.values()) tile.image.close();
        }
        this.sealed = undefined;
        this.working = revision();
    }
}
