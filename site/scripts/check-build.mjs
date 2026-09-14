import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const output = resolve('dist');
const base = '/buffer-uppercut';
for (const route of ['index.html', 'downloads/index.html', 'installation/index.html']) {
  const html = readFileSync(resolve(output, route), 'utf8');
  assert.match(html, /<html lang="en">/);
  assert.match(html, /id="main"/);
  assert.doesNotMatch(html, /Bearer |github_pat_|ghp_/);
  for (const [, url] of html.matchAll(/(?:href|src)="([^"#]+)"/g)) {
    if (!url.startsWith('/')) continue;
    assert.ok(url.startsWith(`${base}/`), `${route}: missing repository base in ${url}`);
    const relative = decodeURIComponent(url.slice(base.length + 1).split(/[?#]/)[0]);
    const file = resolve(output, relative.endsWith('/') || relative === '' ? `${relative}index.html` : relative);
    assert.ok(file.startsWith(`${output}/`) && existsSync(file), `${route}: missing asset ${url}`);
  }
}
console.log('All three static pages, internal links and local assets verified.');
