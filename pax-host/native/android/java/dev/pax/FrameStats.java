package dev.pax;

import java.util.Arrays;

/** Bounded recent timings; collection never allocates in the frame callback. */
final class FrameStats {
    private final long[] nanos = new long[240];
    private int next, count;
    private long total;
    void add(long duration) {
        nanos[next] = Math.max(0, duration);
        next = (next + 1) % nanos.length;
        count = Math.min(count + 1, nanos.length);
        total++;
    }
    double[] summary() {
        if (count == 0) return new double[]{0, 0, 0, 0, total};
        long[] sorted = Arrays.copyOf(nanos, count);
        Arrays.sort(sorted);
        return new double[]{sorted[(count-1)/2]/1e6, sorted[(int)Math.ceil(count*0.95)-1]/1e6,
            sorted[(int)Math.ceil(count*0.99)-1]/1e6, sorted[count-1]/1e6, total};
    }
}
