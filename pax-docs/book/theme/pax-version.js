(function () {
    const script = document.currentScript;
    const scriptUrl = script && script.src ? script.src : new URL("theme/pax-version.js", document.baseURI).href;
    const VERSION_SEGMENT = /^v?\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?(?:\+[A-Za-z0-9.-]+)?$/;

    function el(tag, className, text) {
        const node = document.createElement(tag);
        if (className) {
            node.className = className;
        }
        if (text !== undefined) {
            node.textContent = text;
        }
        return node;
    }

    function normalizeVersions(manifest) {
        if (!manifest || !Array.isArray(manifest.versions)) {
            return { latest: null, versions: [] };
        }

        const seen = new Set();
        const versions = [];
        manifest.versions.forEach((entry) => {
            const version = typeof entry === "string" ? entry : entry && entry.version;
            if (!version || seen.has(version)) {
                return;
            }
            seen.add(version);
            versions.push({
                version,
                label: (entry && entry.label) || version,
                path: normalizeVersionPath((entry && entry.path) || `/${version}/`),
            });
        });

        const latest = manifest.latest || null;
        return { latest, versions };
    }

    function normalizeVersionPath(path) {
        if (!path) {
            return "/";
        }
        return `/${String(path).replace(/^\/+|\/+$/g, "")}/`;
    }

    function currentVersionFromPath(versions) {
        const first = window.location.pathname.split("/").filter(Boolean)[0] || "";
        if (!first || !VERSION_SEGMENT.test(first)) {
            return "latest";
        }
        return versions.some((entry) => entry.version === first) ? first : "latest";
    }

    function pathWithoutVersion(versions) {
        const parts = window.location.pathname.split("/").filter(Boolean);
        if (parts.length && versions.some((entry) => entry.version === parts[0])) {
            parts.shift();
        }
        return parts.join("/");
    }

    function pathForSelection(selection, manifest) {
        const relative = pathWithoutVersion(manifest.versions);
        const suffix = relative ? `${relative}` : "";
        if (selection === "latest") {
            return `/${suffix}`;
        }

        const version = manifest.versions.find((entry) => entry.version === selection);
        const base = version ? version.path : normalizeVersionPath(selection);
        return `${base}${suffix}`;
    }

    function routeTo(selection, manifest) {
        const target = new URL(pathForSelection(selection, manifest), window.location.origin);
        target.search = window.location.search;
        target.hash = window.location.hash;
        if (target.href !== window.location.href) {
            window.location.href = target.href;
        }
    }

    function manifestUrls() {
        const urls = [];
        if (window.location.protocol === "http:" || window.location.protocol === "https:") {
            urls.push(new URL("/versions.json", window.location.origin).href);
        }
        urls.push(new URL("../versions.json", scriptUrl).href);
        return [...new Set(urls)];
    }

    function fetchManifest(urls) {
        const [url, ...rest] = urls;
        if (!url) {
            return Promise.resolve({ latest: null, versions: [] });
        }
        return fetch(url, { cache: "no-cache" })
            .then((response) => {
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                return response.json();
            })
            .catch((error) => {
                if (!rest.length) {
                    throw error;
                }
                return fetchManifest(rest);
            });
    }

    function installVersionPicker(manifest) {
        const rightButtons = document.querySelector("#menu-bar .right-buttons") || document.querySelector(".menu-bar .right-buttons");
        if (!rightButtons || document.querySelector(".pax-version-picker")) {
            return;
        }

        const picker = el("label", "pax-version-picker");
        const label = el("span", "pax-version-label", "Version:");
        const select = el("select", "pax-version-select");
        select.setAttribute("aria-label", "Select docs version");

        const latest = document.createElement("option");
        latest.value = "latest";
        latest.textContent = manifest.latest ? `Latest (${manifest.latest})` : "Latest";
        select.appendChild(latest);

        manifest.versions.forEach((entry) => {
            const option = document.createElement("option");
            option.value = entry.version;
            option.textContent = entry.label;
            select.appendChild(option);
        });

        select.value = currentVersionFromPath(manifest.versions);
        if (manifest.versions.length === 0) {
            select.disabled = true;
            latest.textContent = "dev";
        }

        select.addEventListener("change", () => routeTo(select.value, manifest));
        picker.appendChild(label);
        picker.appendChild(select);
        rightButtons.insertBefore(picker, rightButtons.firstChild);
    }

    function hydrate() {
        fetchManifest(manifestUrls())
            .then(normalizeVersions)
            .then(installVersionPicker)
            .catch(() => installVersionPicker({ latest: null, versions: [] }));
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", hydrate);
    } else {
        hydrate();
    }
})();
