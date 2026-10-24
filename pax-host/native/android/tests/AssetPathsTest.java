package dev.pax;

/** Asset paths must agree with web-authored examples without escaping a root. */
public final class AssetPathsTest {
    public static void main(String[] args) {
        for (String path : new String[]{"/assets/fonts/Manrope.ttf", "assets/fonts/Manrope.ttf", "fonts/Manrope.ttf"})
            check(AssetPaths.bundled(path).equals("fonts/Manrope.ttf"), path);
        for (String path : new String[]{"", "/font.ttf", "assets/../font.ttf", "assets//font.ttf", "./font.ttf", "C:\\font.ttf", "fonts/./font.ttf"})
            rejects(() -> AssetPaths.bundled(path), path);
        check(AssetPaths.remote("https://fonts.googleapis.com/css2?family=IBM+Plex+Mono").getHost().equals("fonts.googleapis.com"), "remote font");
        check(AssetPaths.isRemote("HTTPS://example.org/image.png"), "scheme case");
        check(AssetPaths.isRemote("//example.org/image.png"), "protocol relative");
        for (String path : new String[]{"http://example.org/image.png", "file:///secret", "https://user:pass@example.org/image.png", "https:///image.png", "//example.org/image.png"})
            rejects(() -> AssetPaths.remote(path), path);
        System.out.println("Android asset path checks passed");
    }
    static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }
    static void rejects(Runnable action, String message) {
        try { action.run(); } catch (IllegalArgumentException expected) { return; }
        throw new AssertionError("Accepted invalid path: " + message);
    }
}
