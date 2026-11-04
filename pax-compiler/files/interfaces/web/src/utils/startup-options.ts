export type LoaderExit = 'full_mount' | 'first_viewport';

/** Loader exit policy is independent of the incremental-mount scheduling choice. */
export function resolveLoaderExit(config: unknown, override?: LoaderExit): LoaderExit {
    if (override !== undefined && override !== 'full_mount' && override !== 'first_viewport') {
        throw new Error('loaderExit must be full_mount or first_viewport');
    }
    const configured = typeof config === 'object' && config !== null
        && (config as {loader_exit?: unknown}).loader_exit === 'first_viewport';
    return override ?? (configured ? 'first_viewport' : 'full_mount');
}

export function readLoaderExit(override?: LoaderExit): LoaderExit {
    return resolveLoaderExit(readWebConfig(), override);
}

/** Resolve once in the host and pass the same scheduling choice into Wasm. */
export function resolveStartupOptions(config: unknown, search: string, yieldMount?: boolean) {
    if (yieldMount !== undefined && typeof yieldMount !== "boolean") {
        throw new Error("yieldMount must be a boolean");
    }
    const query = new URLSearchParams(search);
    const override = query.get("pax_mount_yield");
    const configured = typeof config === "object" && config !== null
        && (config as {yield_mount?: unknown}).yield_mount === true;
    const yielded = override === "0" ? false : override === "1" ? true : yieldMount ?? configured;
    return {
        yieldMount: yielded,
    };
}

export function readWebConfig(): unknown {
    const element = document.getElementById("pax-web-config");
    let config: unknown = {};
    try { config = JSON.parse(element?.textContent || "{}"); } catch { /* Custom hosts may omit metadata. */ }
    return config;
}

export function readStartupOptions(yieldMount?: boolean) {
    return resolveStartupOptions(readWebConfig(), window.location.search, yieldMount);
}
