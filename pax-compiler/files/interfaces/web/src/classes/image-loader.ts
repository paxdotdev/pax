export type DecodedImage = {pixels: Uint8ClampedArray; width: number; height: number};
type PendingImage = {ids: Set<number>; controller: AbortController; done: Promise<void>};
type ImageLoadHooks = {
    loaded(path: string): boolean;
    decode(path: string, signal: AbortSignal): Promise<DecodedImage>;
    complete(path: string, image: DecodedImage, ids: number[]): void;
    ready(ids: number[]): void;
    trace?(event: string, details: Record<string, unknown>): void;
};

// One chassis owns the requests. Tiles do not: retiring a tile must not cancel
// a resource still requested by another mounted Image or another scroller.
export class ImageLoader {
    private pending = new Map<string, PendingImage>();
    private current = new Map<number, string>();
    private disposed = false;

    constructor(private hooks: ImageLoadHooks) {}

    request(path: string, id: number): Promise<void> {
        if (this.disposed) return Promise.resolve();
        const previous = this.current.get(id);
        if (previous != null && previous !== path) this.pending.get(previous)?.ids.delete(id);
        this.current.set(id, path);
        if (this.hooks.loaded(path)) {
            this.current.delete(id);
            this.hooks.ready([id]);
            this.hooks.trace?.('image-decoded-cache-hit', {path, node: id});
            return Promise.resolve();
        }
        const existing = this.pending.get(path);
        if (existing) {
            existing.ids.add(id);
            this.hooks.trace?.('image-request-coalesced', {path, node: id});
            return existing.done;
        }
        const entry: PendingImage = {ids: new Set([id]), controller: new AbortController(), done: Promise.resolve()};
        this.pending.set(path, entry);
        this.hooks.trace?.('image-request', {path, node: id});
        entry.done = Promise.resolve().then(async () => {
            try {
                if (this.disposed) return;
                const image = await this.hooks.decode(path, entry.controller.signal);
                if (this.disposed) return;
                const ids = [...entry.ids].filter(node => this.current.get(node) === path);
                this.hooks.trace?.('image-decoded', {path, consumers: ids.length,
                    bytes: image.pixels.byteLength, width: image.width, height: image.height});
                this.hooks.complete(path, image, ids);
                this.hooks.trace?.('image-cpu-ready', {path, consumers: ids.length});
            } catch (error) {
                if (!this.disposed) {
                    this.hooks.trace?.('image-load-failed', {path, message: String(error)});
                    throw error;
                }
            } finally {
                this.pending.delete(path);
                for (const node of entry.ids) {
                    if (this.current.get(node) === path) this.current.delete(node);
                }
            }
        });
        return entry.done;
    }

    dispose() {
        this.disposed = true;
        for (const entry of this.pending.values()) entry.controller.abort();
        this.pending.clear();
        this.current.clear();
    }
}
