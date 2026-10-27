import { expect } from '@playwright/test'

/** Asserts `mediaUrl` answers a byte-range request with a partial response. */
export async function expectRangeSupported(request, mediaUrl) {
  const response = await request.get(mediaUrl, { headers: { Range: 'bytes=0-1023' } })
  expect(response.status()).toBe(206)
  expect(response.headers()['content-range']).toMatch(/^bytes 0-1023\/\d+$/)
  expect((await response.body()).length).toBe(1024)
}

/**
 * Asserts the `<img>` at `locator` has actually loaded, not merely rendered:
 * `Thumbnail` swaps a broken image for a placeholder `<span>`, which a bare
 * visibility check on the slot would accept.
 */
export async function expectImageLoaded(locator) {
  await expect(locator).toBeVisible()
  await expect
    .poll(() => locator.evaluate((img) => img.tagName === 'IMG' && img.complete && img.naturalWidth > 0), {
      message: 'image loaded',
    })
    .toBe(true)
}
