type RouteMetadata = {
    title: string;
    description: string;
    index: boolean;
    social_image: string | null;
    social_image_alt: string | null;
};

type RouteCatalogEntry = {
    pattern: string;
    concrete_path: string | null;
    depth: number;
    order: number;
    is_default: boolean;
    metadata: RouteMetadata;
};

type RouteCatalog = {
    version: number;
    site: {
        title: string;
        site_name: string;
        site_url: string | null;
        base_path: string;
        social_image: string | null;
        social_image_alt: string | null;
    };
    routes: RouteCatalogEntry[];
};

let routeCatalogPromise: Promise<RouteCatalog | null> | null = null;

function loadRouteCatalog(): Promise<RouteCatalog | null> {
    if (routeCatalogPromise != null) {
        return routeCatalogPromise;
    }
    let catalogUrl = new URL("route-metadata.json", document.baseURI);
    routeCatalogPromise = fetch(catalogUrl, {cache: "no-cache"})
        .then((response) => response.ok ? response.json() as Promise<RouteCatalog> : null)
        .catch(() => null);
    return routeCatalogPromise;
}

function decodeUriComponentSafely(value: string): string {
    try {
        return decodeURIComponent(value);
    } catch {
        return value;
    }
}

function normalizedPath(pathname: string): string {
    let segments = pathname
        .split("/")
        .filter((segment) => segment.length > 0)
        .map(decodeUriComponentSafely);
    return segments.length === 0 ? "/" : `/${segments.join("/")}`;
}

function patternMatches(pattern: string, pathname: string): boolean {
    let patternSegments = pattern.split("/").filter((segment) => segment.length > 0);
    let pathSegments = normalizedPath(pathname)
        .split("/")
        .filter((segment) => segment.length > 0);
    let pathIndex = 0;

    for (let patternIndex = 0; patternIndex < patternSegments.length; patternIndex++) {
        let segment = patternSegments[patternIndex];
        if (segment === "*") {
            return patternIndex === patternSegments.length - 1;
        }
        if (pathIndex >= pathSegments.length) {
            return false;
        }
        if (!segment.startsWith(":") && segment !== pathSegments[pathIndex]) {
            return false;
        }
        pathIndex += 1;
    }
    return pathIndex === pathSegments.length;
}

function selectedRoute(catalog: RouteCatalog, url: URL): RouteCatalogEntry | null {
    let selected: RouteCatalogEntry | null = null;
    for (let route of catalog.routes) {
        if (!patternMatches(route.pattern, url.pathname)) {
            continue;
        }
        if (
            selected == null
            || route.depth > selected.depth
            || (
                route.depth === selected.depth
                && selected.is_default
                && !route.is_default
            )
            || (
                route.depth === selected.depth
                && selected.is_default === route.is_default
                && route.order < selected.order
            )
        ) {
            selected = route;
        }
    }
    return selected;
}

function absoluteSiteUrl(catalog: RouteCatalog, path: string | null): string | null {
    if (catalog.site.site_url == null || path == null) {
        return null;
    }
    let base = new URL(`${catalog.site.site_url.replace(/\/+$/, "")}/`);
    return new URL(path.replace(/^\/+/, ""), base).href.replace(/\/+$/, "");
}

function absoluteResourceUrl(catalog: RouteCatalog, resource: string | null): string | null {
    if (resource == null) {
        return null;
    }
    try {
        let parsed = new URL(resource);
        if (parsed.protocol === "http:" || parsed.protocol === "https:") {
            return parsed.href;
        }
    } catch {
        // Resolve project-relative web resources against the configured public site below.
    }
    if (catalog.site.site_url == null) {
        return null;
    }
    let base = new URL(`${catalog.site.site_url.replace(/\/+$/, "")}/`);
    return new URL(resource.replace(/^\/+/, ""), base).href;
}

function setMeta(selectorKey: "name" | "property", key: string, value: string | null) {
    let selector = `meta[${selectorKey}="${CSS.escape(key)}"]`;
    let existing = document.head.querySelector(selector) as HTMLMetaElement | null;
    if (value == null) {
        existing?.remove();
        return;
    }
    if (existing == null) {
        existing = document.createElement("meta");
        existing.setAttribute(selectorKey, key);
        document.head.appendChild(existing);
    }
    existing.content = value;
}

function setCanonical(value: string | null) {
    let existing = document.head.querySelector('link[rel="canonical"]') as HTMLLinkElement | null;
    if (value == null) {
        existing?.remove();
        return;
    }
    if (existing == null) {
        existing = document.createElement("link");
        existing.rel = "canonical";
        document.head.appendChild(existing);
    }
    existing.href = value;
}

function applyRouteMetadata(catalog: RouteCatalog, route: RouteCatalogEntry, url: URL) {
    let metadata = route.metadata;
    let isConcretePath = route.concrete_path != null
        && normalizedPath(route.concrete_path) === normalizedPath(url.pathname);
    let shouldIndex = metadata.index && isConcretePath;
    let canonicalUrl = absoluteSiteUrl(catalog, isConcretePath ? route.concrete_path : null);
    let socialImageSource = metadata.social_image ?? catalog.site.social_image;
    let socialImage = absoluteResourceUrl(catalog, socialImageSource);
    let socialImageAlt = metadata.social_image_alt ?? catalog.site.social_image_alt;

    document.title = metadata.title;
    setMeta("name", "description", metadata.description);
    setMeta("name", "robots", shouldIndex ? "index,follow" : "noindex,follow");
    setCanonical(canonicalUrl);

    setMeta("property", "og:type", "website");
    setMeta("property", "og:site_name", catalog.site.site_name);
    setMeta("property", "og:title", metadata.title);
    setMeta("property", "og:description", metadata.description);
    setMeta("property", "og:url", canonicalUrl);
    setMeta("property", "og:image", socialImage);
    setMeta("property", "og:image:alt", socialImage == null ? null : socialImageAlt);

    setMeta("name", "twitter:card", socialImage == null ? "summary" : "summary_large_image");
    setMeta("name", "twitter:title", metadata.title);
    setMeta("name", "twitter:description", metadata.description);
    setMeta("name", "twitter:image", socialImage);
    setMeta("name", "twitter:image:alt", socialImage == null ? null : socialImageAlt);
}

function applyUnmatchedRouteMetadata(catalog: RouteCatalog) {
    document.title = catalog.site.title;
    setMeta("name", "description", null);
    setMeta("name", "robots", "noindex,follow");
    setCanonical(null);

    setMeta("property", "og:type", "website");
    setMeta("property", "og:site_name", catalog.site.site_name);
    setMeta("property", "og:title", catalog.site.title);
    setMeta("property", "og:description", null);
    setMeta("property", "og:url", null);
    setMeta("property", "og:image", null);
    setMeta("property", "og:image:alt", null);

    setMeta("name", "twitter:card", "summary");
    setMeta("name", "twitter:title", catalog.site.title);
    setMeta("name", "twitter:description", null);
    setMeta("name", "twitter:image", null);
    setMeta("name", "twitter:image:alt", null);
}

export async function updateDocumentRouteMetadata(url: URL) {
    let catalog = await loadRouteCatalog();
    if (catalog == null) {
        return;
    }
    let route = selectedRoute(catalog, url);
    if (route == null) {
        applyUnmatchedRouteMetadata(catalog);
        return;
    }
    applyRouteMetadata(catalog, route, url);
}
