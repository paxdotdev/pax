// Shared by the independently packaged blog and website. Never initialize in
// the Pax runtime: mounting a loader, header, or embedded demo isn't navigation.
(() => {
    const staging = location.origin === 'https://staging.pax.dev';
    if ((!staging && location.origin !== 'https://www.pax.dev') || window.top !== window ||
        document.getElementById('pax-analytics-settings')) return;

    const id = 'G-MG3GLRDHYR';
    const key = 'pax.analytics-consent.v1';
    const disable = 'ga-disable-' + id;
    const ctas = ['hero', 'footer'];
    const placements = new Map([
        ['copy_install', ctas], ['copy_ai_prompt', ctas], ['open_in_codex', ctas],
        ['view_source', [...ctas, 'paxflix', 'cube', 'modularity', 'toolkit', 'open_source', 'oscilloscope']],
    ]);
    const adsOff = {ad_storage: 'denied', ad_user_data: 'denied', ad_personalization: 'denied'};
    let choice = null;
    let initialized = false;
    const read = () => {
        try { return localStorage.getItem(key); } catch { return choice; }
    };
    function gtag() { window.dataLayer.push(arguments); }

    function start() {
        window[disable] = false;
        if (!initialized) {
            initialized = true;
            window.dataLayer ||= [];
            window.gtag = gtag;
            gtag('consent', 'default', {...adsOff, analytics_storage: 'granted'});
            gtag('js', new Date());
            const ref = document.referrer ? new URL(document.referrer) : null;
            gtag('config', id, {
                // GA requires omitting debug_mode (not setting it to false) in production.
                ...(staging ? {debug_mode: true} : {}),
                // One automatic pageview per document. Never pass URL queries or fragments.
                page_location: location.origin + location.pathname,
                page_referrer: ref ? ref.origin + (ref.origin === location.origin ? ref.pathname : '/') : '',
                cookie_domain: 'none', allow_google_signals: false, allow_ad_personalization_signals: false,
            });
            const tag = document.createElement('script');
            tag.async = true;
            tag.referrerPolicy = 'no-referrer';
            tag.src = 'https://www.googletagmanager.com/gtag/js?id=' + id;
            document.head.append(tag);
        }
        window.paxSiteAction = (name, placement) => {
            if (choice === 'granted' && placements.get(name)?.includes(placement)) {
                gtag('event', name, {placement, send_to: id});
            }
        };
    }

    const css = document.createElement('link');
    css.rel = 'stylesheet';
    css.href = new URL('consent.css', document.currentScript.src).href;
    document.head.append(css);
    const panel = document.createElement('details');
    panel.id = 'pax-analytics-settings';
    panel.innerHTML = `<summary>Analytics settings</summary>
        <p>May we use Google Analytics cookies to count page visits and actions like copying the install command? We don’t send clipboard contents or prompts, or use analytics for ads.</p>
        <div><button type="button" value="denied">Decline</button><button type="button" value="granted">Allow analytics</button></div>`;
    document.body.append(panel);
    const buttons = panel.querySelectorAll('button');

    function apply(value) {
        const previous = choice;
        choice = value === 'granted' || value === 'denied' ? value : null;
        if (choice === 'granted') {
            start();
        } else {
            window[disable] = true;
            delete window.paxSiteAction;
            if (previous === 'granted') {
                // Our tag uses host-only, root-path cookies; leave other sites' cookies alone.
                for (const name of ['_ga', '_ga_' + id.slice(2)]) {
                    document.cookie = name + '=; Max-Age=0; Path=/; Secure; SameSite=Lax';
                }
            }
        }
        panel.open = !choice;
        for (const button of buttons) button.setAttribute('aria-pressed', String(button.value === choice));
    }
    for (const button of buttons) button.addEventListener('click', () => {
        try { localStorage.setItem(key, button.value); } catch { /* This document only. */ }
        apply(button.value);
        panel.querySelector('summary').focus({preventScroll: true});
    });
    window.addEventListener('storage', event => {
        if (event.key === key || event.key === null) apply(read());
    });
    window.addEventListener('pageshow', event => {
        if (event.persisted) apply(read());
    });
    apply(read());
})();
