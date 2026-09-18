// Check the learning path and API reference under both publication URL layouts.
// No network requests or writes; example bundle completeness is also checked by
// the publisher itself before it records or uploads a build.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const book = fileURLToPath(new URL('../book/', import.meta.url));
const output = path.resolve(process.argv[2] || path.join(book, 'book'));
const summary = fs.readFileSync(path.join(book, 'src/SUMMARY.md'), 'utf8');
const pages = [...summary.matchAll(/^\s*- \[[^\]]+\]\(([^)]+\.md)\)$/gm)]
    .map(match => match[1].replace(/\.md$/, '.html'));
const cache = new Map();
const read = relative => {
    if (!cache.has(relative)) cache.set(relative, fs.readFileSync(path.join(output, relative), 'utf8'));
    return cache.get(relative);
};
const errors = new Set();
let links = 0;
for (const prefix of ['/', '/0.39.0/']) {
    for (const page of pages) {
        const html = read(page);
        const main = html.match(/<main>([\s\S]*?)<\/main>/)?.[1];
        assert(main, `No article body in ${page}`);
        for (const [, raw] of main.matchAll(/<a\b[^>]*\bhref="([^"]+)"/g)) {
            const href = raw.replaceAll('&amp;', '&');
            if (/^(https?:|mailto:|tel:)/.test(href)) continue;
            links++;
            const target = new URL(href, `https://docs.example${prefix}${page}`);
            if (!target.pathname.startsWith(prefix)) {
                errors.add(`${page}: escapes publication prefix: ${href}`);
                continue;
            }
            const relative = decodeURIComponent(target.pathname.slice(prefix.length));
            if (!fs.existsSync(path.join(output, relative))) {
                errors.add(`${page}: missing file: ${href}`);
                continue;
            }
            if (target.hash && relative.endsWith('.html')) {
                const ids = new Set([...read(relative).matchAll(/\bid="([^"]+)"/g)].map(match => match[1]));
                if (!ids.has(decodeURIComponent(target.hash.slice(1)))) {
                    errors.add(`${page}: missing anchor: ${href}`);
                }
            }
        }
    }
}
console.log(JSON.stringify({ pages: pages.length, urlLayouts: 2, links, errors: [...errors] }, null, 2));
if (errors.size) process.exitCode = 1;
