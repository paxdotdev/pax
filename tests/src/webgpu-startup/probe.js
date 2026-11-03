// Qualification only: inject failures before the unmodified Pax host starts.
(() => {
    const params = new URLSearchParams(location.search);
    const mode = params.get('gpu_test') || 'real';
    const requireGpu = params.get('pax_renderer') === 'gpu';
    const requireCpu = params.get('pax_renderer') === 'cpu';
    const output = document.body.appendChild(document.createElement('pre'));
    output.id = 'gpu-startup-test';
    output.style.cssText = 'position:fixed;inset:0 0 auto;max-height:40vh;overflow:auto;z-index:2147483647;background:#fff;color:#111;font:12px monospace;pointer-events:none';
    const write = message => output.textContent += `${Math.round(performance.now())} ${message}\n`;
    let adapters = 0, devices = 0;
    // Observe successful acquisition without asking for a context ourselves: doing
    // so would lock an unused canvas to a backend and could change the result.
    const contexts = new WeakMap();
    const getContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function(kind, ...args) {
        if (mode === 'context-null' && kind === 'webgpu') return null;
        const context = getContext.call(this, kind, ...args);
        if (context) contexts.set(this, kind);
        return context;
    };
    let lastHeartbeat = performance.now();
    let heartbeatCount = 0, longestHeartbeat = 0;
    const heartbeat = setInterval(() => {
        const now = performance.now();
        longestHeartbeat = Math.max(longestHeartbeat, now - lastHeartbeat);
        lastHeartbeat = now;
        heartbeatCount++;
    }, 250);
    for (const level of ['info', 'log', 'warn', 'error']) {
        const original = console[level];
        console[level] = (...args) => {
            const message = args.map(String).join(' ');
            write(message);
            original.apply(console, args);
        };
    }
    const gpu = navigator.gpu;
    if (mode === 'gpu-absent') {
        Object.defineProperty(navigator, 'gpu', {value: undefined});
    } else if (gpu) {
        const requestAdapter = gpu.requestAdapter.bind(gpu);
        gpu.requestAdapter = options => {
            adapters++;
            write(`adapter ${adapters} requested ${JSON.stringify(options)}`);
            if (adapters > 1) return Promise.reject(new Error('REGRESSION: redundant adapter acquisition'));
            if (mode === 'adapter-null') return Promise.resolve(null);
            if (mode === 'adapter-reject') return Promise.reject(new Error('Injected adapter failure'));
            const pending = requestAdapter(options);
            pending.then(adapter => {
                write(`adapter resolved: ${Boolean(adapter)}`);
                if (!adapter) return;
                const requestDevice = adapter.requestDevice.bind(adapter);
                adapter.requestDevice = descriptor => {
                    devices++;
                    write(`device ${devices} requested`);
                    if (mode === 'device-reject') return Promise.reject(new Error('Injected device failure'));
                    const device = requestDevice(descriptor);
                    device.then(() => write('device resolved'), error => write(`device rejected: ${error}`));
                    return device;
                };
            }, error => write(`adapter rejected: ${error}`));
            return pending;
        };
    }
    window.finishGpuProbe = mounted => {
        clearInterval(heartbeat);
        // The detached GPU capability canvas may exist even after CPU fallback.
        // Only current Pax layer canvases identify the mounted renderer; ignore
        // retired canvases, auxiliary canvases, and tiles not initialized yet.
        const canvasContexts = [...document.querySelectorAll('canvas[data-layer-id]')]
            .map(canvas => contexts.get(canvas)).filter(Boolean);
        const kinds = new Set(canvasContexts);
        const renderer = kinds.size !== 1 ? (kinds.size ? 'mixed' : 'unknown')
            : kinds.has('webgpu') ? 'gpu' : kinds.has('2d') ? 'cpu' : 'unknown';
        const expectedMounted = requireCpu || mode === 'real' || !requireGpu;
        const expectedRenderer = !requireCpu && mode === 'real' ? 'gpu' : 'cpu';
        const expectedAdapters = requireCpu || ['gpu-absent', 'context-null'].includes(mode) ? 0 : 1;
        const expectedDevices = !requireCpu && ['real', 'device-reject'].includes(mode) ? 1 : 0;
        const passed = mounted === expectedMounted && adapters === expectedAdapters
            && devices === expectedDevices && (!mounted || renderer === expectedRenderer);
        write(`event loop: ${heartbeatCount} heartbeats; longest interval=${Math.round(longestHeartbeat)}ms`);
        write(`${passed ? 'PASS' : 'FAIL'} mode=${mode} mounted=${mounted} renderer=${renderer} adapters=${adapters} devices=${devices} canvases=${canvasContexts.length}`);
        output.dataset.result = passed ? 'pass' : 'fail';
    };
    document.addEventListener('visibilitychange', () => write(`visibility=${document.visibilityState}`));
    write(`mode=${mode} requireGpu=${requireGpu} requireCpu=${requireCpu} visibility=${document.visibilityState}`);
})();
