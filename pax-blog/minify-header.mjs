import {readFile, writeFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
import {resolve, join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {gzipSync} from 'node:zlib';

// Reuse the pinned toolchain that builds Pax's web interface; no second npm lock.
const require = createRequire(new URL('../pax-compiler/files/interfaces/web/package.json', import.meta.url));
const {transform} = require('esbuild');

export async function minifyHeaderBundle(directory) {
    const stats = [];
    for (const name of ['pax-interface-web.js', 'pax-cartridge.js']) {
        const path = join(directory, name);
        const source = await readFile(path, 'utf8');
        // Preserve the interface's classic-script global (window.Pax) and the
        // cartridge's ESM exports/import URLs. This is minification, not bundling.
        const {code} = await transform(source, {
            minify: true, legalComments: 'inline', target: 'es2020', sourcefile: name,
        });
        await writeFile(path, code);
        stats.push({name, before: gzipSync(source).length, after: gzipSync(code).length});
    }
    return stats;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
    if (process.argv.length !== 3) throw new Error('Usage: node minify-header.mjs <staged-header-directory>');
    for (const stat of await minifyHeaderBundle(resolve(process.argv[2]))) {
        console.log(`${stat.name}: ${stat.before} → ${stat.after} gzip bytes`);
    }
}
