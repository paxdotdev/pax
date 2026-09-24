import { browserRouteLocation, pushRouteHistoryState } from "./route-location";
import { updateDocumentRouteMetadata } from "./route-metadata";

function normalizePath(path: string): string {
    return path.replace(/%[0-9a-f]{2}/gi, escape => {
        const character = String.fromCharCode(parseInt(escape.slice(1), 16));
        return /^[A-Za-z0-9._~-]$/.test(character) ? character : escape.toUpperCase();
    });
}

let serverOwnedPrefixes: string[] | undefined;
function configuredPrefixes(): string[] {
    if (serverOwnedPrefixes === undefined) {
        const config = document.getElementById("pax-web-config");
        const prefixes = config ? JSON.parse(config.textContent || "{}").server_owned_prefixes : [];
        serverOwnedPrefixes = Array.isArray(prefixes) ? prefixes : [];
    }
    return serverOwnedPrefixes;
}

export function isServerOwnedPath(path: string, prefixes = configuredPrefixes()): boolean {
    path = normalizePath(path);
    return prefixes.some(prefix => prefix === "/" || path === prefix || path.startsWith(prefix + "/"));
}

/** Dispatch web navigation before changing the app's history or route metadata. */
export function navigateBrowser(destination: string | undefined, target: string | undefined, routeChanged: (url: URL) => void) {
    if (!destination) {
        console.error("no valid url target!");
        return;
    }
    let url: URL | undefined;
    try {
        url = new URL(destination, browserRouteLocation());
    } catch {
        // Preserve ordinary browser handling for malformed or unsupported URLs.
    }
    if (url && target === "current" && url.origin === window.location.origin) {
        if (isServerOwnedPath(url.pathname)) {
            // Do not push a synthetic app entry first: Back should return to the
            // actual app location, and query-backed embeds must leave their entry.
            window.location.assign(url.href);
        } else {
            pushRouteHistoryState(url);
            routeChanged(url);
            void updateDocumentRouteMetadata(url);
        }
        return;
    }
    window.open(destination, target === "new" ? "_blank" : "_self");
}
