export type NativeMessageTiming = {type: string; count: number; totalMs: number; maxMs: number};

// One opt-in batch, not one performance entry per message. Values and DOM
// nodes never enter the report; deferred layout/paint and async work are not
// included in these synchronous handler durations.
export class NativeMessageProfile {
    private types = new Map<string, NativeMessageTiming>();

    record(type: string, duration: number) {
        let timing = this.types.get(type);
        if (!timing) {
            timing = {type, count: 0, totalMs: 0, maxMs: 0};
            this.types.set(type, timing);
        }
        timing.count++;
        timing.totalMs += duration;
        timing.maxMs = Math.max(timing.maxMs, duration);
    }

    snapshot(): NativeMessageTiming[] {
        return [...this.types.values()].map(t => ({...t})).sort((a,b) => b.totalMs-a.totalMs);
    }
}
