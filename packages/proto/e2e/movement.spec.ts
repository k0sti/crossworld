import { test, expect, type Page } from '@playwright/test';

/** Click the canvas to gain focus, then send keyboard events */
async function enterGame(page: Page): Promise<void> {
  await page.goto('/');
  await page.click('#btn-guest');

  // Wait for scene to fully initialize
  const canvas = page.locator('canvas');
  await expect(canvas).toBeVisible({ timeout: 30_000 });
  await expect(page.locator('#hud')).toBeVisible({ timeout: 15_000 });

  // Let physics settle (character falls from y=20 to ground)
  await page.waitForTimeout(3000);

  // Click canvas to give it focus (pointer lock may fail in headless but keyboard still works)
  await canvas.click({ force: true });
  await page.waitForTimeout(200);
}

/** Parse position from HUD text like "Pos: 1.2, 3.4, 5.6" */
function parsePos(text: string | null): { x: number; y: number; z: number } | null {
  if (!text) return null;
  const match = text.match(/Pos:\s*([-\d.]+),\s*([-\d.]+),\s*([-\d.]+)/);
  if (!match) return null;
  return { x: parseFloat(match[1]), y: parseFloat(match[2]), z: parseFloat(match[3]) };
}

test.describe('WASD Movement', () => {
  test('W key moves player forward', async ({ page }) => {
    await enterGame(page);

    const posBefore = parsePos(await page.locator('#hud-pos').textContent());
    expect(posBefore).not.toBeNull();

    await page.screenshot({ path: 'test-output/03-before-move-w.png' });

    // Hold W for 1 second
    await page.keyboard.down('w');
    await page.waitForTimeout(1000);
    await page.keyboard.up('w');
    await page.waitForTimeout(200);

    const posAfter = parsePos(await page.locator('#hud-pos').textContent());
    expect(posAfter).not.toBeNull();

    await page.screenshot({ path: 'test-output/04-after-move-w.png' });

    // Position should have changed on XZ plane
    const dx = posAfter!.x - posBefore!.x;
    const dz = posAfter!.z - posBefore!.z;
    const distXZ = Math.sqrt(dx * dx + dz * dz);

    expect(distXZ).toBeGreaterThan(0.5);
  });

  test('S key moves player backward', async ({ page }) => {
    await enterGame(page);

    const posBefore = parsePos(await page.locator('#hud-pos').textContent());
    expect(posBefore).not.toBeNull();

    await page.keyboard.down('s');
    await page.waitForTimeout(1000);
    await page.keyboard.up('s');
    await page.waitForTimeout(200);

    const posAfter = parsePos(await page.locator('#hud-pos').textContent());
    expect(posAfter).not.toBeNull();

    const dx = posAfter!.x - posBefore!.x;
    const dz = posAfter!.z - posBefore!.z;
    const distXZ = Math.sqrt(dx * dx + dz * dz);

    expect(distXZ).toBeGreaterThan(0.5);
  });

  test('A key moves player left', async ({ page }) => {
    await enterGame(page);

    const posBefore = parsePos(await page.locator('#hud-pos').textContent());

    await page.keyboard.down('a');
    await page.waitForTimeout(1000);
    await page.keyboard.up('a');
    await page.waitForTimeout(200);

    const posAfter = parsePos(await page.locator('#hud-pos').textContent());

    const dx = posAfter!.x - posBefore!.x;
    const dz = posAfter!.z - posBefore!.z;
    const distXZ = Math.sqrt(dx * dx + dz * dz);

    expect(distXZ).toBeGreaterThan(0.5);
  });

  test('D key moves player right', async ({ page }) => {
    await enterGame(page);

    const posBefore = parsePos(await page.locator('#hud-pos').textContent());

    await page.keyboard.down('d');
    await page.waitForTimeout(1000);
    await page.keyboard.up('d');
    await page.waitForTimeout(200);

    const posAfter = parsePos(await page.locator('#hud-pos').textContent());

    const dx = posAfter!.x - posBefore!.x;
    const dz = posAfter!.z - posBefore!.z;
    const distXZ = Math.sqrt(dx * dx + dz * dz);

    expect(distXZ).toBeGreaterThan(0.5);
  });

  test('screenshots differ after movement', async ({ page }) => {
    await enterGame(page);

    const before = await page.screenshot({ path: 'test-output/05-screenshot-before.png' });

    await page.keyboard.down('w');
    await page.waitForTimeout(1500);
    await page.keyboard.up('w');
    await page.waitForTimeout(200);

    const after = await page.screenshot({ path: 'test-output/06-screenshot-after.png' });

    // Screenshots should differ (scene moved)
    expect(Buffer.compare(before, after)).not.toBe(0);
  });
});
