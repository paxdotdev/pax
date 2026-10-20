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

export function readStartupOptions(yieldMount?: boolean) {
    const element = document.getElementById("pax-web-config");
    let config: unknown = {};
    try { config = JSON.parse(element?.textContent || "{}"); } catch { /* Custom hosts may omit metadata. */ }
    return resolveStartupOptions(config, window.location.search, yieldMount);
}
