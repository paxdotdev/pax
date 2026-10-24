package dev.pax;

/** Query-only navigation must update the current screen's state. */
public final class NativeRoutesTest {
    public static void main(String[] args) {
        String base = "/films/paxflix?page=1#hero";
        check(base, "?page=2", "/films/paxflix?page=2");
        check(base, "?", "/films/paxflix?");
        check(base, "", "/films/paxflix?page=1");
        check(base, "#details", "/films/paxflix?page=1#details");
        check(base, "../notes", "/notes");
        check(base, "next?sort=new#top", "/films/next?sort=new#top");
        check(base, "/", "/");
        check("/films/a%2Fb?tag=a%26b#old", "#x%20y", "/films/a%2Fb?tag=a%26b#x%20y");
        check("/films/a%2Fb?tag=a%26b#old", "?tag=c%26d", "/films/a%2Fb?tag=c%26d");
        System.out.println("Android native route resolution checks passed");
    }
    static void check(String base, String reference, String expected) {
        String actual = NativeRoutes.resolve(base, reference);
        if (!actual.equals(expected)) throw new AssertionError(reference + ": " + actual + " != " + expected);
    }
}
