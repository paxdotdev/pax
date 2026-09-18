export type RouteChangePayload = {
    path_segments: string[];
    query: Record<string, string[]>;
    fragment: string | null;
};

type RouteHistoryState = {
    pax_route: RouteChangePayload;
};

// A host can opt into query-backed routing without claiming its own URL space.
// Keep the physical entry document stable so nested/versioned embeds can reload.
export function browserRouteLocation(browserUrl = new URL(window.location.href)): URL {
    const route = browserUrl.searchParams.get("pax_route");
    if (route == null) return browserUrl;
    const url = new URL(browserUrl.origin);
    if (!route.startsWith("/") || route.startsWith("//")) return url;
    try {
        const candidate = new URL(route, url);
        return candidate.origin === url.origin ? candidate : url;
    } catch {
        return url;
    }
}

function browserHistoryUrl(routeUrl: URL): URL {
    const browserUrl = new URL(window.location.href);
    if (!browserUrl.searchParams.has("pax_route")) return routeUrl;
    browserUrl.searchParams.set("pax_route", routeUrl.pathname + routeUrl.search + routeUrl.hash);
    return browserUrl;
}

function decodeUriComponentSafely(value: string): string {
    try {
        return decodeURIComponent(value);
    } catch {
        return value;
    }
}

export function serializeRouteLocation(url: URL): RouteChangePayload {
    let query: Record<string, string[]> = {};
    url.searchParams.forEach((value, key) => {
        if (!query[key]) {
            query[key] = [];
        }
        query[key].push(value);
    });

    return {
        path_segments: url.pathname
            .split("/")
            .filter((segment) => segment.length > 0)
            .map(decodeUriComponentSafely),
        query,
        fragment: url.hash ? decodeUriComponentSafely(url.hash.slice(1)) : null,
    };
}

function routeHistoryState(url: URL): RouteHistoryState {
    return {
        pax_route: serializeRouteLocation(url),
    };
}

export function replaceCurrentRouteHistoryState(url: URL) {
    window.history.replaceState(routeHistoryState(url), "", browserHistoryUrl(url));
}

export function pushRouteHistoryState(url: URL) {
    if (url.href === browserRouteLocation().href) {
        replaceCurrentRouteHistoryState(url);
        return;
    }

    window.history.pushState(routeHistoryState(url), "", browserHistoryUrl(url));
}
