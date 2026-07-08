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
    // FIPS 180-2 multi-block vector (56 bytes, exercises padding and multi-block compression)
    await expect(generateChallenge('abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq')).resolves.toBe(
      '248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1'
    )
  })

  it('keeps verifier generation working with getRandomValues-only browser crypto', async () => {
    stubCryptoWithoutSubtle()

    const verifier = generateVerifier()
    expect(verifier).toMatch(/^[A-Za-z0-9_-]+$/)
    await expect(generateChallenge(verifier)).resolves.toMatch(/^[0-9a-f]{64}$/)
  })
})
