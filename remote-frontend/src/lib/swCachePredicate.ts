// MIRROR of the hive service worker's runtime api-cache rule in
// remote-frontend/vite.config.ts. It cannot be imported by the config: Workbox
// generateSW serializes the urlPattern arrow into sw.js via toString(), so the
// config's predicate must stay self-contained inline. This module exists to pin
// the exclusions under vitest; a drift-guard grep (task 102) ties the two
// copies together.
// Exclusions:
// - /v1/shape: Electric proxy long-poll/streaming traffic; caching would serve
//   stale/partial real-time data (adversarial review F3).
// - /v1/oauth: both OAuth legs (/v1/oauth/{provider}/start and
//   /v1/oauth/{provider}/callback) are GET navigations on the hive origin; a SW
//   intercepting or cache-falling-back on them breaks sign-in on the hive AND on
//   every node whose popup traverses this origin (F-2026-08-03-02).
export function isApiCacheable(pathname: string): boolean {
  return (
    pathname.startsWith('/v1/') &&
    !pathname.startsWith('/v1/shape') &&
    !pathname.startsWith('/v1/oauth')
  );
}
