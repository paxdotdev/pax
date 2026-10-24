package dev.pax;

public final class FrameStatsTest {
    public static void main(String[] args) {
        FrameStats stats = new FrameStats();
        check(stats.summary()[0] == 0, "empty");
        stats.add(1000000000);
        for (int i=0;i<240;i++) stats.add((i+1)*1000000L);
        double[] result=stats.summary();
        check(result[0]==120 && result[1]==228 && result[2]==238 && result[3]==240 && result[4]==241,
            "expired spike must leave the bounded timing window");
        System.out.println("Android frame timing checks passed");
    }
    static void check(boolean condition, String message) { if (!condition) throw new AssertionError(message); }
}
