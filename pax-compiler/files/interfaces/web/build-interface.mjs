import {build} from 'esbuild';

const worker = await build({entryPoints: ['src/classes/native-mask-worker.ts'], bundle: true, write: false});
await build({entryPoints: ['src/index.ts'], bundle: true, globalName: 'Pax',
    outfile: 'public/pax-interface-web.js',
    define: {__PAX_NATIVE_MASK_WORKER_SOURCE__: JSON.stringify(worker.outputFiles[0].text)}});
