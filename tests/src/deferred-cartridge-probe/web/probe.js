import init, {probe_table, probe_stage} from './pax-cartridge.js';

const status = document.querySelector('#status');
const button = document.querySelector('#load');
const timings = {};
const start = performance.now();
const app = await window.Pax.init_web({element: document.querySelector('#app'), cartridgeUrl: new URL('./pax-cartridge', location.href).href});
const raw = await init();
timings.baseReadyMs = performance.now() - start;
const monolithic = !!raw.probe_stats(4);
let pending, module, slot, registered = monolithic;
let bridgeCalls = 0;

async function prefetch({url = './child.wasm', bridge = 'wasm', signal, expectedAbi = 2} = {}) {
  if (module) return module;
  if (pending) return pending;
  pending = (async () => {
    const begin = performance.now();
    const response = await fetch(url, {signal});
    if (!response.ok) throw new Error(`Child fetch: HTTP ${response.status}`);
    const bytes = await response.arrayBuffer();
    if (signal?.aborted) throw signal.reason;
    timings.childFetchMs = performance.now() - begin;
    const imports = bridge === 'js' ? {
      probe_bind: (context, field) => { bridgeCalls++; return raw.probe_bind(context, field); },
      probe_read: handle => { bridgeCalls++; return raw.probe_read(handle); },
      probe_write: (handle, value) => { bridgeCalls++; return raw.probe_write(handle, value); },
    } : {probe_bind: raw.probe_bind, probe_read: raw.probe_read, probe_write: raw.probe_write};
    const compileStart = performance.now();
    const result = await WebAssembly.instantiate(bytes, {pax_probe: imports});
    if (signal?.aborted) throw signal.reason;
    if (result.instance.exports.probe_abi() !== expectedAbi) throw new Error('Child ABI mismatch');
    if (result.instance.exports.probe_capabilities() !== 0) throw new Error('Unavailable runtime capability');
    timings.childCompileInstantiateMs = performance.now() - compileStart;
    const exports = result.instance.exports;
    const data = new Uint8Array(exports.memory.buffer);
    const definition = data.slice(exports.probe_definition_ptr(), exports.probe_definition_ptr() + exports.probe_definition_len());
    const descriptor = new TextDecoder('utf-8', {fatal: true}).decode(data.subarray(exports.probe_descriptor_ptr(), exports.probe_descriptor_ptr() + exports.probe_descriptor_len()));
    const stageStart = performance.now();
    probe_stage(definition, descriptor);
    timings.definitionRegistrationMs = performance.now() - stageStart;
    timings.definitionBytes = definition.byteLength;
    timings.descriptorBytes = new TextEncoder().encode(descriptor).byteLength;
    module = result.instance;
    status.textContent = 'Fetched; no child instance mounted';
    return module;
  })();
  try { return await pending; } finally { pending = undefined; }
}

function activate() {
  if (registered) return;
  if (!module) throw new Error('Prefetch before activation');
  const table = probe_table();
  if (!(table instanceof WebAssembly.Table)) throw new Error('Host function table was not exported');
  slot = table.grow(1);
  table.set(slot, module.exports.probe_dispatch);
  const begin = performance.now();
  if (raw.probe_register(slot, module.exports.probe_abi(), module.exports.probe_capabilities()) !== 1) {
    table.set(slot, null);
    throw new Error('Host rejected child registration');
  }
  timings.registrationMs = performance.now() - begin;
  registered = true;
  status.textContent = 'Child registered in the base scene';
}

button.disabled = monolithic;
status.textContent = monolithic ? 'Monolithic control' : 'Base ready · child not fetched';
button.onclick = async () => {
  button.disabled = true;
  try { await prefetch({bridge: document.querySelector('#bridge').value}); activate(); }
  catch (error) { status.textContent = String(error); button.disabled = false; }
};

window.probe = {raw, app, prefetch, activate, timings,
  get module() { return module; }, get bridgeCalls() { return bridgeCalls; },
  get registered() { return registered; },
};
