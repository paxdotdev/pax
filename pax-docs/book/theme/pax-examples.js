(function () {
    const script = document.currentScript;
    const scriptUrl = script && script.src ? script.src : new URL("theme/pax-examples.js", document.baseURI).href;
    const examplesRoot = new URL("../_pax_examples/", scriptUrl);

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

    function exampleUrl(path, suffix) {
        return new URL(`${path}/${suffix}`, examplesRoot).href;
    }

    function replaceHost(host, node) {
        const parent = host.parentElement;
        if (
            parent &&
            parent.tagName === "P" &&
            parent.children.length === 1 &&
            parent.textContent.trim() === ""
        ) {
            parent.replaceWith(node);
        } else {
            host.replaceWith(node);
        }
    }

    function createChrome(host, manifest) {
        const title = host.getAttribute("title") || manifest.title || manifest.path;
        const requestedHeight = Number(host.getAttribute("height") || manifest.height || 520);
        const height = Number.isFinite(requestedHeight) && requestedHeight > 0 ? requestedHeight : 520;

        const card = el("section", "pax-example-card");
        const header = el("header", "pax-example-header");
        const heading = el("h4", "pax-example-title", `Example: ${title}`);
        const actions = el("div", "pax-example-actions");
        if (
            manifest.app &&
            manifest.app.available &&
            manifest.app.build_skipped &&
            manifest.built_fingerprint !== manifest.source_fingerprint
        ) {
            actions.appendChild(el("span", "pax-example-stale", "bundle may be stale"));
        }
        header.appendChild(heading);
        header.appendChild(actions);
        card.appendChild(header);

        if (manifest.app && manifest.app.available) {
            const entryUrl = new URL(exampleUrl(manifest.path, manifest.app.index || "app/index.html"));
            entryUrl.searchParams.set("pax_route", "/");
            const appUrl = entryUrl.href;
            const controls = el("div", "pax-example-controls");
            const restart = el("button", "pax-example-button", "Restart");
            restart.type = "button";
            restart.setAttribute("aria-label", `Restart ${title} from the beginning`);
            const standalone = el("a", "pax-example-standalone", "Open standalone ↗");
            standalone.href = appUrl;
            standalone.target = "_blank";
            standalone.rel = "noopener noreferrer";
            const status = el("span", "pax-example-status");
            status.setAttribute("role", "status");
            status.hidden = true;
            controls.append(restart, standalone, status);
            card.appendChild(controls);

            const stage = el("div", "pax-example-stage");
            card.appendChild(stage);
            let generation = 0;
            let currentIframe;
            // Observe the live stage, not its potentially very long source-code panel.
            let inViewport = typeof IntersectionObserver === "undefined";
            function syncPlayback() {
                currentIframe?.contentWindow?.Pax?.setSuspended?.(!inViewport || document.hidden);
            }
            if (typeof IntersectionObserver !== "undefined") {
                const observer = new IntersectionObserver(entries => {
                    inViewport = entries.some(entry => entry.isIntersecting);
                    syncPlayback();
                });
                observer.observe(stage);
            }
            document.addEventListener("visibilitychange", syncPlayback);
            function mountExample() {
                const currentGeneration = ++generation;
                const iframe = el("iframe", "pax-example-frame");
                iframe.title = `${title} — interactive example`;
                iframe.sandbox = "allow-scripts allow-same-origin allow-forms allow-pointer-lock";
                iframe.style.height = `${height}px`;
                currentIframe = iframe;
                iframe.addEventListener("load", () => {
                    if (currentGeneration === generation) syncPlayback();
                });
                status.textContent = "";
                status.hidden = true;
                iframe.addEventListener("error", () => {
                    if (currentGeneration === generation) {
                        status.textContent = "Could not load the example. Try Restart or Open standalone.";
                        status.hidden = false;
                    }
                });
                const embeddedUrl = new URL(appUrl);
                // Prevent a frame from running before the observer/load handshake.
                embeddedUrl.searchParams.set("pax_suspended", "1");
                iframe.src = embeddedUrl.href;
                // Replace the browsing context so timers, input state, and the old runtime end together.
                stage.replaceChildren(iframe);
            }
            restart.addEventListener("click", mountExample);
            mountExample();
        } else {
            const missing = el("div", "pax-example-missing");
            const message = manifest.app && manifest.app.build_error
                ? `Example build failed: ${manifest.app.build_error}`
                : manifest.app && manifest.app.build_skipped
                    ? "Example build skipped. Run `pax-cli docs build` without `--skip-examples` to generate the runnable embed."
                    : "Runnable example output is not available yet.";
            missing.textContent = message;
            card.appendChild(missing);
        }

        if (manifest.files && manifest.files.length) {
            card.appendChild(createSourceTabs(manifest.files));
        }

        return card;
    }

    function createSourceTabs(files) {
        const source = el("div", "pax-example-source");
        const tabs = el("div", "pax-example-tabs");
        const panels = el("div", "pax-example-panels");

        files.forEach((file, index) => {
            const button = el("button", "pax-example-tab", file.path);
            button.type = "button";
            button.setAttribute("aria-selected", index === 0 ? "true" : "false");

            const panel = el("div", "pax-example-panel");
            panel.hidden = index !== 0;

            const pre = el("pre", "pax-example-pre");
            const code = el("code", languageClass(file.language), file.contents || "");
            pre.appendChild(code);
            panel.appendChild(pre);
            if (window.PaxHighlight && typeof window.PaxHighlight.highlightCode === "function") {
                window.PaxHighlight.highlightCode(code);
            } else if (window.hljs && typeof window.hljs.highlightElement === "function") {
                window.hljs.highlightElement(code);
            } else if (window.hljs && typeof window.hljs.highlightBlock === "function") {
                window.hljs.highlightBlock(code);
            }

            if (file.truncated) {
                panel.appendChild(el("p", "pax-example-truncated", "Source truncated for docs output."));
            }

            button.addEventListener("click", () => {
                Array.from(tabs.children).forEach((child) => {
                    child.setAttribute("aria-selected", child === button ? "true" : "false");
                });
                Array.from(panels.children).forEach((child) => {
                    child.hidden = child !== panel;
                });
            });

            tabs.appendChild(button);
            panels.appendChild(panel);
        });

        source.appendChild(tabs);
        source.appendChild(panels);
        return source;
    }

    function languageClass(language) {
        switch (language) {
            case "rust":
                return "language-rust";
            case "pax":
                return "language-pax";
            default:
                return language ? `language-${language}` : "";
        }
    }

    function hydrate(host) {
        const path = host.getAttribute("path");
        if (!path) {
            replaceHost(host, el("div", "pax-example-error", "Missing pax-example path."));
            return;
        }

        fetch(exampleUrl(path, "manifest.json"))
            .then((response) => {
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                return response.json();
            })
            .then((manifest) => replaceHost(host, createChrome(host, manifest)))
            .catch((error) => {
                const fallback = el("div", "pax-example-error");
                fallback.textContent = `Example metadata for \`${path}\` is unavailable. Run \`pax-cli docs build\`. (${error.message})`;
                replaceHost(host, fallback);
            });
    }

    function hydrateAll() {
        document.querySelectorAll("pax-example").forEach(hydrate);
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", hydrateAll);
    } else {
        hydrateAll();
    }
})();
