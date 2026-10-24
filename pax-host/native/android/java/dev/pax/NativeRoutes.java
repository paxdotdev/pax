package dev.pax;

import java.net.URI;

/** Resolve in-app locations while preserving path, query and fragment semantics. */
final class NativeRoutes {
    static String resolve(String current, String reference) {
        URI base = URI.create(current), next = URI.create(reference);
        if (!next.isAbsolute() && next.getRawAuthority() == null && next.getRawPath().isEmpty()) {
            // URI.resolve treats an empty relative path as a sibling directory
            // for query-only and empty references. These keep the current path.
            // Use raw components so encoded delimiters are not decoded twice.
            String query = next.getRawQuery() == null ? base.getRawQuery() : next.getRawQuery();
            return base.getRawPath() + (query == null ? "" : "?" + query)
                    + (next.getRawFragment() == null ? "" : "#" + next.getRawFragment());
        }
        return base.resolve(next).normalize().toString();
    }
}
