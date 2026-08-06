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
});
