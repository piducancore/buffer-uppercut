import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkInventory, checkNotices } from './check-notices.mjs';

const data = (license, text) => ({ third_party_libraries: [{
  package_name: 'dependency', package_version: '1', license,
  licenses: [{ license, text }],
}] });

test('rejects empty and unresolved baselines', () => {
  assert.throws(() => checkNotices({ third_party_libraries: [] }));
  assert.throws(() => checkNotices(data('MIT', 'NOT FOUND')));
  assert.doesNotThrow(() => checkNotices(data('MIT', 'Upstream license text')));
});

test('root MIT must not stand in for custom upstream licenses', () => {
  assert.throws(() => checkNotices(data('LicenseRef-TruceLicense-1.0', 'MIT License')));
  assert.throws(() => checkNotices(data('LicenseRef-Slint-Royalty-free-2.0', 'MIT License')));
});

test('requires the approved and discovered package inventories to match', () => {
  const baseline = data('MIT', 'Upstream license text');
  assert.doesNotThrow(() => checkInventory(baseline, structuredClone(baseline)));
  const changed = structuredClone(baseline);
  changed.third_party_libraries[0].package_version = '2';
  assert.throws(() => checkInventory(baseline, changed), /inventory changed/);
});
