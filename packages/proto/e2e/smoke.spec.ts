import { test, expect } from '@playwright/test';

test.describe('Smoke tests', () => {
  test('app loads and shows login screen', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await page.goto('/');
    await expect(page.locator('#login-screen')).toBeVisible();
    await expect(page.locator('#btn-guest')).toBeVisible();

    await page.screenshot({ path: 'test-output/01-login-screen.png' });
    expect(errors).toEqual([]);
  });

  test('guest login initializes 3D scene', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await page.goto('/');
    await page.click('#btn-guest');

    // Wait for canvas to appear (Three.js renderer creates it)
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible({ timeout: 30_000 });

    // Wait for HUD to show (game loop started)
    await expect(page.locator('#hud')).toBeVisible({ timeout: 15_000 });

    // Let a few frames render
    await page.waitForTimeout(2000);

    // Verify HUD shows position data
    const hudPos = await page.locator('#hud-pos').textContent();
    expect(hudPos).toContain('Pos:');

    // Verify FPS is updating
    const hudFps = await page.locator('#hud-fps').textContent();
    expect(hudFps).toContain('FPS:');

    await page.screenshot({ path: 'test-output/02-scene-loaded.png' });

    // No uncaught errors
    expect(errors).toEqual([]);
  });

  test('canvas has non-zero dimensions', async ({ page }) => {
    await page.goto('/');
    await page.click('#btn-guest');
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible({ timeout: 30_000 });

    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThan(100);
    expect(box!.height).toBeGreaterThan(100);
  });
});
