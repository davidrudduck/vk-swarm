import { describe, expect, it } from 'vitest';
import { isApiCacheable } from './swCachePredicate';

describe('isApiCacheable (hive SW api-cache rule, mirror of vite.config.ts)', () => {
  it('excludes the OAuth start leg', () => {
    expect(isApiCacheable('/v1/oauth/github/start')).toBe(false);
  });
  it('excludes the OAuth callback leg', () => {
    expect(isApiCacheable('/v1/oauth/github/callback')).toBe(false);
  });
  it('excludes Electric shape traffic (adversarial review F3 precedent)', () => {
    expect(isApiCacheable('/v1/shape/tasks')).toBe(false);
  });
  it('still caches ordinary /v1/ API responses', () => {
    expect(isApiCacheable('/v1/projects')).toBe(true);
  });
  it('ignores non-/v1 paths', () => {
    expect(isApiCacheable('/assets/app.js')).toBe(false);
  });

  describe('boundary paths', () => {
    it('excludes the bare /v1/oauth path itself', () => {
      expect(isApiCacheable('/v1/oauth')).toBe(false);
    });
    it('excludes the bare /v1/shape path itself', () => {
      expect(isApiCacheable('/v1/shape')).toBe(false);
    });
    it('treats the exclusions as prefixes: /v1/oauthx is also excluded', () => {
      // startsWith semantics — any sibling route beginning with the literal
      // "/v1/oauth" is swept into the exclusion. Documented, intentional.
      expect(isApiCacheable('/v1/oauthx')).toBe(false);
    });
    it('treats /v1/shapes as excluded by the same prefix semantics', () => {
      expect(isApiCacheable('/v1/shapes')).toBe(false);
    });
    it('caches the bare /v1/ root path', () => {
      expect(isApiCacheable('/v1/')).toBe(true);
    });
    it('does not cache /v1 without the trailing slash', () => {
      expect(isApiCacheable('/v1')).toBe(false);
    });
    it('does not cache the empty path', () => {
      expect(isApiCacheable('')).toBe(false);
    });
    it('is case-sensitive on the /v1/ prefix', () => {
      expect(isApiCacheable('/V1/projects')).toBe(false);
    });
    it('is case-sensitive on the oauth exclusion (uppercase OAUTH stays cacheable)', () => {
      // URL pathnames are case-sensitive; the server routes lowercase only.
      expect(isApiCacheable('/v1/OAUTH/github/start')).toBe(true);
    });
    it('does not exclude oauth appearing deeper than the prefix position', () => {
      expect(isApiCacheable('/v1/projects/oauth')).toBe(true);
    });
  });
});
