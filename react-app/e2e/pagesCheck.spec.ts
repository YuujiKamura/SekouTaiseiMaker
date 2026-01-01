import { test, expect } from '@playwright/test';

test('Check Pages deployment has API key option', async ({ page }) => {
  await page.goto('https://yuujikamura.github.io/SekouTaiseiMaker/');
  await page.waitForTimeout(5000);  // Wait for WASM to load

  // Check page title
  const title = await page.title();
  console.log('Page title:', title);

  // Find and click menu button
  const menuBtn = page.locator('button.menu-btn');
  const menuVisible = await menuBtn.isVisible();
  console.log('Menu button visible:', menuVisible);

  if (menuVisible) {
    await menuBtn.click();
    await page.waitForTimeout(500);

    // Check for API key option
    const menuDropdown = page.locator('.menu-dropdown');
    const dropdownVisible = await menuDropdown.isVisible();
    console.log('Menu dropdown visible:', dropdownVisible);

    if (dropdownVisible) {
      const menuText = await menuDropdown.textContent();
      console.log('Menu content:', menuText?.substring(0, 200));

      const hasApiKey = menuText?.includes('APIキー設定');
      console.log('Has API key option:', hasApiKey);
    }
  } else {
    console.log('Menu button NOT visible on Pages!');
    const html = await page.content();
    console.log('Page HTML (first 500 chars):', html.substring(0, 500));
  }
});
