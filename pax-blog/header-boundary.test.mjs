import test from 'node:test';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const demos = ['paxflix', 'calculator', 'led-oscilloscope', 'space-game'];
function dependencyGraph(manifest) {
    const metadata = JSON.parse(execFileSync('cargo', [
        'metadata', '--manifest-path', fileURLToPath(manifest), '--format-version', '1',
        '--filter-platform', 'wasm32-unknown-unknown', '--features', 'pax-kit/web', '--locked',
    ], {encoding: 'utf8', maxBuffer: 16 * 1024 * 1024}));
    const packages = new Map(metadata.packages.map(pkg => [pkg.id, pkg.name]));
    return new Map(metadata.resolve.nodes.map(node => [packages.get(node.id), node.features]));
}

test('blog header resolves no demo cartridges or custom shader feature', () => {
    const graph = dependencyGraph(new URL('./header/Cargo.toml', import.meta.url));
    assert.ok(graph.has('pax-website'), 'header still imports the canonical website component');
    assert.ok(!graph.get('pax-website').includes('app'));
    for (const demo of demos) assert.ok(!graph.has(demo), `${demo} leaked into header`);
    for (const crate of ['pax-kit', 'pax-std', 'pax-runtime', 'pax-gpu']) {
        assert.ok(!graph.get(crate).includes('shaders'), `${crate}/shaders leaked into header`);
    }
});

test('default website retains all demo cartridges and custom shaders', () => {
    const graph = dependencyGraph(new URL('../pax-website/Cargo.toml', import.meta.url));
    assert.ok(graph.get('pax-website').includes('app'));
    for (const demo of demos) assert.ok(graph.has(demo), `${demo} missing from website`);
    assert.ok(graph.get('pax-kit').includes('shaders'));
});
