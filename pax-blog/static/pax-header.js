// Articles never wait for Pax. Only this reserved navigation region is enhanced.
export function navigationDestination(detail, base) {
    if (!detail || typeof detail.url !== "string") return null;
    let url;
    try { url = new URL(detail.url, base); } catch { return null; }
    if (!['http:', 'https:'].includes(url.protocol)) return null;
    return {url: url.href, newTab: detail.target === 'new'};
}

export async function enhanceHeader(header, loadPax) {
    const mount = header.querySelector('[data-pax-header-mount]');
    const fallback = header.querySelector('.pax-header-fallback');
    const controller = new AbortController();
    const close = () => controller.abort();
    // Retain a bfcache-restored instance; do not turn Back into another startup.
    window.addEventListener('pagehide', event => { if (!event.persisted) close(); });
    mount.addEventListener('pax-navigate', event => {
        const destination = navigationDestination(event.detail, document.baseURI);
        if (!destination) return;
        if (destination.newTab) window.open(destination.url, '_blank', 'noopener');
        else window.location.assign(destination.url);
    });
    try {
        const Pax = await loadPax();
        await Pax.init_web({
            element: mount,
            cartridgeUrl: new URL('_pax/header/pax-cartridge', import.meta.url).href,
            pageScroll: false,
            documentRouting: false,
            signal: controller.signal,
        });
        // Let the first native text/canvas presentation reach the screen before
        // replacing the fallback. It stays available to keyboard/assistive users.
        await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
        if (controller.signal.aborted) return;
        header.dataset.paxReady = 'true';
        // The HTML links provide keyboard semantics; the canvas region must not
        // introduce a duplicate keyboard stop or replace a focused fallback link.
        for (const element of mount.querySelectorAll('[tabindex]')) element.tabIndex = -1;
        const syncFocus = () => {
            const focused = fallback.contains(document.activeElement);
            header.dataset.htmlFocus = String(focused);
            mount.inert = focused;
        };
        header.addEventListener('focusin', syncFocus);
        header.addEventListener('focusout', () => queueMicrotask(syncFocus));
        syncFocus();
    } catch (error) {
        close();
        header.dataset.paxReady = 'failed';
        console.warn('Pax header unavailable; using HTML navigation.', error);
    }
}

if (typeof document !== 'undefined') {
    const header = document.querySelector('[data-pax-header]');
    if (header) void enhanceHeader(header, () => new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = new URL('_pax/header/pax-interface-web.js', import.meta.url).href;
        script.onload = () => window.Pax?.init_web ? resolve(window.Pax) : reject(new Error('Pax interface missing'));
        script.onerror = () => reject(new Error('Pax header bundle did not load'));
        document.head.append(script);
    }));
}
