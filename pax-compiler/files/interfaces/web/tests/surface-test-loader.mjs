import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformSync} from 'esbuild';

// Load the small, real scheduling modules in the fake DOM tests. Unrelated
// native controls remain stubbed; surface policy/reconciliation does not.
export function surfaceRequire(name) {
    const modules = new Set(['./surface-coordinator', './surface-host-policy',
        './surface-scheduler', './surface-trace', './image-loader']);
    if (!modules.has(name)) return {};
    const module = {exports: {}};
    const source = readFileSync(new URL(`../src/classes/${name.slice(2)}.ts`, import.meta.url), 'utf8');
    vm.runInNewContext(transformSync(source, {loader: 'ts', format: 'cjs'}).code, {
        module, exports: module.exports, require: surfaceRequire, performance, AbortController,
    });
    return module.exports;
}
