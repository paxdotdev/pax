export type RouteChangePayload = {
    path_segments: string[];
    query: Record<string, string[]>;
    fragment: string | null;
};

type RouteHistoryState = {
    pax_route: RouteChangePayload;
};

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
    window.history.replaceState(routeHistoryState(url), "", url);
}

export function pushRouteHistoryState(url: URL) {
    if (url.href === window.location.href) {
        replaceCurrentRouteHistoryState(url);
        return;
    }

    window.history.pushState(routeHistoryState(url), "", url);
}
