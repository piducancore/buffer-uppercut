export const repository = 'piducancore/buffer-uppercut';
export const releasesUrl = `https://github.com/${repository}/releases`;

export const platforms = [
  { slug: 'windows-x64', name: 'Windows', detail: '64-bit · CLAP + VST3' },
  { slug: 'macos', name: 'macOS', detail: 'Architecture listed in release notes · CLAP + VST3' },
  { slug: 'linux-x64', name: 'Linux', detail: '64-bit · CLAP + VST3' },
];

// Only explicitly distributable packages are eligible. Internal candidates,
// standalone hosts and unknown signing status must never become download buttons.
export function packagesFor(release) {
  return platforms.flatMap((platform) => {
    const asset = release.assets.find((asset) =>
      asset.name.startsWith('Buffer-Uppercut-') &&
      asset.name.endsWith(`-${platform.slug}-unsigned.zip`) &&
      !asset.name.includes('not-for-distribution')
    );
    return asset ? [{ ...platform, asset, signing: 'Unsigned' }] : [];
  });
}

export function publicReleases(data) {
  if (!Array.isArray(data)) throw new Error('Invalid GitHub releases response');
  return data.filter((release) => !release.draft && release.published_at)
    .sort((a, b) => Date.parse(b.published_at) - Date.parse(a.published_at));
}

export async function loadReleases() {
  // Deterministic offline development; production explicitly enables API sync.
  if (process.env.RELEASES_SOURCE !== 'github') return [];
  const headers = { Accept: 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28' };
  if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  const releases = [];
  for (let page = 1; page <= 10; page++) {
    const response = await fetch(`https://api.github.com/repos/${repository}/releases?per_page=100&page=${page}`, {
      headers, signal: AbortSignal.timeout(15000),
    });
    if (!response.ok) throw new Error(`GitHub release sync failed: HTTP ${response.status}`);
    const data = await response.json();
    releases.push(...publicReleases(data));
    if (data.length < 100) return publicReleases(releases);
  }
  throw new Error('Release pagination exceeded limit; refusing a partial archive');
}
