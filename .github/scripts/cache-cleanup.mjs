const token = process.env.GITHUB_TOKEN;
const repository = process.env.GITHUB_REPOSITORY;
const prefixes = (process.env.CACHE_PREFIXES ?? "")
  .split(/[\n,]/)
  .map((value) => value.trim())
  .filter(Boolean);
const keepLatest = Number.parseInt(
  process.env.MAX_CACHES_PER_PREFIX ?? process.env.KEEP_LATEST_PER_PREFIX ?? "4",
  10,
);
const keepRecentDays = Number.parseInt(
  process.env.MAX_CACHE_AGE_DAYS ?? process.env.KEEP_RECENT_DAYS ?? "14",
  10,
);

if (!token) {
  throw new Error("GITHUB_TOKEN is required.");
}

if (!repository) {
  throw new Error("GITHUB_REPOSITORY is required.");
}

if (prefixes.length === 0) {
  console.log("No cache prefixes configured, skipping cleanup.");
  process.exit(0);
}

const [owner, repo] = repository.split("/");
if (!owner || !repo) {
  throw new Error(`Invalid GITHUB_REPOSITORY: ${repository}`);
}

const headers = {
  Accept: "application/vnd.github+json",
  Authorization: `Bearer ${token}`,
  "X-GitHub-Api-Version": "2022-11-28",
};

async function github(path, init = {}) {
  const response = await fetch(`https://api.github.com${path}`, {
    ...init,
    headers: {
      ...headers,
      ...(init.headers ?? {}),
    },
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`GitHub API ${response.status} ${response.statusText}: ${body}`);
  }

  if (response.status === 204) {
    return null;
  }

  return response.json();
}

async function listAllCaches() {
  const caches = [];
  let page = 1;

  while (true) {
    const data = await github(
      `/repos/${owner}/${repo}/actions/caches?per_page=100&page=${page}`,
    );
    const entries = data.actions_caches ?? [];
    caches.push(...entries);

    if (entries.length < 100) {
      break;
    }

    page += 1;
  }

  return caches;
}

function shouldDelete(cache, index, now) {
  if (index < keepLatest) {
    return false;
  }

  const lastAccessedAt = Date.parse(cache.last_accessed_at);
  if (Number.isNaN(lastAccessedAt)) {
    return true;
  }

  const ageDays = (now - lastAccessedAt) / 86_400_000;
  return ageDays > keepRecentDays;
}

async function main() {
  const now = Date.now();
  const caches = await listAllCaches();
  const managedCaches = caches.filter((cache) =>
    prefixes.some((prefix) => cache.key.startsWith(prefix)),
  );

  console.log(
    `Found ${managedCaches.length} managed caches across prefixes: ${prefixes.join(", ")}`,
  );

  for (const prefix of prefixes) {
    const scopedCaches = managedCaches
      .filter((cache) => cache.key.startsWith(prefix))
      .sort(
        (left, right) =>
          Date.parse(right.last_accessed_at) - Date.parse(left.last_accessed_at),
      );

    console.log(`Prefix ${prefix}: ${scopedCaches.length} cache(s)`);

    for (const [index, cache] of scopedCaches.entries()) {
      if (!shouldDelete(cache, index, now)) {
        continue;
      }

      console.log(
        `Deleting cache ${cache.id} key=${cache.key} last_accessed_at=${cache.last_accessed_at}`,
      );
      await github(`/repos/${owner}/${repo}/actions/caches/${cache.id}`, {
        method: "DELETE",
      });
    }
  }
}

await main();
