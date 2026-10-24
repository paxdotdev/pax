package dev.pax;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Typeface;
import android.util.Log;
import java.io.*;
import java.net.*;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.*;
import java.util.concurrent.*;
import java.util.regex.*;

/** Session-owned I/O. Workers retain only the application context, never Views. */
final class PaxAssets implements AutoCloseable {
    static final int MAX_DOWNLOAD = 16 * 1024 * 1024;
    static final long MAX_PIXELS = 16L * 1024 * 1024;
    static final long DISK_BUDGET = 64L * 1024 * 1024;
    final Context context;
    final String[] roots;
    final File cache;
    final ThreadPoolExecutor worker = new ThreadPoolExecutor(1, 1, 0, TimeUnit.SECONDS,
            new ArrayBlockingQueue<Runnable>(256));
    final Set<String> loading = new HashSet<>(), loadedImages = new HashSet<>(), failed = new HashSet<>();
    final Map<String, Typeface> fonts = new HashMap<>();
    final Map<String, NativeImage> nativeImages = new HashMap<>();
    // At most one decoded image waits for a UI upload, even when rendering is paused.
    final Semaphore decodedSlot = new Semaphore(1);
    final ArrayDeque<Image> readyImages = new ArrayDeque<>();
    volatile boolean closed;
    int fontRevision;
    int nativeImageRevision;
    long nativeImageBytes;
    volatile long decodedBytes;
    long uploadedBytes;

    static final class Image {
        final String path;
        final int width, height;
        final ByteBuffer rgba;
        Image(String path, int width, int height, ByteBuffer rgba) {
            this.path = path; this.width = width; this.height = height; this.rgba = rgba;
        }
    }

    // UI-thread leases share one decoded bitmap per active URL. Workers hold
    // these records, never an Activity or View; the last lease cancels delivery.
    static final class NativeImage {
        final String path;
        int references = 1;
        volatile boolean cancelled;
        Bitmap bitmap;
        NativeImage(String path) { this.path = path; }
    }

    PaxAssets(Context context, String[] roots) {
        this.context = context.getApplicationContext();
        this.roots = roots.clone();
        cache = new File(this.context.getCacheDir(), "pax-assets");
        cache.mkdirs();
    }

    InputStream bundled(String source) throws IOException {
        String relative = AssetPaths.bundled(source);
        for (String root : roots) {
            try { return context.getAssets().open(root + "/" + relative); }
            catch (FileNotFoundException missing) { /* Try the next authored root. */ }
        }
        throw new FileNotFoundException("Bundled asset not found: " + source);
    }

    String bundledPath(String source) throws IOException {
        String relative = AssetPaths.bundled(source);
        for (String root : roots) {
            String path = root + "/" + relative;
            try (InputStream ignored = context.getAssets().open(path)) { return path; }
            catch (FileNotFoundException missing) { /* Try the next authored root. */ }
        }
        throw new FileNotFoundException("Bundled font not found: " + source);
    }

    static String hash(String value) throws Exception {
        StringBuilder out = new StringBuilder();
        for (byte b : MessageDigest.getInstance("SHA-256").digest(value.getBytes(StandardCharsets.UTF_8)))
            out.append(String.format(Locale.ROOT, "%02x", b & 255));
        return out.toString();
    }

    File download(String source) throws Exception {
        URI uri = AssetPaths.remote(source);
        File file = new File(cache, hash(source));
        if (file.isFile()) { file.setLastModified(System.currentTimeMillis()); return file; }
        for (int redirects = 0; redirects <= 5; redirects++) {
            if (closed || Thread.currentThread().isInterrupted()) throw new InterruptedIOException();
            HttpURLConnection connection = (HttpURLConnection) uri.toURL().openConnection();
            connection.setInstanceFollowRedirects(false);
            connection.setConnectTimeout(10000); connection.setReadTimeout(10000);
            // A native font consumer needs TrueType/OpenType, not browser WOFF.
            connection.setRequestProperty("User-Agent", "Pax/1.0 Android");
            File temporary = new File(cache, file.getName() + ".partial");
            try {
                int status = connection.getResponseCode();
                if (status >= 300 && status < 400) {
                    String location = connection.getHeaderField("Location");
                    if (location == null) throw new IOException("Redirect without Location");
                    uri = AssetPaths.remote(uri.resolve(location).toString()); continue;
                }
                if (status != 200) throw new IOException("HTTP " + status + " loading " + source);
                if (connection.getContentLengthLong() > MAX_DOWNLOAD) throw new IOException("Asset exceeds 16 MiB download limit");
                try (InputStream input = connection.getInputStream(); OutputStream output = new FileOutputStream(temporary)) {
                    byte[] chunk = new byte[16384]; int total = 0, count;
                    while ((count = input.read(chunk)) != -1) {
                        total += count;
                        if (total > MAX_DOWNLOAD) throw new IOException("Asset exceeds 16 MiB download limit");
                        if (closed || Thread.currentThread().isInterrupted()) throw new InterruptedIOException();
                        output.write(chunk, 0, count);
                    }
                }
                if (!temporary.renameTo(file)) throw new IOException("Cannot cache downloaded asset");
                trimCache(); return file;
            } finally { temporary.delete(); connection.disconnect(); }
        }
        throw new IOException("Too many asset redirects");
    }

    void trimCache() {
        File[] files = cache.listFiles((dir, name) -> !name.endsWith(".partial"));
        if (files == null) return;
        Arrays.sort(files, Comparator.comparingLong(File::lastModified));
        long total = 0; for (File file : files) total += file.length();
        for (File file : files) if (total > DISK_BUDGET) { long size = file.length(); if (file.delete()) total -= size; }
    }

    InputStream open(String source) throws Exception {
        return AssetPaths.isRemote(source) ? new FileInputStream(download(source)) : bundled(source);
    }

    Bitmap decodeBitmap(String path) throws Exception {
        BitmapFactory.Options options = new BitmapFactory.Options(); options.inJustDecodeBounds = true;
        try (InputStream input = open(path)) { BitmapFactory.decodeStream(input, null, options); }
        if (options.outWidth <= 0 || options.outHeight <= 0 || options.outWidth > 8192 || options.outHeight > 8192 ||
                (long) options.outWidth * options.outHeight > MAX_PIXELS)
            throw new IOException("Image exceeds Android 8192-axis / 16-megapixel decode limit: " + path);
        options.inJustDecodeBounds = false; options.inPreferredConfig = Bitmap.Config.ARGB_8888; options.inScaled = false;
        Bitmap bitmap;
        try (InputStream input = open(path)) { bitmap = BitmapFactory.decodeStream(input, null, options); }
        if (bitmap == null) throw new IOException("Cannot decode " + path);
        return bitmap;
    }

    NativeImage acquireNativeImage(String path) {
        if (closed || path.isEmpty()) return null;
        NativeImage existing = nativeImages.get(path);
        if (existing != null) { existing.references++; return existing; }
        NativeImage image = new NativeImage(path); nativeImages.put(path, image);
        submit("native-image:" + path, () -> {
            if (image.cancelled) return;
            decodedSlot.acquire();
            Bitmap bitmap = null; boolean posted = false;
            try {
                if (closed || image.cancelled) return;
                bitmap = decodeBitmap(path);
                Bitmap decoded = bitmap;
                decodedBytes = decoded.getAllocationByteCount();
                posted = PaxActivity.UI.post(() -> {
                    try {
                        if (closed || image.cancelled) decoded.recycle();
                        else {
                            image.bitmap = decoded;
                            nativeImageBytes += decoded.getAllocationByteCount();
                            nativeImageRevision++;
                        }
                    } finally { decodedBytes = 0; decodedSlot.release(); }
                });
            } catch (Exception error) {
                if (!image.cancelled) throw error;
            } finally {
                if (!posted) {
                    if (bitmap != null) bitmap.recycle();
                    decodedBytes = 0; decodedSlot.release();
                }
            }
        });
        return image;
    }

    void releaseNativeImage(NativeImage image) {
        if (image == null || --image.references != 0) return;
        image.cancelled = true;
        nativeImages.remove(image.path, image);
        if (image.bitmap != null) {
            nativeImageBytes -= image.bitmap.getAllocationByteCount();
            // A submitted Android display list may still use the bitmap. Let
            // its native references expire instead of recycling displayed pixels.
            image.bitmap = null;
        }
    }

    void requestImage(String path) {
        if (closed || loadedImages.contains(path) || !begin("image:" + path)) return;
        submit("image:" + path, () -> {
            decodedSlot.acquire();
            boolean queued = false;
            Bitmap bitmap = null;
            try {
                bitmap = decodeBitmap(path);
                int w = bitmap.getWidth(), h = bitmap.getHeight();
                ByteBuffer rgba = ByteBuffer.allocateDirect(w * h * 4);
                int[] row = new int[w];
                for (int y = 0; y < h; y++) {
                    if (closed || Thread.currentThread().isInterrupted()) throw new InterruptedIOException();
                    bitmap.getPixels(row, 0, w, 0, y, w, 1);
                    for (int pixel : row) { rgba.put((byte)(pixel >> 16)); rgba.put((byte)(pixel >> 8)); rgba.put((byte)pixel); rgba.put((byte)(pixel >>> 24)); }
                }
                rgba.flip();
                synchronized (readyImages) {
                    if (!closed) { readyImages.add(new Image(path, w, h, rgba)); decodedBytes = rgba.capacity(); queued = true; }
                }
            } finally { if (bitmap != null) bitmap.recycle(); if (!queued) decodedSlot.release(); }
        });
    }

    Image takeImage() { synchronized (readyImages) { return readyImages.poll(); } }
    void uploaded(Image image) {
        loadedImages.add(image.path); uploadedBytes += image.rgba.capacity(); decodedBytes = 0; decodedSlot.release();
    }

    Typeface font(String source, String family, int weight, boolean italic) {
        String key = source + "|" + family + "|" + weight + "|" + italic;
        Typeface font = fonts.get(key);
        if (font != null) return font;
        if (!source.isEmpty() && !closed && begin("font:" + key)) submit("font:" + key, () -> {
            Typeface.Builder builder = AssetPaths.isRemote(source)
                    ? new Typeface.Builder(fontFile(source, weight, italic))
                    : new Typeface.Builder(context.getAssets(), bundledPath(source));
            // Weight metadata alone does not select a variable font's glyphs.
            Typeface loaded = builder.setFontVariationSettings("'wght' " + weight + ", 'ital' " + (italic ? 1 : 0))
                    .setWeight(weight).setItalic(italic).build();
            if (loaded == null) throw new IOException("Cannot load font " + source);
            PaxActivity.UI.post(() -> { if (!closed) { fonts.put(key, loaded); fontRevision++; } });
        });
        return SystemFonts.resolve(family, weight, italic);
    }

    boolean fontReady(String source, String family, int weight, boolean italic) {
        String key = source + "|" + family + "|" + weight + "|" + italic;
        return source.isEmpty() || fonts.containsKey(key) || failed.contains("font:" + key);
    }

    File fontFile(String source, int weight, boolean italic) throws Exception {
        File file = download(source);
        byte[] prefix = new byte[64]; int count;
        try (InputStream input = new FileInputStream(file)) { count = input.read(prefix); }
        String head = new String(prefix, 0, Math.max(0, count), StandardCharsets.UTF_8).trim();
        if (!head.startsWith("/*") && !head.startsWith("@font-face")) return file;
        if (file.length() > 256 * 1024) throw new IOException("Font stylesheet exceeds 256 KiB");
        String css = new String(java.nio.file.Files.readAllBytes(file.toPath()), StandardCharsets.UTF_8);
        Matcher faces = Pattern.compile("@font-face\\s*\\{([^}]+)\\}", Pattern.CASE_INSENSITIVE).matcher(css);
        String chosen = null; int distance = Integer.MAX_VALUE;
        while (faces.find()) {
            String face = faces.group(1);
            Matcher location = Pattern.compile("url\\(['\"]?([^)'\"\\s]+)['\"]?\\)", Pattern.CASE_INSENSITIVE).matcher(face);
            Matcher number = Pattern.compile("font-weight\\s*:\\s*(\\d+)", Pattern.CASE_INSENSITIVE).matcher(face);
            boolean slanted = Pattern.compile("font-style\\s*:\\s*(italic|oblique)", Pattern.CASE_INSENSITIVE).matcher(face).find();
            int score = Math.abs((number.find() ? Integer.parseInt(number.group(1)) : 400) - weight) + (slanted == italic ? 0 : 1000);
            if (location.find() && score < distance) { chosen = location.group(1); distance = score; }
        }
        if (chosen == null) throw new IOException("Font stylesheet has no supported font face");
        return download(AssetPaths.remote(source).resolve(chosen).toString());
    }

    boolean begin(String key) { return !failed.contains(key) && loading.add(key); }
    interface Work { void run() throws Exception; }
    void submit(String key, Work work) {
        try {
            worker.execute(() -> {
                try { if (!closed) work.run(); }
                catch (Exception error) { PaxActivity.UI.post(() -> { if (!closed) { failed.add(key); fontRevision++; Log.e("Pax", "Asset load failed: " + key, error); } }); }
                // Keep successful keys deduplicated for the session lifetime.
            });
        } catch (RejectedExecutionException full) { loading.remove(key); throw new IllegalStateException("Android asset queue exceeds 256 requests", full); }
    }

    @Override public void close() {
        closed = true; worker.shutdownNow();
        synchronized (readyImages) { readyImages.clear(); decodedBytes = 0; }
        for (NativeImage image : nativeImages.values()) { image.cancelled = true; image.bitmap = null; }
        nativeImages.clear(); nativeImageBytes = 0;
        fonts.clear();
    }
}
