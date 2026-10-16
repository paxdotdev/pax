// Constant-time writes; chronological materialization happens only on inspection.
export class TraceRing<T> {
    private entries: T[] = [];
    private next = 0;
    overwritten = 0;
    constructor(private capacity: number) {}
    push(value: T) {
        if (this.entries.length === this.capacity) this.overwritten++;
        this.entries[this.next] = value;
        this.next = (this.next + 1) % this.capacity;
    }
    snapshot(): T[] {
        return this.entries.length < this.capacity ? this.entries.slice()
            : this.entries.slice(this.next).concat(this.entries.slice(0, this.next));
    }
    clear() { this.entries = []; this.next = 0; this.overwritten = 0; }
}
