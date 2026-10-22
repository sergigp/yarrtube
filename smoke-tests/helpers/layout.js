import { expect } from '@playwright/test'

/** Asserts the page is no wider than the viewport, so nothing scrolls sideways. */
export async function expectNoHorizontalOverflow(page) {
  const { scrollWidth, innerWidth } = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    innerWidth: window.innerWidth,
  }))
  expect(scrollWidth, `page width on ${page.url()}`).toBeLessThanOrEqual(innerWidth)
}

/** Asserts `locator`'s box lies inside the viewport. */
export async function expectWithinViewport(page, locator) {
  const { width, height } = page.viewportSize()
  const box = await locator.boundingBox()
  expect(box, 'element is rendered').not.toBeNull()
  expect(box.x).toBeGreaterThanOrEqual(0)
  expect(box.y).toBeGreaterThanOrEqual(0)
  expect(Math.round(box.x + box.width)).toBeLessThanOrEqual(width)
  expect(Math.round(box.y + box.height)).toBeLessThanOrEqual(height)
}
