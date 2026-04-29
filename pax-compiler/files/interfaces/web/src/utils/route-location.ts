export type RouteChangePayload = {
    path_segments: string[];
    query: Record<string, string[]>;
    fragment: string | null;
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
