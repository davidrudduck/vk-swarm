import { describe, it, expect } from 'vitest'
import { readFileSync } from 'fs'
import { join } from 'path'

describe('swConfigDriftGuard', () => {
  describe('navigateFallbackDenylist', () => {
    it('should have navigateFallbackDenylist configured in vite.config.ts', () => {
      const configSource = readFileSync(join(__dirname, '../../vite.config.ts'), 'utf8')
      expect(configSource).toContain('navigateFallbackDenylist')
    })

    it('should match /v1/* paths in the denylist regex', () => {
      const configSource = readFileSync(join(__dirname, '../../vite.config.ts'), 'utf8')
      const match = configSource.match(/navigateFallbackDenylist:\s*\[(\/.+?\/)\]/)
      expect(match).not.toBeNull()
      if (match) {
        const regexStr = match[1].slice(1, -1) // Remove slashes
        const regex = new RegExp(regexStr)
        expect(regex.test('/v1/oauth/github/start')).toBe(true)
        expect(regex.test('/v1/oauth/github/callback')).toBe(true)
        expect(regex.test('/v1/oauth/google/start')).toBe(true)
        expect(regex.test('/v1/projects')).toBe(true)
      }
    })

    it('should NOT match non-/v1/* paths in the denylist regex', () => {
      const configSource = readFileSync(join(__dirname, '../../vite.config.ts'), 'utf8')
      const match = configSource.match(/navigateFallbackDenylist:\s*\[(\/.+?\/)\]/)
      expect(match).not.toBeNull()
      if (match) {
        const regexStr = match[1].slice(1, -1)
        const regex = new RegExp(regexStr)
        expect(regex.test('/')).toBe(false)
        expect(regex.test('/login')).toBe(false)
        expect(regex.test('/oauth/callback')).toBe(false)
      }
    })
  })

  describe('api-cache predicate sync', () => {
    it('should contain the same exclusion clauses in vite.config.ts api-cache rule', () => {
      const configSource = readFileSync(join(__dirname, '../../vite.config.ts'), 'utf8')
      expect(configSource).toContain("startsWith('/v1/')")
      expect(configSource).toContain("!url.pathname.startsWith('/v1/shape')")
      expect(configSource).toContain("!url.pathname.startsWith('/v1/oauth')")
    })

    it('should mirror the exclusion clauses in swCachePredicate.ts', () => {
      const mirrorSource = readFileSync(join(__dirname, './swCachePredicate.ts'), 'utf8')
      expect(mirrorSource).toContain("startsWith('/v1/')")
      expect(mirrorSource).toContain("!pathname.startsWith('/v1/shape')")
      expect(mirrorSource).toContain("!pathname.startsWith('/v1/oauth')")
    })

    it('should have matching startsWith clause counts between config and mirror', () => {
      const configSource = readFileSync(join(__dirname, '../../vite.config.ts'), 'utf8')
      const mirrorSource = readFileSync(join(__dirname, './swCachePredicate.ts'), 'utf8')

      // Extract the urlPattern arrow from config (from "urlPattern: ({ url }) =>" to the handler)
      const configMatch = configSource.match(/urlPattern:\s*\(\{\s*url\s*\}\)\s*=>\s*([\s\S]*?),\s*handler:/m)
      expect(configMatch).not.toBeNull()

      // Extract the isApiCacheable function body from mirror
      const mirrorMatch = mirrorSource.match(/return\s*\(([\s\S]*?)\);/m)
      expect(mirrorMatch).not.toBeNull()

      if (configMatch && mirrorMatch) {
        const configCount = (configMatch[1].match(/startsWith\(/g) || []).length
        const mirrorCount = (mirrorMatch[1].match(/startsWith\(/g) || []).length
        expect(configCount).toBe(mirrorCount)
      }
    })
  })
})
