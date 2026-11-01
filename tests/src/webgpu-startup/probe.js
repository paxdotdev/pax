// Qualification only: inject failures before the unmodified Pax host starts.
(() => {
    const params = new URLSearchParams(location.search);
    const mode = params.get('gpu_test') || 'real';
    const requireGpu = params.get('pax_renderer') === 'gpu';
    const output = document.body.appendChild(document.createElement('pre'));
    output.id = 'gpu-startup-test';
    output.style.cssText = 'position:fixed;inset:0 0 auto;max-height:40vh;overflow:auto;z-index:2147483647;background:#fff;color:#111;font:12px monospace;pointer-events:none';
    const write = message => output.textContent += `${Math.round(performance.now())} ${message}\n`;
    let adapters = 0, devices = 0, renderer;
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
            if (message.includes('using WebGPU browser renderer')) renderer = 'gpu';
            if (message.includes('using Vello CPU browser renderer')) renderer = 'cpu';
            write(message);
            original.apply(console, args);
        };
    }
    const gpu = navigator.gpu;
    if (mode === 'gpu-absent') {
        Object.defineProperty(navigator, 'gpu', {value: undefined});
    } else if (mode === 'context-null') {
        const getContext = HTMLCanvasElement.prototype.getContext;
        HTMLCanvasElement.prototype.getContext = function(kind, ...args) {
            return kind === 'webgpu' ? null : getContext.call(this, kind, ...args);
        };
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
        const expectedMounted = mode === 'real' || !requireGpu;
        const expectedRenderer = mode === 'real' ? 'gpu' : 'cpu';
        const expectedAdapters = ['gpu-absent', 'context-null'].includes(mode) ? 0 : 1;
        const expectedDevices = ['real', 'device-reject'].includes(mode) ? 1 : 0;
        const passed = mounted === expectedMounted && adapters === expectedAdapters
            && devices === expectedDevices && (!mounted || renderer === expectedRenderer);
        write(`event loop: ${heartbeatCount} heartbeats; longest interval=${Math.round(longestHeartbeat)}ms`);
        write(`${passed ? 'PASS' : 'FAIL'} mode=${mode} mounted=${mounted} renderer=${renderer} adapters=${adapters} devices=${devices}`);
        output.dataset.result = passed ? 'pass' : 'fail';
    };
    document.addEventListener('visibilitychange', () => write(`visibility=${document.visibilityState}`));
    write(`mode=${mode} requireGpu=${requireGpu} visibility=${document.visibilityState}`);
})();
