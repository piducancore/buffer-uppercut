import { test } from 'node:test';
import assert from 'node:assert/strict';
import { packagesFor, publicReleases } from '../src/lib/releases.mjs';

test('only distributable unsigned plugin packages get download buttons', () => {
  const assets = [
    'Buffer-Uppercut-0.1.0-windows-x64-unsigned.zip',
    'Buffer-Uppercut-0.1.0-linux-x64-not-for-distribution.zip',
    'Buffer-Uppercut-0.1.0-macos.zip',
    'standalone-windows-x64-unsigned.zip',
  ].map((name) => ({ name }));
  assert.deepEqual(packagesFor({ assets }).map((item) => item.slug), ['windows-x64']);
});

test('drafts never leak into history and newest published date comes first', () => {
  const data = [
    { tag_name: 'old', draft: false, published_at: '2026-01-01' },
    { tag_name: 'draft', draft: true, published_at: '2026-09-14' },
    { tag_name: 'new', draft: false, prerelease: true, published_at: '2026-09-01' },
    { tag_name: 'unpublished', draft: false, published_at: null },
  ];
  assert.deepEqual(publicReleases(data).map((item) => item.tag_name), ['new', 'old']);
});
