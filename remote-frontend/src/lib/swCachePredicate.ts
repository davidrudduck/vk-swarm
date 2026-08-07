// MIRROR of the hive service worker's runtime api-cache rule in
// remote-frontend/vite.config.ts. It cannot be imported by the config: Workbox
// generateSW serializes the urlPattern arrow into sw.js via toString(), so the
// config's predicate must stay self-contained inline. This module exists to pin
// the exclusions under vitest; swConfigDriftGuard.test.ts keeps the two
// copies in sync.
// Exclusions:
// - /v1/shape: Electric proxy long-poll/streaming traffic; caching would serve
//   stale/partial real-time data (adversarial review F3).
// - /v1/oauth: both OAuth legs (/v1/oauth/{provider}/start and
//   /v1/oauth/{provider}/callback) are GET navigations; the PRIMARY sign-in fix is
//   navigateFallbackDenylist in vite.config.ts (navigations). This cache exclusion is
//   defense-in-depth for non-navigation /v1/oauth fetches (F-2026-08-03-02).
export function isApiCacheable(pathname: string): boolean {
  return (
    pathname.startsWith('/v1/') &&
    !pathname.startsWith('/v1/shape') &&
    !pathname.startsWith('/v1/oauth')
  );
}
