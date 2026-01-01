import { test, expect } from '@playwright/test';

test('Check menu has API key option', async ({ page }) => {
  await page.goto('http://localhost:8080');
  await page.waitForTimeout(3000);

  // Find and click menu button
  const menuBtn = page.locator('button.menu-btn');
  const menuVisible = await menuBtn.isVisible();
  console.log('Menu button visible:', menuVisible);

  if (menuVisible) {
    await menuBtn.click();
    await page.waitForTimeout(500);

    // Check for API key option
    const menuText = await page.locator('.menu-dropdown').textContent();
    console.log('Menu content:', menuText);

    const hasApiKey = menuText?.includes('APIキー設定');
    console.log('Has API key option:', hasApiKey);
    expect(hasApiKey).toBe(true);
  } else {
    console.log('Menu button NOT visible!');
    // Take screenshot
    await page.screenshot({ path: 'menu-debug.png' });
  }
});
