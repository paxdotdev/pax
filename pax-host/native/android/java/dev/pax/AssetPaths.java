package dev.pax;

import java.net.URI;

/** Shared URL rules for packaged and downloaded Android resources. */
final class AssetPaths {
    static String bundled(String source) {
        String path = source.startsWith("/assets/") ? source.substring(1) : source;
        if (path.startsWith("assets/")) path = path.substring(7);
        if (path.isEmpty() || path.startsWith("/") || path.indexOf(':') >= 0 ||
                path.indexOf('\\') >= 0 || path.indexOf('\0') >= 0)
            throw new IllegalArgumentException("Invalid bundled asset: " + source);
        for (String part : path.split("/", -1))
            if (part.isEmpty() || part.equals(".") || part.equals(".."))
                throw new IllegalArgumentException("Invalid bundled asset: " + source);
        return path;
    }

    static URI remote(String source) {
        URI uri = URI.create(source);
        if (!"https".equalsIgnoreCase(uri.getScheme()) || uri.getHost() == null ||
                uri.getRawUserInfo() != null)
            throw new IllegalArgumentException("Remote Android assets require an HTTPS URL: " + source);
        return uri;
    }

    static boolean isRemote(String source) {
        return source.matches("(?i)^[a-z][a-z0-9+.-]*:.*") || source.startsWith("//");
    }
}
