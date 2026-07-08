---
id: "101"
phase: 1
title: Implement browser-safe SHA-256 fallback for PKCE
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/pkce.ts
  - remote-frontend/src/pkce.test.ts
  - remote-frontend/src/api.ts
  - remote-frontend/src/setupTests.ts
irreversible: false
scope_test: "remote-frontend/src/pkce.test.ts"
allowed_change: mixed
covers_criteria: [SC1, SC8]
---

## Failing test (write first)

Create `remote-frontend/src/pkce.test.ts`:

```ts
import { afterEach, describe, expect, it, vi } from 'vitest'
import { generateChallenge, generateVerifier } from './pkce'

const subtleDigest = vi.fn()

function stubCryptoWithoutSubtle(): void {
  vi.stubGlobal('crypto', {
    getRandomValues: vi.fn((array: Uint8Array) => {
      for (let i = 0; i < array.length; i += 1) {
        array[i] = i
      }
      return array
    }),
  })
}

describe('PKCE challenge generation', () => {
  afterEach(() => {
    subtleDigest.mockReset()
    vi.unstubAllGlobals()
  })

  it('uses native crypto.subtle.digest when available', async () => {
    subtleDigest.mockResolvedValue(new Uint8Array([0, 15, 16, 255]).buffer)
    vi.stubGlobal('crypto', {
      subtle: { digest: subtleDigest },
      getRandomValues: vi.fn(),
    })

    await expect(generateChallenge('abc')).resolves.toBe('000f10ff')

    expect(subtleDigest).toHaveBeenCalledTimes(1)
    expect(subtleDigest.mock.calls[0][0]).toBe('SHA-256')
    expect(Array.from(subtleDigest.mock.calls[0][1] as Uint8Array)).toEqual([97, 98, 99])
  })

  it('falls back to in-repo SHA-256 when crypto.subtle is unavailable', async () => {
    stubCryptoWithoutSubtle()

    await expect(generateChallenge('')).resolves.toBe(
      'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
    )
    await expect(generateChallenge('abc')).resolves.toBe(
      'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'
    )
  })

  it('keeps verifier generation working with getRandomValues-only browser crypto', async () => {
    stubCryptoWithoutSubtle()

    const verifier = generateVerifier()
    expect(verifier).toMatch(/^[A-Za-z0-9_-]+$/)
    await expect(generateChallenge(verifier)).resolves.toMatch(/^[0-9a-f]{64}$/)
  })
})
```

This test must fail on the current code with `Cannot read properties of undefined (reading 'digest')` in the fallback cases.

## Change

### File: `remote-frontend/src/pkce.ts`

- **Anchor:** function `generateChallenge`, lines 7-12.
- **Before:**
  ```ts
  export async function generateChallenge(verifier: string): Promise<string> {
    const encoder = new TextEncoder()
    const data = encoder.encode(verifier)
    const hash = await crypto.subtle.digest('SHA-256', data)
    return bytesToHex(new Uint8Array(hash))
  }
  ```
- **After:**
  ```ts
  export async function generateChallenge(verifier: string): Promise<string> {
    const encoder = new TextEncoder()
    const data = encoder.encode(verifier)
    const hash = await sha256(data)
    return bytesToHex(hash)
  }

  async function sha256(data: Uint8Array): Promise<Uint8Array> {
    const subtle = globalThis.crypto?.subtle
    if (typeof subtle?.digest === 'function') {
      return new Uint8Array(await subtle.digest('SHA-256', data))
    }
    return sha256Fallback(data)
  }

  const SHA256_INITIAL = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
  ]

  const SHA256_K = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
  ]

  function sha256Fallback(data: Uint8Array): Uint8Array {
    const bitLength = data.length * 8
    const paddedLength = Math.ceil((data.length + 9) / 64) * 64
    const padded = new Uint8Array(paddedLength)
    padded.set(data)
    padded[data.length] = 0x80

    const view = new DataView(padded.buffer)
    view.setUint32(paddedLength - 8, Math.floor(bitLength / 0x100000000))
    view.setUint32(paddedLength - 4, bitLength >>> 0)

    const hash = SHA256_INITIAL.slice()
    const words = new Uint32Array(64)

    for (let offset = 0; offset < paddedLength; offset += 64) {
      for (let i = 0; i < 16; i += 1) {
        words[i] = view.getUint32(offset + i * 4)
      }
      for (let i = 16; i < 64; i += 1) {
        const s0 = rotateRight(words[i - 15], 7) ^ rotateRight(words[i - 15], 18) ^ (words[i - 15] >>> 3)
        const s1 = rotateRight(words[i - 2], 17) ^ rotateRight(words[i - 2], 19) ^ (words[i - 2] >>> 10)
        words[i] = (words[i - 16] + s0 + words[i - 7] + s1) >>> 0
      }

      let [a, b, c, d, e, f, g, h] = hash
      for (let i = 0; i < 64; i += 1) {
        const ch = (e & f) ^ (~e & g)
        const maj = (a & b) ^ (a & c) ^ (b & c)
        const s0 = rotateRight(a, 2) ^ rotateRight(a, 13) ^ rotateRight(a, 22)
        const s1 = rotateRight(e, 6) ^ rotateRight(e, 11) ^ rotateRight(e, 25)
        const t1 = (h + s1 + ch + SHA256_K[i] + words[i]) >>> 0
        const t2 = (s0 + maj) >>> 0
        h = g
        g = f
        f = e
        e = (d + t1) >>> 0
        d = c
        c = b
        b = a
        a = (t1 + t2) >>> 0
      }

      hash[0] = (hash[0] + a) >>> 0
      hash[1] = (hash[1] + b) >>> 0
      hash[2] = (hash[2] + c) >>> 0
      hash[3] = (hash[3] + d) >>> 0
      hash[4] = (hash[4] + e) >>> 0
      hash[5] = (hash[5] + f) >>> 0
      hash[6] = (hash[6] + g) >>> 0
      hash[7] = (hash[7] + h) >>> 0
    }

    const out = new Uint8Array(32)
    const outView = new DataView(out.buffer)
    for (let i = 0; i < hash.length; i += 1) {
      outView.setUint32(i * 4, hash[i])
    }
    return out
  }

  function rotateRight(value: number, bits: number): number {
    return (value >>> bits) | (value << (32 - bits))
  }
  ```

### File: `remote-frontend/src/pkce.test.ts` (CREATE)

Create exactly the test file shown in `## Failing test (write first)`.

### File: `remote-frontend/src/setupTests.ts` (READ-ONLY sibling acknowledgement)

Read this sibling before adding `remote-frontend/src/pkce.test.ts`. It currently only imports `@testing-library/jest-dom`; no test setup helper needs to move into it. If that changes during execution, record the divergence in `docs/plans/fix-nonloopback-signin/decisions-ledger.md`.

### File: `remote-frontend/src/api.ts` (READ-ONLY sibling acknowledgement)

Read this sibling before adding `remote-frontend/src/pkce.test.ts` because `wai-plan-lint.sh` flags same-directory creations. It is an API adapter, not a test harness sibling; do not edit it. If execution discovers an actual shared pattern requirement, record the reason in `docs/plans/fix-nonloopback-signin/decisions-ledger.md`.

## Allowed moves

- Edit only `generateChallenge()` and add the private SHA-256 helpers/constants in `remote-frontend/src/pkce.ts`.
- Keep public exports and storage-key constants unchanged.
- Create `remote-frontend/src/pkce.test.ts` exactly as the failing test above, then make it pass.
- Do not import `node:crypto`, `Buffer`, or any new dependency.
- Do not edit OAuth route files in this task.

## STOP triggers

- `generateVerifier`, `storeVerifier`, `retrieveVerifier`, `clearVerifier`, `storeInvitationToken`, `retrieveInvitationToken`, or `clearInvitationToken` would need a signature or storage-key change.
- A SHA-256 known vector differs from the expected digest.
- The fallback needs Node-only APIs or a package dependency.
- The native branch is not covered by a test.
- Any unlisted file needs an edit.

## Done when

`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npm run test:run -- src/pkce.test.ts" bash ~/.claude/wai/scripts/task-gate.sh fix-nonloopback-signin 101` exits 0.
