import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export function checkNotices(data) {
  if (!Array.isArray(data.third_party_libraries) || !data.third_party_libraries.length) {
    throw new Error('Empty or invalid third-party notice baseline');
  }
  for (const dependency of data.third_party_libraries) {
    const label = `${dependency.package_name}@${dependency.package_version}`;
    if (!dependency.licenses?.length || dependency.licenses.some((license) =>
      !license.text?.trim() || /NOT FOUND/i.test(license.text))) {
      throw new Error(`Unresolved license text: ${label}`);
    }
    if (dependency.license?.includes('LicenseRef-TruceLicense-1.0') &&
      !dependency.licenses.some((license) => /Truce License/i.test(license.text) && /Framework Rider/i.test(license.text))) {
      throw new Error(`Missing upstream TRUCE license/rider: ${label}`);
    }
    if (dependency.license?.includes('LicenseRef-Slint-Royalty-free-2.0') &&
      !dependency.licenses.some((license) => license.license === 'LicenseRef-Slint-Royalty-free-2.0' && /Attribution/.test(license.text))) {
      throw new Error(`Missing selected Slint royalty-free license: ${label}`);
    }
  }
}

export function checkInventory(baseline, discovered) {
  const keys = (data) => new Set(data.third_party_libraries.map((dependency) =>
    `${dependency.package_name}@${dependency.package_version}`));
  const approved = keys(baseline);
  const current = keys(discovered);
  const missing = [...current].filter((dependency) => !approved.has(dependency));
  const stale = [...approved].filter((dependency) => !current.has(dependency));
  if (missing.length || stale.length) {
    throw new Error(`Third-party inventory changed; missing approval: ${missing.join(', ') || 'none'}; stale: ${stale.join(', ') || 'none'}`);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const baseline = JSON.parse(readFileSync(process.argv[2], 'utf8'));
  checkNotices(baseline);
  if (process.argv[3]) {
    checkInventory(baseline, JSON.parse(readFileSync(process.argv[3], 'utf8')));
  }
  console.log('Notice completeness checks passed; manual compliance review is still required.');
}
